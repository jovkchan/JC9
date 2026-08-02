<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, defineAsyncComponent } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '@/stores/project'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import TerminalView from '@/components/TerminalView.vue'
import LogPanel from '@/components/LogPanel.vue'
import JcModal from '@/components/ui/JcModal.vue'
import JcContextMenu from '@/components/ui/JcContextMenu.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import type { JcContextMenuItem } from '@/components/ui'

const memTypeOptions = [
  { label: 'decision', value: 'decision' },
  { label: 'bugfix', value: 'bugfix' },
  { label: 'architecture', value: 'architecture' },
  { label: 'pattern', value: 'pattern' },
  { label: 'config', value: 'config' },
  { label: 'discovery', value: 'discovery' }
]

// 异步载入编辑器和工具组件
const NoteEditor = defineAsyncComponent(() => import('@/components/notes/NoteEditor.vue'))
const JsonFormatter = defineAsyncComponent(() => import('@/components/tools/JsonFormatter.vue'))
const Base64Tool = defineAsyncComponent(() => import('@/components/tools/Base64Tool.vue'))
const EnvViewer = defineAsyncComponent(() => import('@/components/tools/EnvViewer.vue'))
const TimestampTool = defineAsyncComponent(() => import('@/components/tools/TimestampTool.vue'))
const RegexTester = defineAsyncComponent(() => import('@/components/tools/RegexTester.vue'))
const PortKiller = defineAsyncComponent(() => import('@/components/tools/PortKiller.vue'))
const UuidGenerator = defineAsyncComponent(() => import('@/components/tools/UuidGenerator.vue'))
const SshKeyGenerator = defineAsyncComponent(() => import('@/components/tools/SshKeyGenerator.vue'))
const SslCertGenerator = defineAsyncComponent(() => import('@/components/tools/SslCertGenerator.vue'))
const UrlTool = defineAsyncComponent(() => import('@/components/tools/UrlTool.vue'))
const UnicodeTool = defineAsyncComponent(() => import('@/components/tools/UnicodeTool.vue'))
const JwtDecoder = defineAsyncComponent(() => import('@/components/tools/JwtDecoder.vue'))
const HashTool = defineAsyncComponent(() => import('@/components/tools/HashTool.vue'))
const HtmlEscapeTool = defineAsyncComponent(() => import('@/components/tools/HtmlEscapeTool.vue'))
const SqlFormatter = defineAsyncComponent(() => import('@/components/tools/SqlFormatter.vue'))
const DiffViewer = defineAsyncComponent(() => import('@/components/tools/DiffViewer.vue'))
const ColorConverter = defineAsyncComponent(() => import('@/components/tools/ColorConverter.vue'))
const ImageBase64 = defineAsyncComponent(() => import('@/components/tools/ImageBase64.vue'))
const QrTool = defineAsyncComponent(() => import('@/components/tools/QrTool.vue'))
const TimeCalculator = defineAsyncComponent(() => import('@/components/tools/TimeCalculator.vue'))
const RadixConverter = defineAsyncComponent(() => import('@/components/tools/RadixConverter.vue'))
const DnsResolver = defineAsyncComponent(() => import('@/components/tools/DnsResolver.vue'))
const CronGenerator = defineAsyncComponent(() => import('@/components/tools/CronGenerator.vue'))
const CaseConverter = defineAsyncComponent(() => import('@/components/tools/CaseConverter.vue'))
const LoremIpsum = defineAsyncComponent(() => import('@/components/tools/LoremIpsum.vue'))
const TextLines = defineAsyncComponent(() => import('@/components/tools/TextLines.vue'))
const MdToTxt = defineAsyncComponent(() => import('@/components/tools/MdToTxt.vue'))
const SymmetricCrypto = defineAsyncComponent(() => import('@/components/tools/SymmetricCrypto.vue'))
const RsaCrypto = defineAsyncComponent(() => import('@/components/tools/RsaCrypto.vue'))
const CssUnits = defineAsyncComponent(() => import('@/components/tools/CssUnits.vue'))
const SvgHelper = defineAsyncComponent(() => import('@/components/tools/SvgHelper.vue'))
const IconGenerator = defineAsyncComponent(() => import('@/components/tools/IconGenerator.vue'))

