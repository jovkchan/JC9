import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Command, Project, RunningStatus } from '@/types'

function genId() { return crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2, 9)}` }

export interface RunningTab { projectId: string; projectName: string; commandId: string; commandName: string; command: string }
export interface DocTab { id: string; title: string; command: string; content: string; loading: boolean }
export interface ToolTab { id: string; title: string; toolType: string }
export interface ShortcutItem { id: string; name: string; command: string; category: string; description: string; favorite?: boolean; useCount?: number }
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
  const activeTabIndex = ref(0)
  const activeDocIndex = ref(-1)
  const activeToolIndex = ref(-1)
  const activeTabType = ref<'term'|'doc'|'tool'>('term')
  const shortcuts = ref<ShortcutItem[]>([])
  const pendingInput = ref('')
  const recentTools = ref<string[]>(JSON.parse(localStorage.getItem('jc9-recent-tools') || '[]'))

  async function loadShortcuts() { try { shortcuts.value = await invoke<ShortcutItem[]>('get_shortcuts') } catch (e) { console.error(e) } }
  async function saveShortcuts(userOnly: ShortcutItem[]) { try { await invoke('save_shortcuts', { shortcuts: userOnly }) } catch (e) { console.error(e) } }
  function addShortcut(s: Omit<ShortcutItem, 'id'>) { const item: ShortcutItem = { ...s, id: genId() }; shortcuts.value.push(item); saveShortcuts(shortcuts.value.filter(x => !isBuiltin(x.id))) }
  function removeShortcut(id: string) { shortcuts.value = shortcuts.value.filter(s => s.id !== id); saveShortcuts(shortcuts.value.filter(x => !isBuiltin(x.id))) }
  function isBuiltin(id: string) { return id.startsWith('go-') || id.startsWith('npm-') || id.startsWith('yarn-') || id.startsWith('npx-') || id.startsWith('git-') }
  function useShortcut(s: ShortcutItem) { s.useCount = (s.useCount||0) + 1; if (!isBuiltin(s.id)) saveShortcuts(shortcuts.value.filter(x => !isBuiltin(x.id))); pendingInput.value = s.command }
  function toggleFav(id: string) { const s = shortcuts.value.find(x=>x.id===id); if(s){s.favorite=!s.favorite; if(!isBuiltin(id)) saveShortcuts(shortcuts.value.filter(x=>!isBuiltin(x.id)))} }
  function updateShortcut(id: string, data: Partial<ShortcutItem>) { const s = shortcuts.value.find(x=>x.id===id); if(s){ Object.assign(s, data); if(!isBuiltin(id)) saveShortcuts(shortcuts.value.filter(x=>!isBuiltin(x.id))) } }
  const frequentShortcuts = computed(() => [...shortcuts.value].filter(s=>(s.useCount||0)>0).sort((a,b)=>(b.useCount||0)-(a.useCount||0)))
  const favShortcuts = computed(() => shortcuts.value.filter(s=>s.favorite))

  let _defaultTerminalStarted = false
  function startDefaultTerminal() {
    if (_defaultTerminalStarted || runningTabs.value.length > 0) return; _defaultTerminalStarted = true
    if (projects.value.length === 0) addProject('默认终端')
    const p = projects.value[0]; if (p.commands.length === 0) addCommand(p.id, { name: '终端', command: '', workingDir: '' })
    startCommand(p.id, p.commands[0])
  }

  async function loadProjects() { try { projects.value = await invoke<Project[]>('get_projects') } catch (e) { console.error(e) } }
  async function saveProjects() { try { await invoke('save_all_projects', { projects: JSON.parse(JSON.stringify(projects.value)) }) } catch (e) { console.error(e) } }

  function addProject(name: string) {
    const p: Project = { id: genId(), name, commands: [], createdAt: new Date().toISOString() }
    projects.value.push(p); selectedProjectId.value = p.id; saveProjects()
  }
  function removeProject(id: string) {
    projects.value.find(p => p.id === id)?.commands.forEach(c => stopCommand(id, c.id))
    projects.value = projects.value.filter(p => p.id !== id)
    if (selectedProjectId.value === id) selectedProjectId.value = projects.value[0]?.id ?? null
    saveProjects()
  }
  function updateProjectName(id: string, name: string) { const p = projects.value.find(p => p.id === id); if (p) { p.name = name; saveProjects() } }

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
    try { await invoke('start_command', { processId, projectId, commandId: cmd.id, workingDir: cmd.workingDir, command: cmd.command }) }
    catch (e) { runningMap.value[processId] = 'stopped'; bufferPtyOutput(processId, [...new TextEncoder().encode(`[启动失败] ${e}`)]) }
  }

  async function stopCommand(projectId: string, commandId: string) { const processId = cmdKey(projectId, commandId); try { await invoke('stop_command_by_ids', { projectId, commandId }) } catch { /* */ }; runningMap.value[processId] = 'stopped' }
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
    await loadShortcuts()
  }
  function destroyListeners() { _unlistenExit?.(); _unlistenPty?.() }

  return { projects, selectedProjectId, runningMap, outputMap, logStatsMap, runningTabs, docTabs, toolTabs, activeTabIndex, activeDocIndex, activeToolIndex, activeTabType, shortcuts, pendingInput, frequentShortcuts, favShortcuts, recentTools, clearTermSignal, loadProjects, saveProjects, addProject, removeProject, updateProjectName, addCommand, removeCommand, updateCommand, startCommand, stopCommand, restartCommand, closeTab, closeDocTab, openDoc, openDocFromText, clearOutput, clearLogStats, getOutput, initListeners, destroyListeners, cmdKey, detectProject, bufferPtyOutput, loadShortcuts, addShortcut, removeShortcut, updateShortcut, isBuiltin, useShortcut, toggleFav, startDefaultTerminal, openTool, closeToolTab }
})
