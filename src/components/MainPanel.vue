<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useProjectStore } from '@/stores/project'
import { useNotesStore } from '@/stores/notes'
import TerminalView from '@/components/TerminalView.vue'
import LogPanel from '@/components/LogPanel.vue'
import NoteEditor from '@/components/notes/NoteEditor.vue'
import JsonFormatter from '@/components/tools/JsonFormatter.vue'
import Base64Tool from '@/components/tools/Base64Tool.vue'
import EnvViewer from '@/components/tools/EnvViewer.vue'
import TimestampTool from '@/components/tools/TimestampTool.vue'
import RegexTester from '@/components/tools/RegexTester.vue'
import PortKiller from '@/components/tools/PortKiller.vue'
import UuidGenerator from '@/components/tools/UuidGenerator.vue'
import SshKeyGenerator from '@/components/tools/SshKeyGenerator.vue'
import SslCertGenerator from '@/components/tools/SslCertGenerator.vue'
import UrlTool from '@/components/tools/UrlTool.vue'
import UnicodeTool from '@/components/tools/UnicodeTool.vue'
import JwtDecoder from '@/components/tools/JwtDecoder.vue'
import HashTool from '@/components/tools/HashTool.vue'
import HtmlEscapeTool from '@/components/tools/HtmlEscapeTool.vue'
import SqlFormatter from '@/components/tools/SqlFormatter.vue'
import DiffViewer from '@/components/tools/DiffViewer.vue'
import ColorConverter from '@/components/tools/ColorConverter.vue'
import ImageBase64 from '@/components/tools/ImageBase64.vue'
import QrTool from '@/components/tools/QrTool.vue'
import TimeCalculator from '@/components/tools/TimeCalculator.vue'
import RadixConverter from '@/components/tools/RadixConverter.vue'
import DnsResolver from '@/components/tools/DnsResolver.vue'
import CronGenerator from '@/components/tools/CronGenerator.vue'
import CaseConverter from '@/components/tools/CaseConverter.vue'
import LoremIpsum from '@/components/tools/LoremIpsum.vue'
import TextLines from '@/components/tools/TextLines.vue'
import SymmetricCrypto from '@/components/tools/SymmetricCrypto.vue'
import RsaCrypto from '@/components/tools/RsaCrypto.vue'
import CssUnits from '@/components/tools/CssUnits.vue'
import SvgHelper from '@/components/tools/SvgHelper.vue'
import IconGenerator from '@/components/tools/IconGenerator.vue'



import NoteFeedView from '@/components/notes/NoteFeedView.vue'
import NoteSettings from '@/components/notes/NoteSettings.vue'
import AiHelper from '@/components/tools/AiHelper.vue'
import FloatingSearch from '@/components/notes/FloatingSearch.vue'

const store = useProjectStore()
const notesStore = useNotesStore()

const activeNoteId = computed(() => notesStore.activeNoteTabId)

// Auto-switch to note tab type when a note tab opens
watch(activeNoteId, (id) => {
  if (id !== null) store.activeTabType = 'note'
})

function getEditingNote(noteId: string | null) {
  if (!noteId) return null
  return notesStore.notes.find(n => n.id === noteId) ?? null
}

function onNoteSaved(note: any) {
  // If this was a new note (tab id empty), update the tab
  const idx = notesStore.noteTabs.findIndex(t => t.id === '' || t.id === note.id)
  if (idx >= 0 && note.id) {
    notesStore.noteTabs[idx] = { id: note.id, title: note.title || '无标题' }
    notesStore.activeNoteTabId = note.id
  }
}
const ctxShow = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxIdx = ref(-1)
const showLogPid = ref('')

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
          new Notification(`📅 备忘待办提醒: ${note.title || '备忘'}`, {
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
  document.addEventListener('keydown', handleKeyDownSearch)
  if (typeof Notification !== 'undefined' && Notification.permission === 'default') {
    Notification.requestPermission()
  }
  checkReminders()
  reminderTimer = setInterval(checkReminders, 15000)
})

onUnmounted(() => {
  document.removeEventListener('click', closeCtx)
  document.removeEventListener('keydown', handleKeyDownSearch)
  if (reminderTimer) clearInterval(reminderTimer)
})
</script>