const VersionHistory = defineAsyncComponent(() => import('@/components/notes/VersionHistory.vue'))
const AiHelper = defineAsyncComponent(() => import('@/components/tools/AiHelper.vue'))
const FloatingSearch = defineAsyncComponent(() => import('@/components/notes/FloatingSearch.vue'))

const store = useProjectStore()
const notesStore = useNotesStore()

const activeNoteId = computed(() => notesStore.activeNoteTabId)

// 当前模块的标签（按 sidebarTab 隔离）
const showTermTabs = computed(() => store.sidebarTab === 'projects' || store.sidebarTab === 'workflows')
const showToolTabs = computed(() => store.sidebarTab === 'tools')
const showNoteTabs = computed(() => store.sidebarTab === 'notes')
const showMemoryTabs = computed(() => store.sidebarTab === 'memories')
const showDocTabs = computed(() => store.sidebarTab === 'projects' || store.sidebarTab === 'workflows')

// Auto-switch to note tab type when a note tab opens (仅在笔记模块下)
watch(activeNoteId, (id) => {
  if (id !== null && store.sidebarTab === 'notes') store.activeTabType = 'note'
})

// 切换到笔记模块时，如果有打开的笔记编辑器则激活第一个
watch(() => store.sidebarTab, (tab) => {
  if (tab === 'notes' && notesStore.noteTabs.length > 0 && store.activeTabType !== 'note') {
    store.activeTabType = 'note'
    if (!notesStore.activeNoteTabId) {
      notesStore.activeNoteTabId = notesStore.noteTabs[0].id
    }
  }
})

function getNoteGroupPath(noteId: string | null): string {
  if (!noteId) return ''
  const note = notesStore.notes.find(n => n.id === noteId)
  if (!note?.groupId) return ''
  return notesStore.getGroupPath(note.groupId).map(g => g.name).join(' / ')
}

function getEditingNote(noteId: string | null) {
  if (!noteId) return null
  return notesStore.notes.find(n => n.id === noteId) ?? null
}

function onNoteSaved(note: any) {
  const idx = notesStore.noteTabs.findIndex(t => t.id === '' || t.id === note.id)
  if (idx >= 0 && note.id) {
    notesStore.noteTabs[idx] = { id: note.id, title: note.title || '无标题' }
    notesStore.activeNoteTabId = note.id
  }
}

