import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useStatusStore } from '@/stores/status'
import type { Command, Project, RunningStatus, Workflow } from '@/types'

function genId() { return crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2, 9)}` }

export interface RunningTab { projectId: string; projectName: string; commandId: string; commandName: string; command: string }
export interface DocTab { id: string; title: string; command: string; content: string; loading: boolean }
export interface ToolTab { id: string; title: string; toolType: string }
export interface LogStats { error: number; warn: number; info: number; debug: number }

export const useProjectStore = defineStore('project', () => {
  const decodersMap: Record<string, TextDecoder> = {}
  const textBufferMap: Record<string, string> = {}

  const projects = ref<Project[]>([])
  const selectedProjectId = ref<string | null>(null)
  const runningMap = ref<Record<string, RunningStatus>>({})
  const outputMap = ref<Record<string, number[]>>({})
  const runningTabs = ref<RunningTab[]>([])
  const docTabs = ref<DocTab[]>([])
  const toolTabs = ref<ToolTab[]>([])
  const memoryTabs = ref<{ id: string; title: string; content: string; type: string; scope: string; topicKey: string; editing: boolean }[]>([])
  const activeTabIndex = ref(0)
  const activeDocIndex = ref(-1)
  const activeToolIndex = ref(-1)
  const activeMemoryIndex = ref(-1)
  const activeTabType = ref<'term'|'doc'|'tool'|'note'|'memory'>('term')

  const workflows = ref<Workflow[]>([])
  const pendingInput = ref('')
  const recentTools = ref<string[]>(JSON.parse(localStorage.getItem('jc9-recent-tools') || '[]'))
  const sidebarTab = ref<'projects'|'workflows'|'tools'|'notes'|'memories'>('projects')

  // ── 模块标签状态持久化（切换模块后恢复上次激活的标签）──
  const moduleTabState = ref<Record<string, { type: string; index: number; docIndex: number; toolIndex: number; memoryIndex: number }>>({})

  function saveCurrentModuleState(curTab: string) {
    moduleTabState.value[curTab] = {
      type: activeTabType.value,
      index: activeTabIndex.value,
      docIndex: activeDocIndex.value,
      toolIndex: activeToolIndex.value,
      memoryIndex: activeMemoryIndex.value,
    }
  }

  function restoreModuleState(tab: string) {
    const state = moduleTabState.value[tab]
    if (state) {
      // 校验保存的状态与当前模块匹配，避免之前 bug 导致的脏数据
      const typeOk = (
        (tab === 'projects' || tab === 'workflows') ? (state.type === 'term' || state.type === 'doc') :
        tab === 'tools' ? state.type === 'tool' :
        tab === 'notes' ? state.type === 'note' :
        tab === 'memories' ? state.type === 'memory' :
        false
      )
      if (typeOk) {
        activeTabType.value = state.type as any
        activeTabIndex.value = Math.min(state.index, Math.max(0, runningTabs.value.length - 1))
        activeDocIndex.value = Math.min(state.docIndex, Math.max(-1, docTabs.value.length - 1))
        activeToolIndex.value = Math.min(state.toolIndex, Math.max(-1, toolTabs.value.length - 1))
        activeMemoryIndex.value = Math.min(state.memoryIndex, Math.max(-1, memoryTabs.value.length - 1))
        return
      }
      // 脏数据 → 清除并走 fallback
      delete moduleTabState.value[tab]
    }
    // 首次进入或脏数据已清除 → 激活第一个可用标签
    if (tab === 'tools' && toolTabs.value.length > 0) {
      activeTabType.value = 'tool'; activeToolIndex.value = 0
    } else if (tab === 'notes') {
      // notes 交给 MainPanel 处理
    } else if (tab === 'memories' && memoryTabs.value.length > 0) {
      activeTabType.value = 'memory'; activeMemoryIndex.value = 0
    } else if ((tab === 'projects' || tab === 'workflows') && runningTabs.value.length > 0) {
      activeTabType.value = 'term'; activeTabIndex.value = 0
    } else {
      activeTabType.value = 'term'
      activeTabIndex.value = -1
    }
  }

  // 监听 sidebarTab 切换
  watch(sidebarTab, (newTab, oldTab) => {
    if (oldTab) saveCurrentModuleState(oldTab)
    restoreModuleState(newTab)
  })
  const mainMode = ref<'main' | 'ai'>('main')
  const workflowRunning = ref(false)
  const workflowProgress = ref<{ step: number; total: number; name: string; status: string; stdout: string; stderr: string } | null>(null)

  // ── 工作流加载/保存 ──
  async function loadWorkflows() {
    try {
      const json = await invoke<string>('get_workflows')
      workflows.value = JSON.parse(json)
    } catch (e) {
      console.error(e)
      useStatusStore().pushMessage(`加载工作流失败: ${e}`, 'error')
    }
  }

  async function persistWorkflows() {
    try {
      await invoke('save_workflows', { workflowsJson: JSON.stringify(workflows.value) })
    } catch (e) {
      console.error(e)
    }
  }

  function addWorkflow(w: Omit<Workflow, 'id'>) {
    const item: Workflow = { ...w, id: genId() }
    workflows.value.push(item)
    persistWorkflows()
  }

  function removeWorkflow(id: string) {
    workflows.value = workflows.value.filter(w => w.id !== id)
    persistWorkflows()
  }

  function updateWorkflow(id: string, data: Partial<Workflow>) {
    const w = workflows.value.find(x => x.id === id)
    if (w) { Object.assign(w, data); persistWorkflows() }
  }

  function toggleWfFav(id: string) {
    const w = workflows.value.find(x => x.id === id)
    if (w) { w.favorite = !w.favorite; persistWorkflows() }
  }

  const frequentWorkflows = computed(() =>
    [...workflows.value].filter(w => (w.useCount || 0) > 0).sort((a, b) => (b.useCount || 0) - (a.useCount || 0))
  )
  const favWorkflows = computed(() => workflows.value.filter(w => w.favorite))

  // ── 工作流执行 ──
  async function runWorkflow(id: string) {
    const w = workflows.value.find(x => x.id === id)
    if (!w || w.steps.length === 0) return
    if (workflowRunning.value) {
      useStatusStore().pushMessage('已有工作流正在运行', 'warn')
      return
    }

    w.useCount = (w.useCount || 0) + 1
    persistWorkflows()
    workflowRunning.value = true
    workflowProgress.value = null

    // 创建工作流终端标签页
    const wfProcessId = `workflow-${id.slice(0, 8)}`
    const wfFullKey = cmdKey('workflow', wfProcessId)
    const existing = runningTabs.value.findIndex(t => t.commandId === wfProcessId)
    if (existing >= 0) {
      activeTabIndex.value = existing
    } else {
      runningTabs.value.push({
        projectId: 'workflow',
        projectName: '工作流',
        commandId: wfProcessId,
        commandName: w.name,
        command: `工作流: ${w.steps.length} 步`
      })
      activeTabIndex.value = runningTabs.value.length - 1
    }
    activeTabType.value = 'term'
    // 重置输出缓冲区并标记运行中
    outputMap.value[wfFullKey] = []
    runningMap.value[wfFullKey] = 'running'
    delete decodersMap[wfFullKey]
    textBufferMap[wfFullKey] = ''

    // 监听进度事件
    const unlisten = await listen<any>('workflow-event', (event) => {
      workflowProgress.value = event.payload
      const p = event.payload
      if (p.type === 'step_start') {
        useStatusStore().pushMessage(`▶ 步骤 ${p.step}/${p.total}: ${p.name}`, 'info')
        // 标记 PTY 进程 ID 以便 terminal 显示输出
        if (p.processId) {
          runningMap.value[p.processId] = 'running'
        }
      } else if (p.type === 'step_fail') {
        useStatusStore().pushMessage(`❌ 步骤 ${p.step}/${p.total}: ${p.name}`, 'error')
      } else if (p.type === 'workflow_done') {
        useStatusStore().pushMessage(`🏁 工作流「${w.name}」执行完成`, 'success')
        workflowRunning.value = false
        workflowProgress.value = null
        runningMap.value[wfFullKey] = 'stopped'
      }
    })

    try {
      await invoke('run_workflow', { tabId: wfFullKey, steps: w.steps })
    } catch (e) {
      runningMap.value[wfFullKey] = 'stopped'
      useStatusStore().pushMessage(`工作流执行失败: ${e}`, 'error')
      workflowRunning.value = false
    } finally {
      unlisten()
    }
  }

  let _defaultTerminalStarted = false
  function startDefaultTerminal() {
    if (_defaultTerminalStarted || runningTabs.value.length > 0) return; _defaultTerminalStarted = true
    if (projects.value.length === 0) addProject('默认终端')
    const p = projects.value[0]; if (p.commands.length === 0) addCommand(p.id, { name: '终端', command: '', workingDir: '' })
    startCommand(p.id, p.commands[0])
  }

  async function loadProjects() {
    // 重试 3 次，应对 Tauri state 尚未 manage 的时序问题
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        projects.value = await invoke<Project[]>('get_projects')
        return // 成功则退出
      } catch (e) {
        if (attempt < 2) {
          await new Promise(r => setTimeout(r, 200 * (attempt + 1)))
        } else {
          console.error(e)
          const { useStatusStore } = await import('@/stores/status')
          useStatusStore().pushMessage(`加载项目失败: ${e}`, 'error')
        }
      }
    }
  }
  async function saveProjects() { try { await invoke('save_all_projects', { projects: JSON.parse(JSON.stringify(projects.value)) }) } catch (e) { console.error(e) } }

  function addProject(name: string) {
    const p: Project = { id: genId(), name, commands: [], createdAt: new Date().toISOString() }
    projects.value.push(p); selectedProjectId.value = p.id; saveProjects()
    useStatusStore().pushMessage(`项目「${name}」已创建`, 'success')
  }
  function removeProject(id: string) {
    const p = projects.value.find(p => p.id === id)
    const name = p?.name ?? ''
    projects.value.find(p => p.id === id)?.commands.forEach(c => stopCommand(id, c.id))
    projects.value = projects.value.filter(p => p.id !== id)
    if (selectedProjectId.value === id) selectedProjectId.value = projects.value[0]?.id ?? null
    saveProjects()
    useStatusStore().pushMessage(`项目「${name}」已删除`)
  }
  function updateProjectName(id: string, name: string) { const p = projects.value.find(p => p.id === id); if (p) { p.name = name; saveProjects(); useStatusStore().pushMessage(`项目已重命名为「${name}」`) } }

  function addCommand(projectId: string, c: Omit<Command, 'id'>) { const p = projects.value.find(p => p.id === projectId); if (p) { p.commands.push({ ...c, id: genId() }); saveProjects() } }
  function removeCommand(projectId: string, commandId: string) { stopCommand(projectId, commandId); const p = projects.value.find(p => p.id === projectId); if (p) { p.commands = p.commands.filter(c => c.id !== commandId); saveProjects() } }
  function updateCommand(projectId: string, u: Command) { const p = projects.value.find(p => p.id === projectId); if (p) { const i = p.commands.findIndex(c => c.id === u.id); if (i !== -1) { p.commands[i] = u; saveProjects() } } }

  function cmdKey(projectId: string, commandId: string) { return `${projectId}::${commandId}` }

  async function startCommand(projectId: string, cmd: Command, keepOutput = false) {
    const processId = cmdKey(projectId, cmd.id)
    const project = projects.value.find(p => p.id === projectId)
    if (!keepOutput) {
      outputMap.value[processId] = []
      logStatsMap.value[processId] = { error: 0, warn: 0, info: 0, debug: 0 }
      delete decodersMap[processId]
      textBufferMap[processId] = ''
    } else {
      const divider = `\r\n\x1b[1;33m--- 进程重启 --- \x1b[0m\r\n`
      bufferPtyOutput(processId, Array.from(new TextEncoder().encode(divider)))
    }
    runningMap.value[processId] = 'running'
    const existing = runningTabs.value.findIndex(t => t.projectId === projectId && t.commandId === cmd.id)
    if (existing >= 0) { activeTabIndex.value = existing; activeTabType.value = 'term' }
    else { runningTabs.value.push({ projectId, projectName: project?.name ?? '', commandId: cmd.id, commandName: cmd.name, command: cmd.command }); activeTabIndex.value = runningTabs.value.length - 1; activeTabType.value = 'term' }
    try {
      await invoke('start_command', { processId, projectId, commandId: cmd.id, workingDir: cmd.workingDir, command: cmd.command })
      useStatusStore().pushMessage(`命令「${cmd.name}」已启动`, 'success')
    } catch (e) {
      runningMap.value[processId] = 'stopped'; bufferPtyOutput(processId, [...new TextEncoder().encode(`[启动失败] ${e}`)])
      useStatusStore().pushMessage(`命令「${cmd.name}」启动失败: ${e}`, 'error')
    }
  }

  async function stopCommand(projectId: string, commandId: string) {
    const processId = cmdKey(projectId, commandId)
    const p = projects.value.find(x => x.id === projectId)
    const c = p?.commands.find(x => x.id === commandId)
    try { await invoke('stop_command_by_ids', { projectId, commandId }) } catch { /* */ }
    runningMap.value[processId] = 'stopped'
    if (c) useStatusStore().pushMessage(`命令「${c.name}」已停止`)
  }
  async function restartCommand(projectId: string, cmd: Command) { await stopCommand(projectId, cmd.id); await new Promise(r => setTimeout(r, 500)); await startCommand(projectId, cmd, true) }

  function closeTab(index: number) { const tab = runningTabs.value[index]; if (tab) stopCommand(tab.projectId, tab.commandId); runningTabs.value.splice(index, 1); if (activeTabIndex.value >= runningTabs.value.length) activeTabIndex.value = Math.max(0, runningTabs.value.length - 1) }
  function closeDocTab(index: number) { docTabs.value.splice(index, 1); if (activeDocIndex.value >= docTabs.value.length) activeDocIndex.value = Math.max(-1, docTabs.value.length - 1) }

  function recordRecentTool(toolType: string) {
    let list = [...recentTools.value].filter(t => t !== toolType)
    list.unshift(toolType)
    if (list.length > 4) list = list.slice(0, 4)
    recentTools.value = list
    localStorage.setItem('jc9-recent-tools', JSON.stringify(list))
  }

  function openTool(toolType: string, title: string) {
    const existing = toolTabs.value.findIndex(t => t.toolType === toolType)
    if (existing >= 0) {
      activeToolIndex.value = existing
      activeTabType.value = 'tool'
    } else {
      toolTabs.value.push({ id: genId(), title, toolType })
      activeToolIndex.value = toolTabs.value.length - 1
      activeTabType.value = 'tool'
    }
    recordRecentTool(toolType)
  }

  function closeToolTab(index: number) {
    toolTabs.value.splice(index, 1)
    if (activeToolIndex.value >= toolTabs.value.length) {
      activeToolIndex.value = Math.max(0, toolTabs.value.length - 1)
    }
  }

  function openMemoryTab(memory: { id: string; title: string; content: string; type: string; scope: string; topicKey?: string }, editing = false) {
    const existing = memoryTabs.value.findIndex(t => t.id === memory.id)
    if (existing >= 0) {
      activeMemoryIndex.value = existing
      activeTabType.value = 'memory'
    } else {
      memoryTabs.value.push({ ...memory, topicKey: memory.topicKey || '', editing })
      activeMemoryIndex.value = memoryTabs.value.length - 1
      activeTabType.value = 'memory'
    }
  }

  function closeMemoryTab(index: number) {
    memoryTabs.value.splice(index, 1)
    if (activeMemoryIndex.value >= memoryTabs.value.length) {
      activeMemoryIndex.value = Math.max(-1, memoryTabs.value.length - 1)
    }
  }

  function toggleMemoryEdit(index: number) {
    if (memoryTabs.value[index]) {
      memoryTabs.value[index].editing = !memoryTabs.value[index].editing
    }
  }

  async function openDoc(command: string, title: string) {
    const id = genId()
    const tab: DocTab = { id, title, command, content: '', loading: true }
    docTabs.value.push(tab); activeDocIndex.value = docTabs.value.length - 1; activeTabType.value = 'doc'
    try {
      const text = await invoke<string>('fetch_doc', { command })
      const existing = docTabs.value.find(t => t.id === id)
      if (existing) { existing.content = text; existing.loading = false }
    } catch {
      const existing = docTabs.value.find(t => t.id === id)
      if (existing) { existing.content = '获取文档失败，请检查网络。'; existing.loading = false }
    }
  }

  function openDocFromText(pid: string, title: string, content: string) {
    docTabs.value.push({ id: genId(), title, command: `日志: ${pid}`, content, loading: false })
    activeDocIndex.value = docTabs.value.length - 1; activeTabType.value = 'doc'
  }

  function bufferPtyOutput(pid: string, data: number[]) {
    if (!outputMap.value[pid]) outputMap.value[pid] = []
    for (const b of data) outputMap.value[pid]!.push(b)
    if (outputMap.value[pid]!.length > 200000) outputMap.value[pid] = outputMap.value[pid]!.slice(-100000)

    // 流式解码并处理
    if (!decodersMap[pid]) {
      decodersMap[pid] = new TextDecoder('utf-8', { fatal: false })
    }
    const chunkText = decodersMap[pid].decode(new Uint8Array(data), { stream: true })
    const fullText = (textBufferMap[pid] || '') + chunkText
    
    // 按行切分，只处理完整的行，最后未闭合的部分存入缓冲区
    const lines = fullText.split(/\r?\n/)
    if (lines.length > 1) {
      textBufferMap[pid] = lines.pop() || ''
      const completedText = lines.join('\n')
      parseLogLevels(pid, completedText)
    } else {
      textBufferMap[pid] = fullText
    }
  }

  const logStatsMap = ref<Record<string, LogStats>>({})
  function parseLogLevels(pid: string, text: string) {
    if (!logStatsMap.value[pid]) logStatsMap.value[pid] = { error: 0, warn: 0, info: 0, debug: 0 }
    const s = logStatsMap.value[pid]!
    const upper = text.toUpperCase()
    if (/\bERROR\b|\bFATAL\b|\bPANIC\b/.test(upper)) s.error += (upper.match(/\bERROR\b/g)?.length||0) + (upper.match(/\bFATAL\b/g)?.length||0) + (upper.match(/\bPANIC\b/g)?.length||0)
    if (/\bWARN(ING)?\b/.test(upper)) s.warn += (upper.match(/\bWARN(ING)?\b/g)?.length||0)
    if (/\bDEBUG\b/.test(upper)) s.debug += (upper.match(/\bDEBUG\b/g)?.length||0)
    if (/\bINFO\b|\bTRACE\b/.test(upper)) s.info += (upper.match(/\bINFO\b/g)?.length||0) + (upper.match(/\bTRACE\b/g)?.length||0)
  }
  function clearLogStats(pid: string) { logStatsMap.value[pid] = { error: 0, warn: 0, info: 0, debug: 0 } }

  const clearTermSignal = ref(0)
  function clearOutput(projectId: string, commandId: string) {
    const pid = cmdKey(projectId, commandId)
    outputMap.value[pid] = []
    clearLogStats(pid)
    delete decodersMap[pid]
    textBufferMap[pid] = ''
    clearTermSignal.value++
  }
  function getOutput(pid: string): number[] { return outputMap.value[pid] ?? [] }

  // Smart project detection via Rust backend
  async function detectProject(dir: string) {
    try {
      const info = await invoke<{ name: string; language: string; suggest_commands: { name: string; command: string; working_dir: string }[] }>('detect_project', { dir })
      return { name: info.name, lang: info.language, suggestCommands: info.suggest_commands.map(c => ({ name: c.name, command: c.command, workingDir: c.working_dir })) }
    } catch (e) { console.error(e); return null }
  }

  let _unlistenExit: (() => void) | null = null
  let _unlistenPty: (() => void) | null = null
  async function initListeners() {
    _unlistenExit = await listen<{ processId: string }>('process-exited', (e) => {
      runningMap.value[e.payload.processId] = 'stopped'
    })
    _unlistenPty = await listen<{ processId: string; data: number[] }>('pty-output', (e) => {
      bufferPtyOutput(e.payload.processId, e.payload.data)
    })
    await loadWorkflows()
  }
  function destroyListeners() { _unlistenExit?.(); _unlistenPty?.() }

  // ── 快速启动一个空 shell 终端 ──
  let _quickTerminalSeq = 0
  async function startQuickTerminal(): Promise<string | null> {
    const name = `快速终端 ${++_quickTerminalSeq}`
    // 用第一个项目，没有则创建一个
    if (projects.value.length === 0) addProject('默认终端')
    const p = projects.value[0]
    const cmd: Command = { id: crypto.randomUUID(), name, command: '', workingDir: '' }
    p.commands.push(cmd)
    await startCommand(p.id, cmd)
    return cmdKey(p.id, cmd.id)
  }

  /** 获取所有运行中终端的列表（用于选择发送目标） */
  function getRunningTerminals(): { processId: string; name: string }[] {
    return runningTabs.value
      .filter(t => runningMap.value[cmdKey(t.projectId, t.commandId)] === 'running')
      .map(t => ({ processId: cmdKey(t.projectId, t.commandId), name: t.commandName || t.command || '终端' }))
  }

  // ── 发送命令到终端 ──
  async function sendToTerminal(text: string, targetProcessId?: string | null) {
    // 确定目标终端
    let resolvedPid = targetProcessId || undefined

    if (!resolvedPid) {
      const tabs = getRunningTerminals()
      if (tabs.length === 0) {
        const newPid = await startQuickTerminal()
        if (!newPid) { useStatusStore().pushMessage('无法创建终端', 'error'); return }
        resolvedPid = newPid
      } else {
        resolvedPid = tabs[tabs.length - 1].processId
      }
    }

    // 切换到终端 tab
    const idx = runningTabs.value.findIndex(t => cmdKey(t.projectId, t.commandId) === resolvedPid)
    if (idx >= 0) { activeTabIndex.value = idx; activeTabType.value = 'term' }

    // 发送命令
    try {
      const encoded = text + '\r\n'
      await invoke('pty_write', {
        processId: resolvedPid,
        data: Array.from(new TextEncoder().encode(encoded)),
      })
      useStatusStore().pushMessage(`已发送到终端`, 'success')
    } catch (e) {
      useStatusStore().pushMessage(`发送到终端失败: ${e}`, 'error')
    }
  }

  return { projects, selectedProjectId, runningMap, outputMap, logStatsMap, runningTabs, docTabs, toolTabs, memoryTabs, activeTabIndex, activeDocIndex, activeToolIndex, activeMemoryIndex, activeTabType, workflows, pendingInput, frequentWorkflows, favWorkflows, recentTools, clearTermSignal, sidebarTab, mainMode, workflowRunning, workflowProgress, loadProjects, saveProjects, addProject, removeProject, updateProjectName, addCommand, removeCommand, updateCommand, startCommand, stopCommand, restartCommand, closeTab, closeDocTab, openDoc, openDocFromText, clearOutput, clearLogStats, getOutput, initListeners, destroyListeners, cmdKey, detectProject, bufferPtyOutput, loadWorkflows, addWorkflow, removeWorkflow, updateWorkflow, toggleWfFav, runWorkflow, startDefaultTerminal, openTool, closeToolTab, openMemoryTab, closeMemoryTab, toggleMemoryEdit, sendToTerminal, getRunningTerminals, startQuickTerminal }
})