<template>
  <div class="panel">
    <!-- Tab Bar: terminals + docs + tools + notes -->
    <div class="tabs" v-if="store.runningTabs.length>0||store.docTabs.length>0||store.toolTabs.length>0||notesStore.noteTabs.length>0" role="tablist">
      <div v-for="(t,i) in store.runningTabs" :key="'t'+t.projectId+t.commandId"
        :class="['tab',{on:store.activeTabType==='term'&&i===store.activeTabIndex}]"
        role="tab" :aria-selected="store.activeTabType==='term'&&i===store.activeTabIndex" tabindex="0"
        @click="store.activeTabType='term';store.activeTabIndex=i" @contextmenu="openCtx($event,i)"
        @keyup.enter="store.activeTabType='term';store.activeTabIndex=i">
        <span class="tdot" :class="{live:store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'}"></span>
        <span class="tl">{{ t.commandName }}</span>
        <button class="tx" @click.stop="store.closeTab(i)" aria-label="关闭标签">✕</button>
      </div>
      <div v-for="(t,i) in store.docTabs" :key="'d'+t.id"
        :class="['tab',{on:store.activeTabType==='doc'&&i===store.activeDocIndex}]"
        role="tab" :aria-selected="store.activeTabType==='doc'&&i===store.activeDocIndex" tabindex="0"
        @click="store.activeTabType='doc';store.activeDocIndex=i"
        @keyup.enter="store.activeTabType='doc';store.activeDocIndex=i">
        <span class="tl">{{ t.title }}</span>
        <button class="tx" @click.stop="store.closeDocTab(i)" aria-label="关闭标签">✕</button>
      </div>
      <div v-for="(t,i) in store.toolTabs" :key="'tl'+t.id"
        :class="['tab',{on:store.activeTabType==='tool'&&i===store.activeToolIndex}]"
        role="tab" :aria-selected="store.activeTabType==='tool'&&i===store.activeToolIndex" tabindex="0"
        @click="store.activeTabType='tool';store.activeToolIndex=i"
        @keyup.enter="store.activeTabType='tool';store.activeToolIndex=i">
        <span class="tl">{{ t.title }}</span>
        <button class="tx" @click.stop="store.closeToolTab(i)" aria-label="关闭标签">✕</button>
      </div>
      <div v-for="t in notesStore.noteTabs" :key="'n'+t.id"
        :class="['tab',{on:store.activeTabType==='note'&&activeNoteId===t.id}]"
        role="tab" :aria-selected="store.activeTabType==='note'&&activeNoteId===t.id" tabindex="0"
        @click="store.activeTabType='note'; notesStore.activeNoteTabId = t.id"
        @keyup.enter="store.activeTabType='note'; notesStore.activeNoteTabId = t.id">
        <span class="tl">{{ t.title || '新笔记' }}</span>
        <button class="tx" @click.stop="notesStore.closeNoteTab(t.id); if(!notesStore.activeNoteTabId)store.activeTabType='term'" aria-label="关闭标签">✕</button>
      </div>
    </div>

    <!-- Terminal content -->
    <div v-for="(t,i) in store.runningTabs" :key="'tc'+t.projectId+t.commandId" class="content" v-show="store.activeTabType==='term'&&i===store.activeTabIndex">
      <div class="bar">
        <code class="cmdtext">{{ t.command }}</code>
        <div class="acts">
          <button v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]!=='running'" class="btn pri" @click="()=>{const c=store.projects.find(p=>p.id===t.projectId)?.commands.find(c=>c.id===t.commandId);if(c)store.startCommand(t.projectId,c)}">启动</button>
          <button v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'" class="btn" @click="store.stopCommand(t.projectId,t.commandId)">停止</button>
          <button v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'" class="btn" @click="()=>{const c=store.projects.find(p=>p.id===t.projectId)?.commands.find(c=>c.id===t.commandId);if(c)store.restartCommand(t.projectId,c)}">重启</button>
          <button class="btn" @click="store.clearOutput(t.projectId,t.commandId)">清屏</button>
          <button class="btn" :class="{on:showLogPid===store.cmdKey(t.projectId,t.commandId)}" @click="toggleLog(store.cmdKey(t.projectId,t.commandId))">日志</button>
        </div>
      </div>
      <div class="term-area">
        <TerminalView :process-id="store.cmdKey(t.projectId,t.commandId)" :active="store.activeTabType==='term'&&i===store.activeTabIndex" />
        <LogPanel v-if="showLogPid===store.cmdKey(t.projectId,t.commandId)" :process-id="store.cmdKey(t.projectId,t.commandId)" />
      </div>
    </div>

    <!-- Doc content -->
    <div v-for="(t,i) in store.docTabs" :key="'dc'+t.id" class="content" v-show="store.activeTabType==='doc'&&i===store.activeDocIndex">
      <div class="bar"><code class="cmdtext">{{ t.command }}</code></div>
      <div class="doc-body" v-if="t.loading">加载中...</div>
      <div class="doc-body" v-else>{{ t.content }}</div>
    </div>

    <!-- Tool content -->
    <div v-for="(t,i) in store.toolTabs" :key="'tlc'+t.id" class="content" v-show="store.activeTabType==='tool'&&i===store.activeToolIndex">
      
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
        <SymmetricCrypto v-else-if="t.toolType === 'aes-des'" />
        <RsaCrypto v-else-if="t.toolType === 'rsa'" />
        <CssUnits v-else-if="t.toolType === 'css'" />
        <SvgHelper v-else-if="t.toolType === 'svg'" />
        <IconGenerator v-else-if="t.toolType === 'icon-generator'" />
        <AiHelper v-else-if="t.toolType === 'ai-helper'" />
      </div>
    </div>

    <!-- Note content -->
    <div v-for="t in notesStore.noteTabs" :key="'nc'+t.id" class="content" v-show="store.activeTabType==='note'&&activeNoteId===t.id">
      <div class="bar"><code class="cmdtext">笔记{{ t.id ? ': ' + (getEditingNote(t.id)?.title || '无标题') : '' }}</code></div>
      <div class="note-body">
        <NoteEditor
          v-if="activeNoteId === t.id"
          :existing-note="getEditingNote(t.id) ?? null"
          @saved="onNoteSaved"
          @cancel="notesStore.closeNoteTab(t.id)"
        />
      </div>
    </div>

    <div v-if="store.runningTabs.length===0&&store.docTabs.length===0&&store.toolTabs.length===0&&notesStore.noteTabs.length===0" class="empty-or-feed" :style="store.sidebarTab === 'notes' ? 'flex: 1; display: flex; flex-direction: column; overflow: hidden;' : ''">
      <template v-if="store.sidebarTab==='projects'"><div class="empty">从左侧项目列表点击命令启动</div></template>
      <template v-else-if="store.sidebarTab==='shortcuts'"><div class="empty">点击快捷命令以执行</div></template>
      <template v-else-if="store.sidebarTab==='tools'"><div class="empty">选择工具开始使用</div></template>
      <template v-else-if="store.sidebarTab==='notes'">
        <NoteFeedView @open-settings="notesStore.showSettings = true" />
      </template>
    </div>

    <Teleport to="body">
      <div v-if="ctxShow" class="ctx" :style="{left:ctxPos.x+'px',top:ctxPos.y+'px'}" @click.stop>
        <div class="ci" @click="ctxRestart">重启</div>
        <div class="ci" @click="ctxRename">重命名</div>
        <div class="ci" @click="ctxStop">停止并关闭</div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div v-if="renameShow" class="mbg" @click.self="renameShow=false">
        <div class="mw" style="min-width:360px">
          <div class="mt">命令重命名</div>
          <div class="mb">
            <div class="fld">
              <label>新名称</label>
              <input v-model="renameValue" placeholder="请输入新名称" @keyup.enter="confirmRename" autofocus />
            </div>
            <div class="acts">
              <button class="btn" @click="renameShow=false">取消</button>
              <button class="btn pri" @click="confirmRename">保存</button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- 笔记设置面板 -->
    <NoteSettings :show="notesStore.showSettings" @close="notesStore.showSettings = false" />
    
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
.acts { display:flex; gap:6px; }
.btn { @include btn-base; font-size:11px; }
.btn.pri { @include btn-primary; }
.btn.on { background:var(--jc-color-accent); color:var(--jc-color-white); }
.empty { flex:1; display:flex; align-items:center; justify-content:center; color:var(--jc-text-secondary); font-size:13px; }
.term-area { flex:1; display:flex; overflow:hidden; }
.tool-view-body { flex:1; display:flex; overflow:hidden; }
.doc-body { flex:1; overflow-y:auto; padding:12px; font-family:'Cascadia Code',Consolas,monospace; font-size:12px; color:var(--jc-text-primary); white-space:pre-wrap; background:var(--jc-bg-app); }
.note-body { flex:1; display:flex; overflow:hidden; flex-direction:column; }
.ctx { @include ctx-menu; min-width:130px; }
.ci { @include ctx-item; }
</style>