async function saveMemoryTab(index: number) {
  const tab = store.memoryTabs[index]
  if (!tab || !tab.title.trim() || !tab.content.trim()) return
  try {
    await invoke('save_memory', {
      memory: {
        id: tab.id,
        scope: tab.scope,
        topicKey: tab.topicKey,
        title: tab.title,
        content: tab.content,
        memoryType: tab.type || 'discovery',
        tags: ['memory'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
    })
    store.toggleMemoryEdit(index)
    useStatusStore().pushMessage('记忆已保存', 'success')
  } catch (e: any) {
    useStatusStore().pushMessage('保存失败: ' + e, 'error')
  }
}
const ctxShow = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxIdx = ref(-1)
const showLogPid = ref('')

// ── Note tab context menu ──
const noteCtxShow = ref(false)
const noteCtxPos = ref({ x: 0, y: 0 })
const noteCtxTabId = ref<string | null>(null)

function openNoteCtx(e: MouseEvent, tabId: string) {
  e.preventDefault()
  noteCtxPos.value = { x: e.clientX, y: e.clientY }
  noteCtxTabId.value = tabId
  noteCtxShow.value = true
}

function closeNoteCtx() { noteCtxShow.value = false }

function noteCtxClose() {
  if (noteCtxTabId.value) {
    notesStore.closeNoteTab(noteCtxTabId.value)
    if (!notesStore.activeNoteTabId) store.activeTabType = 'term'
  }
  closeNoteCtx()
}

function noteCtxRefresh() {
  // 重新打开同一个笔记（强制刷新编辑器）
  if (noteCtxTabId.value) {
    const id = noteCtxTabId.value
    notesStore.closeNoteTab(id)
    setTimeout(() => notesStore.openNoteTab(id), 50)
  }
  closeNoteCtx()
}

function noteCtxCloseOthers() {
  if (!noteCtxTabId.value) return
  const keep = noteCtxTabId.value
  const ids = [...notesStore.noteTabs.filter(t => t.id !== keep).map(t => t.id)]
  ids.forEach(id => notesStore.closeNoteTab(id))
  notesStore.activeNoteTabId = keep
  closeNoteCtx()
}

function noteCtxCloseRight() {
  if (!noteCtxTabId.value) return
  const tabs = [...notesStore.noteTabs]
  const idx = tabs.findIndex(t => t.id === noteCtxTabId.value)
  if (idx >= 0) tabs.slice(idx + 1).forEach(t => notesStore.closeNoteTab(t.id))
  closeNoteCtx()
}

function noteCtxCloseLeft() {
  if (!noteCtxTabId.value) return
  const tabs = [...notesStore.noteTabs]
  const idx = tabs.findIndex(t => t.id === noteCtxTabId.value)
  if (idx > 0) tabs.slice(0, idx).forEach(t => notesStore.closeNoteTab(t.id))
  closeNoteCtx()
}

function noteCtxCloseAll() {
  const ids = [...notesStore.noteTabs.map(t => t.id)]
  ids.forEach(id => notesStore.closeNoteTab(id))
  store.activeTabType = 'term'
  closeNoteCtx()
}

const renameShow = ref(false)
const renameValue = ref('')

function openCtx(e: MouseEvent, i: number) {
  e.preventDefault()
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxIdx.value = i
  ctxShow.value = true
}

function toggleLog(pid: string) {
  showLogPid.value = showLogPid.value === pid ? '' : pid
}

function closeCtx() {
  ctxShow.value = false
}
const ctxItems: JcContextMenuItem[] = [
  { label: '重启', value: 'restart' },
  { label: '重命名', value: 'rename' },
  { label: '停止并关闭', value: 'stop', danger: true },
]
function onCtxSelect(item: JcContextMenuItem) {
  if (item.value === 'restart') ctxRestart()
  else if (item.value === 'rename') ctxRename()
  else if (item.value === 'stop') ctxStop()
}
const noteCtxItems: JcContextMenuItem[] = [
  { label: '刷新', value: 'refresh' },
  { label: '关闭', value: 'close' },
  { label: '关闭其他', value: 'closeOthers' },
  { label: '关闭右侧标签页', value: 'closeRight' },
  { label: '关闭左侧标签页', value: 'closeLeft' },
  { label: '全部关闭', value: 'closeAll' },
]
function onNoteCtxSelect(item: JcContextMenuItem) {
  switch (item.value) {
    case 'refresh': return noteCtxRefresh()
    case 'close': return noteCtxClose()
    case 'closeOthers': return noteCtxCloseOthers()
    case 'closeRight': return noteCtxCloseRight()
    case 'closeLeft': return noteCtxCloseLeft()
    case 'closeAll': return noteCtxCloseAll()
  }
}

function ctxRestart() {
  const t = store.runningTabs[ctxIdx.value]
  const c = store.projects.find(p => p.id === t.projectId)?.commands.find(c => c.id === t.commandId)
  if (t && c) store.restartCommand(t.projectId, c)
  closeCtx()
}

function ctxStop() {
  const t = store.runningTabs[ctxIdx.value]
  if (t) {
    store.stopCommand(t.projectId, t.commandId)
    store.closeTab(ctxIdx.value)
  }
  closeCtx()
}

function ctxRename() {
  const t = store.runningTabs[ctxIdx.value]
  const c = store.projects.find(p => p.id === t.projectId)?.commands.find(c => c.id === t.commandId)
  if (c) {
    renameValue.value = c.name
    renameShow.value = true
  }
  closeCtx()
}

function confirmRename() {
  const t = store.runningTabs[ctxIdx.value]
  const c = store.projects.find(p => p.id === t.projectId)?.commands.find(c => c.id === t.commandId)
  const n = renameValue.value.trim()
  if (c && n) {
    store.updateCommand(t.projectId, { ...c, name: n })
    store.runningTabs[ctxIdx.value].commandName = n
  }
  renameShow.value = false
}

let reminderTimer: ReturnType<typeof setInterval> | null = null
const notifiedTasks = new Set<string>()

function checkReminders() {
  const now = new Date()
  const y = now.getFullYear()
  const m = String(now.getMonth() + 1).padStart(2, '0')
  const d = String(now.getDate()).padStart(2, '0')
  const hh = String(now.getHours()).padStart(2, '0')
  const mm = String(now.getMinutes()).padStart(2, '0')
  const nowTimeMinute = `${y}-${m}-${d} ${hh}:${mm}`

  notesStore.notes.forEach(note => {
    if (note.isDeleted || note.isArchived) return
    const regex = /-\s+\[\s*\]\s+@(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2})\s+([^\n\r]+)/g
    let match
    while ((match = regex.exec(note.content)) !== null) {
      const taskTimeStr = match[1]
      const taskDesc = match[2]
      const taskKey = `${note.id}-${taskTimeStr}-${taskDesc}`

      if (notifiedTasks.has(taskKey)) continue

      if (nowTimeMinute >= taskTimeStr) {
        if (typeof Notification !== 'undefined' && Notification.permission === 'granted') {
          new Notification(`备忘待办提醒: ${note.title || '备忘'}`, {
            body: taskDesc
          })
          notifiedTasks.add(taskKey)
        }
      }
    }
  })
}

function handleKeyDownSearch(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
    if (store.sidebarTab === 'notes') {
      e.preventDefault()
      notesStore.showSearchPanel = !notesStore.showSearchPanel
    }
  }
}

onMounted(() => {
  document.addEventListener('click', closeCtx)
  document.addEventListener('click', closeNoteCtx)
  document.addEventListener('keydown', handleKeyDownSearch)
  if (typeof Notification !== 'undefined' && Notification.permission === 'default') {
    Notification.requestPermission()
  }
  checkReminders()
  reminderTimer = setInterval(checkReminders, 15000)
})

onUnmounted(() => {
  document.removeEventListener('click', closeCtx)
  document.removeEventListener('click', closeNoteCtx)
  document.removeEventListener('keydown', handleKeyDownSearch)
  if (reminderTimer) clearInterval(reminderTimer)
})
</script>

<template>
  <div class="panel">
    <!-- Tab Bar（按 sidebarTab 模块隔离） -->
    <div class="tabs" v-if="showTermTabs&&store.runningTabs.length>0||showDocTabs&&store.docTabs.length>0||showToolTabs&&store.toolTabs.length>0||showMemoryTabs&&store.memoryTabs.length>0||showNoteTabs&&notesStore.noteTabs.length>0" role="tablist">
      <div v-for="(t,i) in store.runningTabs" :key="'t'+t.projectId+t.commandId"
        v-show="showTermTabs&&(store.sidebarTab==='projects'?t.projectId!=='workflow':t.projectId==='workflow')"
        :class="['tab',{on:store.activeTabType==='term'&&i===store.activeTabIndex}]"
        role="tab" :aria-selected="store.activeTabType==='term'&&i===store.activeTabIndex" tabindex="0"
        @click="store.activeTabType='term';store.activeTabIndex=i" @contextmenu="openCtx($event,i)"
        @keyup.enter="store.activeTabType='term';store.activeTabIndex=i">
        <span class="tdot" :class="{live:store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'}"></span>
        <span class="tl">{{ t.commandName }}</span>
        <button class="tx" @click.stop="store.closeTab(i)" aria-label="关闭标签">✕</button>
      </div>
      <div v-for="(t,i) in store.docTabs" :key="'d'+t.id"
        v-show="showDocTabs"
        :class="['tab',{on:store.activeTabType==='doc'&&i===store.activeDocIndex}]"
        role="tab" :aria-selected="store.activeTabType==='doc'&&i===store.activeDocIndex" tabindex="0"
        @click="store.activeTabType='doc';store.activeDocIndex=i"
        @keyup.enter="store.activeTabType='doc';store.activeDocIndex=i">
        <span class="tl">{{ t.title }}</span>
        <button class="tx" @click.stop="store.closeDocTab(i)" aria-label="关闭标签">✕</button>
      </div>
      <div v-for="(t,i) in store.toolTabs" :key="'tl'+t.id"
        v-show="showToolTabs"
        :class="['tab',{on:store.activeTabType==='tool'&&i===store.activeToolIndex}]"
        role="tab" :aria-selected="store.activeTabType==='tool'&&i===store.activeToolIndex" tabindex="0"
        @click="store.activeTabType='tool';store.activeToolIndex=i"
        @keyup.enter="store.activeTabType='tool';store.activeToolIndex=i">
        <span class="tl">{{ t.title }}</span>
        <button class="tx" @click.stop="store.closeToolTab(i)" aria-label="关闭标签">✕</button>
      </div>
      <div v-for="(t,i) in store.memoryTabs" :key="'mem'+t.id"
        v-show="showMemoryTabs"
        :class="['tab',{on:store.activeTabType==='memory'&&i===store.activeMemoryIndex}]"
        role="tab" :aria-selected="store.activeTabType==='memory'&&i===store.activeMemoryIndex" tabindex="0"
        @click="store.activeTabType='memory';store.activeMemoryIndex=i"
        @keyup.enter="store.activeTabType='memory';store.activeMemoryIndex=i">
        <span class="tl">{{ t.title }}</span>
        <button class="tx" @click.stop="store.closeMemoryTab(i)" aria-label="关闭标签">✕</button>
      </div>
      <div v-for="t in notesStore.noteTabs" :key="'n'+t.id"
        v-show="showNoteTabs"
        :class="['tab',{on:store.activeTabType==='note'&&activeNoteId===t.id}]"
        role="tab" :aria-selected="store.activeTabType==='note'&&activeNoteId===t.id" tabindex="0"
        @click="store.activeTabType='note'; notesStore.activeNoteTabId = t.id"
        @keyup.enter="store.activeTabType='note'; notesStore.activeNoteTabId = t.id"
        @contextmenu="openNoteCtx($event, t.id)">
        <span class="tl">{{ t.title || '新笔记' }}</span>
        <button class="tx" @click.stop="notesStore.closeNoteTab(t.id); if(!notesStore.activeNoteTabId)store.activeTabType='term'" aria-label="关闭标签">✕</button>
      </div>
    </div>

    <!-- Terminal content -->
    <div v-for="(t,i) in store.runningTabs" :key="'tc'+t.projectId+t.commandId" class="content" v-show="showTermTabs&&(store.sidebarTab==='projects'?t.projectId!=='workflow':t.projectId==='workflow')&&store.activeTabType==='term'&&i===store.activeTabIndex">
      <div class="bar">
        <code class="cmdtext">{{ t.command }}</code>
        <div class="acts">
          <JcButton v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]!=='running'" size="small" type="primary" @click="()=>{const c=store.projects.find(p=>p.id===t.projectId)?.commands.find(c=>c.id===t.commandId);if(c)store.startCommand(t.projectId,c)}">启动</JcButton>
          <JcButton v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'" size="small" @click="store.stopCommand(t.projectId,t.commandId)">停止</JcButton>
          <JcButton v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'" size="small" @click="()=>{const c=store.projects.find(p=>p.id===t.projectId)?.commands.find(c=>c.id===t.commandId);if(c)store.restartCommand(t.projectId,c)}">重启</JcButton>
          <JcButton size="small" @click="store.clearOutput(t.projectId,t.commandId)">清屏</JcButton>
          <JcButton size="small" :type="showLogPid===store.cmdKey(t.projectId,t.commandId)?'primary':'default'" @click="toggleLog(store.cmdKey(t.projectId,t.commandId))">日志</JcButton>
        </div>
      </div>
      <div class="term-area">
        <TerminalView :process-id="store.cmdKey(t.projectId,t.commandId)" :active="store.activeTabType==='term'&&i===store.activeTabIndex" />
        <LogPanel v-if="showLogPid===store.cmdKey(t.projectId,t.commandId)" :process-id="store.cmdKey(t.projectId,t.commandId)" />
      </div>
    </div>

    <!-- Doc content -->
    <div v-for="(t,i) in store.docTabs" :key="'dc'+t.id" class="content" v-show="showDocTabs&&store.activeTabType==='doc'&&i===store.activeDocIndex">
      <div class="bar"><code class="cmdtext">{{ t.command }}</code></div>
      <div class="doc-body" v-if="t.loading">加载中...</div>
      <div class="doc-body" v-else>{{ t.content }}</div>
    </div>

    <!-- Tool content -->
    <div v-for="(t,i) in store.toolTabs" :key="'tlc'+t.id" class="content" v-show="showToolTabs&&store.activeTabType==='tool'&&i===store.activeToolIndex">
      
      <div class="tool-view-body">
        <JsonFormatter v-if="t.toolType === 'json'" />
        <Base64Tool v-else-if="t.toolType === 'base64'" />
        <EnvViewer v-else-if="t.toolType === 'env'" />
        <TimestampTool v-else-if="t.toolType === 'timestamp'" />
        <RegexTester v-else-if="t.toolType === 'regex'" />
        <PortKiller v-else-if="t.toolType === 'port'" />
        <UuidGenerator v-else-if="t.toolType === 'uuid'" />
        <SshKeyGenerator v-else-if="t.toolType === 'ssh'" />
        <SslCertGenerator v-else-if="t.toolType === 'ssl'" />
        <UrlTool v-else-if="t.toolType === 'url'" />
        <UnicodeTool v-else-if="t.toolType === 'unicode'" />
        <JwtDecoder v-else-if="t.toolType === 'jwt'" />
        <HashTool v-else-if="t.toolType === 'hash'" />
        <HtmlEscapeTool v-else-if="t.toolType === 'html'" />
        <SqlFormatter v-else-if="t.toolType === 'sql'" />
        <DiffViewer v-else-if="t.toolType === 'diff'" />
        <ColorConverter v-else-if="t.toolType === 'color'" />
        <ImageBase64 v-else-if="t.toolType === 'img-base64'" />
        <QrTool v-else-if="t.toolType === 'qr'" />
        <TimeCalculator v-else-if="t.toolType === 'time-calc'" />
        <RadixConverter v-else-if="t.toolType === 'radix'" />
        <DnsResolver v-else-if="t.toolType === 'dns'" />
        <CronGenerator v-else-if="t.toolType === 'cron'" />
        <CaseConverter v-else-if="t.toolType === 'case'" />
        <LoremIpsum v-else-if="t.toolType === 'lorem'" />
        <TextLines v-else-if="t.toolType === 'lines'" />
        <MdToTxt v-else-if="t.toolType === 'md-txt'" />
        <SymmetricCrypto v-else-if="t.toolType === 'aes-des'" />
        <RsaCrypto v-else-if="t.toolType === 'rsa'" />
        <CssUnits v-else-if="t.toolType === 'css'" />
        <SvgHelper v-else-if="t.toolType === 'svg'" />
        <IconGenerator v-else-if="t.toolType === 'icon-generator'" />
        <AiHelper v-else-if="t.toolType === 'ai-helper'" />
      </div>
    </div>

    <!-- Note content -->
    <div v-for="t in notesStore.noteTabs" :key="'nc'+t.id" class="content" v-show="showNoteTabs&&store.activeTabType==='note'&&activeNoteId===t.id">
      <div class="bar">
        
        <code class="cmdtext">笔记{{ t.id ? ': ' + (getEditingNote(t.id)?.title || '无标题') : '' }}</code>
        <span v-if="getNoteGroupPath(t.id)" class="note-group-path">{{ getNoteGroupPath(t.id) }}</span>
      </div>
      <div class="note-body" :class="{ 'with-history': notesStore.showVersionHistory }">
        <div class="note-editor-wrapper">
          <NoteEditor
            v-if="activeNoteId === t.id"
            :existing-note="getEditingNote(t.id) ?? null"
            @saved="onNoteSaved"
            @cancel="notesStore.closeNoteTab(t.id)"
          />
        </div>
        <VersionHistory v-if="notesStore.showVersionHistory && activeNoteId === t.id" />
      </div>
    </div>

    <!-- Memory detail content -->
    <div v-for="(t,i) in store.memoryTabs" :key="'memc'+t.id" class="content" v-show="showMemoryTabs&&store.activeTabType==='memory'&&i===store.activeMemoryIndex">
      <div class="bar">
        <code class="cmdtext">{{ t.title }}</code>
        <div class="acts">
          <!-- 编辑模式：取消/保存放标题栏 -->
          <template v-if="t.editing">
            <button class="btn" @click="store.toggleMemoryEdit(i)">取消</button>
            <button class="btn pri" @click="saveMemoryTab(i)">保存</button>
          </template>
          <!-- 查看模式：编辑按钮 -->
          <button v-else class="btn" @click="store.toggleMemoryEdit(i)">编辑</button>
        </div>
      </div>
      <!-- 查看模式 -->
      <div v-if="!t.editing" class="memory-detail-view">
        <div class="mem-meta">
          <span class="mem-badge type">{{ t.type }}</span>
          <span v-if="t.scope" class="mem-badge scope">{{ t.scope }}</span>
        </div>
        <pre class="mem-content">{{ t.content }}</pre>
      </div>
      <!-- 编辑模式 -->
      <div v-else class="memory-edit-view">
        <div class="mem-edit-field">
          <label>标题</label>
          <JcInput v-model="t.title" />
        </div>
        <div class="mem-edit-field">
          <label>类型</label>
          <JcSelect :model-value="t.type" :options="memTypeOptions" style="width: 100%" @update:model-value="(v) => t.type = v as string" />
        </div>
        <div class="mem-edit-field">
          <label>Scope</label>
          <JcInput v-model="t.scope" placeholder="项目标识" />
        </div>
        <div class="mem-edit-field">
          <label>Topic Key</label>
          <JcInput v-model="t.topicKey" placeholder="去重键" />
        </div>
        <div class="mem-edit-field content-field">
          <label>内容</label>
          <JcTextarea v-model="t.content" class="jc-fill" />
        </div>
      </div>
    </div>

    <div v-if="store.runningTabs.length===0&&store.docTabs.length===0&&store.toolTabs.length===0&&store.memoryTabs.length===0&&notesStore.noteTabs.length===0" class="empty-or-feed" style="flex:1;display:flex">
      <div class="empty">从左侧面板选择功能开始使用</div>
    </div>

    <JcContextMenu :show="ctxShow" :x="ctxPos.x" :y="ctxPos.y" :items="ctxItems" @select="onCtxSelect" @update:show="ctxShow = $event" />

    <JcModal v-model:open="renameShow" title="命令重命名" width="360">
            <div class="fld">
              <label>新名称</label>
              <input v-model="renameValue" placeholder="请输入新名称" @keyup.enter="confirmRename" autofocus />
            </div>
      <template #footer>
        <JcButton @click="renameShow=false">取消</JcButton>
        <JcButton type="primary" @click="confirmRename">保存</JcButton>
      </template>
    </JcModal>

    <!-- 笔记标签页右键菜单 -->
    <JcContextMenu :show="noteCtxShow" :x="noteCtxPos.x" :y="noteCtxPos.y" :items="noteCtxItems" @select="onNoteCtxSelect" @update:show="noteCtxShow = $event" />

    <!-- 笔记设置面板 -->
    <!-- 全局浮动搜索面板 -->
    <FloatingSearch />
  </div>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;
.panel { flex:1; display:flex; flex-direction:column; overflow:hidden; min-width:0; }
.tabs { display:flex; background:var(--jc-bg-elevated); overflow-x:auto; flex-shrink:0; }
.tab { display:flex; align-items:center; gap:4px; padding:6px 12px; font-size:12px; cursor:pointer; color:var(--jc-text-secondary); border-right:1px solid var(--jc-border-default); white-space:nowrap;
  &:hover { color:var(--jc-text-primary); background:var(--jc-bg-hover); }
  &.on { color:var(--jc-text-highlight); background:var(--jc-bg-app); }
}
.tdot { @include dot; }
.tl { max-width:160px; overflow:hidden; text-overflow:ellipsis; }
.tx { background:none; color:var(--jc-text-secondary); font-size:14px; padding:0 4px; cursor:pointer;
  &:hover { color:var(--jc-color-error); }
}
.content { flex:1; display:flex; flex-direction:column; overflow:hidden; }
.bar { @include bar; }
.cmdtext { font-size:11px; color:var(--jc-color-success); font-family:'Cascadia Code',Consolas,monospace; }
.note-group-path { font-size:10px; color:var(--jc-text-secondary); margin-right:8px; }
.acts { display:flex; gap:6px; }
.btn { @include btn-base; font-size:11px; }
.btn.pri { @include btn-primary; }
.btn.on { background:var(--jc-color-accent); color:var(--jc-color-white); }
.empty { flex:1; display:flex; align-items:center; justify-content:center; color:var(--jc-text-secondary); font-size:13px; }
.term-area { flex:1; display:flex; overflow:hidden; }
.tool-view-body { flex:1; display:flex; overflow:hidden; }
.doc-body { flex:1; overflow-y:auto; padding:12px; font-family:'Cascadia Code',Consolas,monospace; font-size:12px; color:var(--jc-text-primary); white-space:pre-wrap; background:var(--jc-bg-app); }
.note-body { flex:1; display:flex; overflow:hidden; flex-direction:column; }
.note-body.with-history { flex-direction:row; }
.memory-detail-view { flex:1; overflow-y:auto; padding:16px; background:var(--jc-bg-app); }
.memory-edit-view { flex:1; overflow:hidden; padding:16px; background:var(--jc-bg-app); display:flex; flex-direction:column; gap:10px; }
.mem-edit-field { display:flex; flex-direction:column; gap:3px; flex-shrink:0; }
.mem-edit-field.content-field { flex:1; min-height:0; display:flex; flex-direction:column; }
.mem-edit-field.content-field textarea { flex:1; resize:none; }
.mem-edit-field label { font-size:11px; color:var(--jc-text-secondary); }
.mem-edit-input { padding:6px 8px; font-size:12px; border:1px solid var(--jc-border-default); border-radius:4px; background:var(--jc-bg-app); color:var(--jc-text-primary); }
.mem-edit-textarea { padding:6px 8px; font-size:12px; border:1px solid var(--jc-border-default); border-radius:4px; background:var(--jc-bg-app); color:var(--jc-text-primary); resize:none; font-family:inherit; }
.mem-edit-actions { display:flex; gap:6px; justify-content:flex-end; }
.mem-meta { display:flex; gap:6px; margin-bottom:12px; }
.mem-badge { font-size:10px; padding:2px 8px; border-radius:4px; }
.mem-badge.type { background:var(--jc-color-accent); color:var(--jc-color-white); }
.mem-badge.scope { background:rgba(88,166,255,0.15); color:#58a6ff; }
.mem-content { font-size:12px; line-height:1.6; white-space:pre-wrap; color:var(--jc-text-primary); }
.note-editor-wrapper { flex:1; display:flex; flex-direction:column; min-height:0; min-width:0; }
.ctx { @include ctx-menu; min-width:130px; }
.ci { @include ctx-item; }
</style>
