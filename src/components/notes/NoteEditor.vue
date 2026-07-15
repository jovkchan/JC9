<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useExecInTerminal } from '@/composables/useExecInTerminal'
import { invoke } from '@tauri-apps/api/core'
import type { Note } from '@/types/notes'
import { mergeAttributes } from '@tiptap/core'
import Link from '@tiptap/extension-link'
import { TableRow } from '@tiptap/extension-table'

// Yiitap: full WYSIWYG editor
import { YiiEditor, OBlockquote, OTable, OTableCell, OTableHeader, OTableWrapper, DetailsSummary, DetailsContent } from '@yiitap/vue'
import 'katex/dist/katex.min.css'

// 自定义链接：jclink:// 渲染为 span（避免浏览器导航）
const CustomLink = Link.extend({
  renderHTML({ HTMLAttributes }) {
    const href = (HTMLAttributes.href as string) || ''
    if (href.startsWith('jclink://note/')) {
      return ['span', mergeAttributes(HTMLAttributes, {
        class: 'note-link',
        'data-note-id': href.replace('jclink://note/', ''),
        title: href,
      }), 0]
    }
    return ['a', mergeAttributes(HTMLAttributes, { rel: 'noopener noreferrer' }), 0]
  },
})

// 自定义 Blockquote，将 content 变更为 block*（原来是 block+），支持空引用块，防止 ProseMirror 报错
const CustomBlockquote = OBlockquote.extend({
  content: 'block*',
})

import { Plugin, PluginKey } from 'prosemirror-state'
import SlashCommandMenu from './SlashCommandMenu.vue'

// ── Slash Command State ──
const slashMenuVisible = ref(false)
const slashMenuRect = ref<{ top: number; left: number } | null>(null)
const slashFromPos = ref<number | null>(null)  // position of '/' in the doc

function showSlashMenu(view: any, pos: number) {
  const coords = view.coordsAtPos(pos)
  const menuHeight = 320 // 菜单最大高度
  const spaceBelow = window.innerHeight - coords.bottom
  const spaceAbove = coords.top
  // 下面空间不足且上方空间更大 → 向上弹出
  const flipUp = spaceBelow < menuHeight && spaceAbove > spaceBelow
  const gap = 8 // 光标与菜单间距
  slashMenuRect.value = {
    top: flipUp ? undefined : coords.bottom + gap,
    bottom: flipUp ? window.innerHeight - coords.top + gap : undefined,
    left: coords.left,
    flipUp,
  } as any
  slashMenuVisible.value = true
  slashFromPos.value = pos
}

function hideSlashMenu() {
  slashMenuVisible.value = false
  slashMenuRect.value = null
  slashFromPos.value = null
}

function slashInsertLink(noteId: string, noteTitle: string) {
  const ed = editorRef.value
  if (!ed || slashFromPos.value === null) return
  const from = slashFromPos.value
  ed.chain()
    .focus()
    .deleteRange({ from, to: ed.state.selection.from })
    .insertContent({
      type: 'text',
      text: noteTitle,
      marks: [{ type: 'link', attrs: { href: `jclink://note/${noteId}` } }],
    })
    .run()
  hideSlashMenu()
}

function slashExecuteCommand(cmdId: string) {
  const ed = editorRef.value
  if (!ed) return
  // 删除 / 字符
  if (slashFromPos.value !== null) {
    const from = slashFromPos.value
    ed.chain().focus().deleteRange({ from, to: ed.state.selection.from }).run()
  }
  switch (cmdId) {
    case 'table': ed.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(); break
    case 'code': ed.chain().focus().toggleCodeBlock().run(); break
    case 'hr': ed.chain().focus().setHorizontalRule().run(); break
    case 'task': ed.chain().focus().toggleTaskList().run(); break
  }
  hideSlashMenu()
}

// 笔记链接点击跳转
const isDark = ref(document.documentElement.getAttribute('data-theme') === 'dark')
let themeObserver: MutationObserver | null = null

function handleJclinkClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  const linkEl = target.closest('.note-link') as HTMLElement | null
  if (linkEl) {
    e.preventDefault()
    e.stopPropagation()
    const linkId = linkEl.dataset.noteId
    if (!linkId) return
    const fullId = store.notes.find(n => n.id.startsWith(linkId))?.id
    if (fullId) store.openNoteTab(fullId)
  }
}

function handleGlobalKeydown(e: KeyboardEvent) {
  // Ctrl + S (或 Cmd + S) 手动保存当前笔记
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
    e.preventDefault()
    doSave(false)
  }
  // Ctrl + Enter (或 Cmd + Enter) 手动保存并创建历史快照
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    doSave(true)
  }
}

onMounted(() => {
  document.addEventListener('click', handleJclinkClick)
  window.addEventListener('keydown', handleGlobalKeydown)
  // 监听明暗模式属性变化
  isDark.value = document.documentElement.getAttribute('data-theme') === 'dark'
  themeObserver = new MutationObserver(() => {
    isDark.value = document.documentElement.getAttribute('data-theme') === 'dark'
  })
  themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })
})
onBeforeUnmount(() => {
  document.removeEventListener('click', handleJclinkClick)
  window.removeEventListener('keydown', handleGlobalKeydown)
  if (themeObserver) {
    themeObserver.disconnect()
  }
})

// ProseMirror plugin: detect '/' at start of line
const slashPluginKey = new PluginKey('slash-command')
const slashPlugin = new Plugin({
  key: slashPluginKey,
  props: {
    handleTextInput(view, from, _to, text) {
      if (text !== '/') return false
      const $pos = view.state.doc.resolve(from)
      const nodeBefore = $pos.nodeBefore
      const isAtLineStart = $pos.parentOffset === 0
      const isAfterSpace = nodeBefore?.isText && nodeBefore.text?.endsWith(' ')
      const isAfterNewline = !nodeBefore
      if (!isAtLineStart && !isAfterSpace && !isAfterNewline) return false
      // 不在代码块内触发
      if (view.state.selection.$head.parent.type.name === 'codeBlock') return false
      showSlashMenu(view, from)
      return false
    },
    handleKeyDown(_view, event) {
      if (!slashMenuVisible.value) return false
      if (event.key === 'Escape') {
        hideSlashMenu()
        return true
      }
      if (['ArrowUp', 'ArrowDown', 'Enter'].includes(event.key) && slashMenuVisible.value) {
        return true // 由菜单自己处理键盘
      }
      return false
    },
  },
})
void slashPlugin

const store = useNotesStore()
const { ctxShow, ctxStyle, runningTerminals, openCtx, closeCtx, execInTerminal, createAndExec } = useExecInTerminal()

// ── 右键菜单与代码块快捷运行集成 ──
const hasSelection = ref(false)
const selectedText = ref('')
const codeBlockText = ref<string | null>(null)

/** 检测光标是否处于代码块内，若是则提取其文本 */
function checkActiveCodeBlock() {
  const ed = editorRef.value
  if (!ed) {
    codeBlockText.value = null
    return
  }
  const { state } = ed
  const { selection } = state
  const { $from } = selection
  let foundText: string | null = null
  for (let d = $from.depth; d > 0; d--) {
    const node = $from.node(d)
    if (node.type.name === 'codeBlock') {
      foundText = node.textContent || ''
      break
    }
  }
  codeBlockText.value = foundText
}

function handleEditorContextMenu(e: MouseEvent) {
  const ed = editorRef.value
  if (!ed) return

  const sel = ed.state.selection
  if (sel && !sel.empty) {
    const text = ed.state.doc.textBetween(sel.from, sel.to)
    if (text && text.trim()) {
      hasSelection.value = true
      selectedText.value = text.trim()
      checkActiveCodeBlock()
      openCtx(e, selectedText.value)
      return
    }
  }

  hasSelection.value = false
  selectedText.value = ''
  checkActiveCodeBlock()

  if (codeBlockText.value !== null) {
    // 在代码块内右键，且无选中文本，快捷提取代码块执行
    openCtx(e, codeBlockText.value)
  } else {
    // 常规右键菜单
    openCtx(e, '')
  }
}

// ── 右键菜单内插入命令快捷触发 ──
function rightClickInsertLink() {
  closeCtx()
  const ed = editorRef.value
  if (!ed) return
  const view = ed.view
  const pos = ed.state.selection.from
  showSlashMenu(view, pos)
}

const props = defineProps<{
  existingNote?: Note | null
}>()

const emit = defineEmits<{
  saved: [note: Note]
  cancel: []
}>()

/** 从草稿中恢复内容（切换标签页后重新打开时使用） */
function getDraftForNote(noteId: string | null | undefined) {
  if (!noteId) return null
  return store.noteContentDrafts[noteId] ?? null
}

const noteIdForDraft = props.existingNote?.id ?? null
const existingDraft = getDraftForNote(noteIdForDraft)

const title = ref(existingDraft?.title ?? props.existingNote?.title ?? '')
const tagInput = ref(existingDraft?.tags?.join(', ') ?? props.existingNote?.tags.join(', ') ?? '')
const saving = ref(false)
const lastSaved = ref('')
const editNoteId = ref<string | null>(props.existingNote?.id ?? null)
const activePopover = ref<string | null>(null)

/** 标志：是否正在自身保存，避免 watcher 回写编辑器时光标跳动 */
let selfSaving = false
/** 标志：切换笔记或外部加载中，不触发自动保存 */
let saveTimer: ReturnType<typeof setTimeout> | null = null

// ── 响应外部笔记变更（MCP 等外部修改或侧栏切换自动刷新）──
watch(() => props.existingNote, async (newNote) => {
  if (!newNote) return
  if (selfSaving) return

  try {
    if (newNote.title !== title.value) {
      title.value = newNote.title
    }
    const isNoteSwitched = editNoteId.value !== newNote.id
    if (isNoteSwitched) {
      editNoteId.value = newNote.id
    }
    const newTagsStr = newNote.tags.join(', ')
    if (newTagsStr !== tagInput.value) {
      tagInput.value = newTagsStr
      syncTags()
    }
    if (editorRef.value) {
      // 只有在真正切换了笔记（不同 ID）时，才重新载入编辑器内容；相同笔记时不覆盖载入，防止与自动保存机制冲突形成加载死循环
      if (isNoteSwitched) {
        editorRef.value.commands.setContent(newNote.content, { contentType: 'markdown' })
      }
    }
  } finally {
    // 延迟设为 false，确保所有的 update/watcher 已经消化完毕
    await nextTick()
  }
}, { deep: true })

const tags = ref<string[]>([])
function syncTags() {
  tags.value = tagInput.value
    .split(/[,，]/)
    .map(t => t.trim())
    .filter(Boolean)
}
syncTags()

// 实时提取正文内联标签（不含代码块内）
const inlineTags = ref<string[]>([])
function extractInlineTags(md: string): string[] {
  const cleaned = md.replace(/```[\s\S]*?```/g, '').replace(/`[^`]+`/g, '')
  const matches = cleaned.match(/#([^\s#.,;:!?()（）\[\]{}]+)/g)
  if (!matches) return []
  return Array.from(new Set(matches.map(m => m.slice(1).trim()).filter(t => t.length >= 1 && t.length <= 40)))
}

let tagScanTimer: ReturnType<typeof setInterval> | null = null
function startTagScan() {
  tagScanTimer = setInterval(() => {
    const md = editorRef.value?.getMarkdown() ?? ''
    inlineTags.value = extractInlineTags(md)
  }, 500)
}
onBeforeUnmount(() => { if (tagScanTimer) clearInterval(tagScanTimer) })

function mergeInlineTag(tag: string) {
  if (!tags.value.includes(tag)) {
    tags.value.push(tag)
    tagInput.value = tags.value.join(', ')
  }
}

function mergeAllInlineTags() {
  for (const t of inlineTags.value) {
    if (!tags.value.includes(t)) tags.value.push(t)
  }
  tagInput.value = tags.value.join(', ')
}

// ── YiiEditor 响应式引用 ──
const yiiEditor = ref<InstanceType<typeof YiiEditor>>()
const editorRef = ref<any>(null)

// 响应式扩展列表
const editorExtensions = computed(() => [
  'Markdown',
  'Highlight',
  'TextAlign',
  'Color',
  'FontFamily',
  'BackgroundColor',
  'TextStyle',
  'Typography',
  'Subscript',
  'Superscript',
  'OBlockMath',
  'InlineMath',
  'TaskList',
  'TaskItem',
  OTable,
  TableRow,
  OTableCell,
  OTableHeader,
  OTableWrapper,
  CustomBlockquote,
  'OCallout',
  'OCodeBlock',
  'OHorizontalRule',
  'OImage',
  'OVideo',
  'OAudio',
  'OEmbed',
  'OMultiColumn',
  'ODetails',
  DetailsSummary,
  DetailsContent,
  'OShortcut',
  'OSelectionDecoration',
  'OColorHighlighter',
  'OColon',
  'OSlash',
  'OSlashZh',
  'Focus',
  'UniqueID',
  'OAiBlock',
  'OModelViewer',
  CustomLink.configure({ openOnClick: true, HTMLAttributes: { rel: 'noopener noreferrer' } }),
])

function onEditorCreate(instance: any) {
  editorRef.value = instance
  startTagScan()

  // 优先从草稿恢复内容（切换标签页再返回时使用），其次从已有笔记加载
  const draft = editNoteId.value ? getDraftForNote(editNoteId.value) : null
  const contentToLoad = draft?.content ?? props.existingNote?.content ?? ''

  if (contentToLoad) {
    try {
      instance.commands.setContent(contentToLoad, { contentType: 'markdown' })
      // 如果有草稿且存在已有笔记，同步草稿中的标题/标签到组件本地状态
      if (draft && props.existingNote) {
        if (draft.title && draft.title !== props.existingNote.title) {
          title.value = draft.title
        }
        if (draft.tags?.length) {
          const draftTagsStr = draft.tags.join(', ')
          if (draftTagsStr !== props.existingNote.tags.join(', ')) {
            tagInput.value = draftTagsStr
            syncTags()
          }
        }
      }
    } catch (e: any) {
      console.error("❌ 加载笔记 markdown 失败:", e)
      console.error("❌ 错误堆栈:", e.stack)
    }
  }
}

function onEditorUpdate() {
  // 手动保存模式：编辑时不触发自动保存
  // 但记录草稿以便关闭标签时自动保存
  updateDraft()
}

/** 将当前编辑器内容写入 store 草稿（供关闭标签时自动保存使用） */
function updateDraft() {
  const noteId = editNoteId.value
  const md = editorRef.value?.getMarkdown() ?? ''
  // 新建笔记尚无 ID，用空字符串作 key
  store.updateNoteDraft(noteId || '', {
    title: title.value,
    content: md,
    tags: tags.value,
  })
}


// YiiEditor 内置菜单栏配置
const defaultMenu: string[] = [
  'bold', 'italic', 'text-format-dropdown', 'separator',
  'heading', 'font-family', 'text-color-dropdown', 'color', 'highlight', 'clearFormat', 'separator',
  'align-dropdown', 'separator',
  'horizontalRule', 'blockquote', 'list-dropdown', 'codeBlock', 'link', 'table', 'callout', 'emoji', 'separator',
  'extension-dropdown',
]

const bubbleMenu: string[] = [
  'bold', 'strike', 'text-color-dropdown', 'highlight', 'clearFormat', 'separator',
  'list-group', 'link', 'callout', 'separator',
  'align-dropdown', 'more',
]

const floatingMenu: string[] = [
  'style-dropdown', 'separator',
  'bold', 'italic', 'text-color-dropdown', 'separator',
  'align-dropdown',
]

// ── Toolbar helpers ──
function execCmd(cmd: string, value?: string) {
  const ed = editorRef.value
  if (!ed) return
  const chain = ed.chain().focus() as any
  switch (cmd) {
    case 'bold': chain.toggleBold().run(); break
    case 'italic': chain.toggleItalic().run(); break
    case 'underline': chain.toggleUnderline().run(); break
    case 'strike': chain.toggleStrike().run(); break
    case 'code': chain.toggleCode().run(); break
    case 'heading': chain.toggleHeading({ level: Number(value) as 1 | 2 | 3 | 4 | 5 | 6 }).run(); break
    case 'bulletList': ed.chain().focus().toggleBulletList().run(); break
    case 'orderedList': ed.chain().focus().toggleOrderedList().run(); break
    case 'taskList': ed.chain().focus().toggleTaskList().run(); break
    case 'blockquote': ed.chain().focus().toggleBlockquote().run(); break
    case 'codeBlock': ed.chain().focus().toggleCodeBlock().run(); break
    case 'horizontalRule': ed.chain().focus().setHorizontalRule().run(); break
    case 'highlight': ed.chain().focus().toggleHighlight().run(); break
    case 'undo': ed.chain().focus().undo().run(); break
    case 'redo': ed.chain().focus().redo().run(); break
    case 'link': {
      const url = prompt('输入链接 URL:', 'https://')
      if (url) ed.chain().focus().setLink({ href: url }).run()
      break
    }
    case 'table': ed.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(); break
    case 'addRowAfter': ed.chain().focus().addRowAfter().run(); break
    case 'addColAfter': ed.chain().focus().addColumnAfter().run(); break
    case 'deleteTable': ed.chain().focus().deleteTable().run(); break
    default: break
  }
  activePopover.value = null
}

async function doSave(createVersion = false) {
  const md = editorRef.value?.getMarkdown() ?? ''

  saving.value = true
  syncTags()
  mergeAllInlineTags()

  selfSaving = true
  try {
    if (editNoteId.value) {
      const existing = store.notes.find(n => n.id === editNoteId.value)
      if (!existing) { selfSaving = false; saving.value = false; return }
      const note: Note = {
        ...existing,
        title: title.value || '无标题',
        content: md,
        format: 'markdown',
        tags: tags.value,
        updatedAt: new Date().toISOString(),
      }
      await store.saveNote(note, createVersion)
      emit('saved', note)
    } else {
      const note = await store.createNote({
        title: title.value || '无标题',
        content: md,
        format: 'markdown',
        tags: tags.value,
        groupId: store.selectedGroupId,
        visibility: 'PRIVATE',
      })
      if (note) {
        editNoteId.value = note.id
        emit('saved', note)
      }
    }
  } finally {
    selfSaving = false
  }
  lastSaved.value = new Date().toLocaleTimeString()
  saving.value = false
}

// 监听标题和标签输入的改变，不再触发自动保存
// 但记录草稿以便关闭标签时自动保存
watch([title, tagInput], () => {
  updateDraft()
})

async function copyNoteId() {
  const id = editNoteId.value
  if (!id) return
  let port = 8899
  try {
    const config = await invoke<{ port: number; host: string }>('get_note_share_config')
    port = config.port
  } catch { /* 兜底 */ }
  const localIp = await getLanIp()
  const host = localIp || '127.0.0.1'
  const url = `http://${host}:${port}/api/notes/${id}/html`
  try {
    await navigator.clipboard.writeText(url)
    const st = useStatusStore()
    st.pushMessage('分享链接已复制到剪贴板', 'success')
  } catch {
    const st = useStatusStore()
    st.pushMessage('❌ 复制失败', 'error')
  }
}

async function getLanIp(): Promise<string | null> {
  const fromWebRtc = await new Promise<string | null>((resolve) => {
    try {
      const pc = new RTCPeerConnection({ iceServers: [] })
      pc.createDataChannel('')
      pc.createOffer().then(offer => pc.setLocalDescription(offer))
      pc.onicecandidate = (e) => {
        if (!e.candidate) { pc.close(); resolve(null); return }
        const match = e.candidate.candidate.match(/(\d+\.\d+\.\d+\.\d+)/)
        if (match) {
          const ip = match[1]
          if (ip !== '127.0.0.1' && !ip.startsWith('169.254.')) {
            pc.close()
            resolve(ip)
          }
        }
      }
      setTimeout(() => { pc.close(); resolve(null) }, 2000)
    } catch { resolve(null) }
  })
  if (fromWebRtc) return fromWebRtc

  try {
    const ip = await invoke<string>('get_local_ip')
    if (ip !== '127.0.0.1' && !ip.startsWith('169.254.')) return ip
  } catch { /* ignore */ }

  return null
}

function doCut() { closeCtx(); document.execCommand('cut') }
function doCopy() { closeCtx(); document.execCommand('copy') }
function doPaste() { closeCtx(); editorRef.value?.chain().focus().run(); document.execCommand('paste') }

onBeforeUnmount(() => {
  if (saveTimer) clearTimeout(saveTimer)
  // 退出前保存最新内容（仅在开启偏好时）
  if (store.getSaveOnClose()) doSave(false)
  if (editorRef.value) {
    editorRef.value.destroy()
  }
})
</script>

<template>
  <div class="editor" :class="{ editing: !!existingNote }">
    <!-- Main Workspace Layout -->
    <div class="layout">
      <div class="layout-body">
        <!-- Editor Content Area (大滚动容器，内含标题和编辑器作为整体滚动) -->
        <section class="layout-content">
          <input v-model="title" class="editor-title-input" placeholder="请在这里输入标题" />
          <YiiEditor ref="yiiEditor" class="editor-yiieditor" :content="''" :extensions="editorExtensions"
            :dark-mode="isDark" :show-main-menu="false" :main-menu="defaultMenu" :show-bubble-menu="true"
            :show-floating-menu="true" :show-side-menu="true" :bubble-menu="bubbleMenu" :floating-menu="floatingMenu"
            page-view="full" locale="zh-CN" @contextmenu="handleEditorContextMenu" @create="onEditorCreate"
            @update="onEditorUpdate" />
        </section>
      </div>
    </div>

    <!-- Slash Triggered Dialog Menu -->
    <SlashCommandMenu :visible="slashMenuVisible" :editorRect="slashMenuRect" @close="hideSlashMenu"
      @insertLink="slashInsertLink" @command="slashExecuteCommand" />

    <!-- Footer Bar: 重新融合标签、字数和操作按钮 -->
    <div class="editor-footer">
      <div class="tag-area">
        <input v-model="tagInput" class="tag-input" placeholder="添加标签, 用逗号分隔" />
        <div v-if="inlineTags.length" class="inline-tags">
          <span class="inline-tags-hint">提取的内联标签:</span>
          <span v-for="t in inlineTags.filter(x => !tags.includes(x))" :key="t" class="inline-tag-chip"
            @click="mergeInlineTag(t)" title="点击添加到标签栏">#{{ t }} +</span>
        </div>
      </div>

      <div class="footer-actions">
        <span v-if="lastSaved" class="saved-hint">已保存 {{ lastSaved }}</span>
        <span v-if="saving" class="saving-hint">保存中...</span>
        <button class="footer-btn pri" @click="doSave(false)" title="手动保存当前修改 (Ctrl+S)">
          保存
        </button>
        <button class="footer-btn" @click="copyNoteId" title="复制局域网分享链接">
          分享
        </button>

        <button class="footer-btn" @click="doSave(true)" title="生成一个版本历史快照 (Ctrl+Enter)">
          快照
        </button>
        <button class="footer-btn" @click="store.openVersionHistory()" title="查看历史修改版本">
          历史
        </button>
        <span class="char-count">{{ editorRef?.getText().length ?? 0 }} 字</span>
      </div>
    </div>
  </div>

  <!-- Custom Context Menu (Right Click) -->
  <Teleport to="body">
    <div v-if="ctxShow" class="ctx-overlay" @click="closeCtx" @contextmenu.prevent="closeCtx">
      <div class="ctx-menu" :style="ctxStyle" @click.stop>
        <div class="ctx-item" @click="execCmd('undo')" title="撤销"><span class="ctx-icon">↩</span> 撤销</div>
        <div class="ctx-item" @click="execCmd('redo')" title="重做"><span class="ctx-icon">↪</span> 重做</div>

        <div class="ctx-divider"></div>
        <div class="ctx-item" @click="doCut"><span class="ctx-icon">✂</span> 剪切</div>
        <div class="ctx-item" @click="doCopy"><span class="ctx-icon"><svg viewBox="0 0 1024 1024" width="12" height="12"
              fill="currentColor">
              <path
                d="M281.6 32h374.464a70.4 70.4 0 0 1 49.792 20.608l201.536 201.536a70.4 70.4 0 0 1 20.608 49.792V806.4a57.6 57.6 0 0 1-57.6 57.6H281.6a57.6 57.6 0 0 1-57.6-57.6V89.6a57.6 57.6 0 0 1 57.6-57.6z m19.2 768h550.4a12.8 12.8 0 0 0 12.8-12.8V303.936a6.4 6.4 0 0 0-0.512-2.496l-1.344-2.048-201.536-201.536a6.4 6.4 0 0 0-4.48-1.856H300.8a12.8 12.8 0 0 0-12.8 12.8v678.4c0 7.04 5.76 12.8 12.8 12.8z" />
              <path
                d="M256 160v64H172.8a12.8 12.8 0 0 0-12.8 12.8v678.4c0 7.04 5.76 12.8 12.8 12.8h550.4a12.8 12.8 0 0 0 12.8-12.8V832h64v102.4a57.6 57.6 0 0 1-57.6 57.6H153.6a57.6 57.6 0 0 1-57.6-57.6V217.6a57.6 57.6 0 0 1 57.6-57.6H256zM672 64v211.2c0 7.04 5.76 12.8 12.8 12.8H896v64h-243.2a44.8 44.8 0 0 1-44.8-44.8V64h64z" />
            </svg></span> 复制</div>
        <div class="ctx-item" @click="doPaste"><span class="ctx-icon">📌</span> 粘贴</div>

        <!-- Condition 1: Command Send to Terminal -->
        <template v-if="hasSelection || codeBlockText !== null">
          <div class="ctx-divider"></div>
          <div class="ctx-title">{{ hasSelection ? '发送选中文本至终端' : '运行当前代码块' }}</div>
          <div v-for="t in runningTerminals" :key="t.processId" class="ctx-item" @click="execInTerminal(t.processId)">
            <span class="ctx-icon">▸</span>
            {{ t.name }}
          </div>
          <div class="ctx-item" @click="createAndExec">
            <span class="ctx-icon plus">＋</span>
            新建终端运行
          </div>
        </template>

        <!-- Condition 2: Quick Insert Commands (when nothing is selected and cursor is not in code block) -->
        <template v-else>
          <div class="ctx-divider"></div>
          <div class="ctx-title">快捷插入</div>
          <div class="ctx-item" @click="execCmd('table')"><span class="ctx-icon">田</span> 插入表格</div>
          <div class="ctx-item" @click="execCmd('codeBlock')"><span class="ctx-icon">💻</span> 插入代码块</div>
          <div class="ctx-item" @click="execCmd('horizontalRule')"><span class="ctx-icon">➖</span> 插入分割线</div>
          <div class="ctx-item" @click="execCmd('taskList')"><span class="ctx-icon">☑</span> 插入任务列表</div>
          <div class="ctx-item" @click="rightClickInsertLink"><span class="ctx-icon">🔗</span> 链接笔记</div>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<style scoped lang="scss">
.editor {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 0px;
  /* 移除外围内边距，使设计更加极简 */
  overflow: hidden;
  background: var(--jc-bg-app, #1e1e1e);

  &.editing {
    background: var(--jc-bg-app, #1e1e1e);
  }
}

/* 重新排版的顶部Header工具栏 */
.editor-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 8px 16px;
  background: var(--jc-bg-elevated, #252526);
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  flex-shrink: 0;
}

.title-input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--jc-text-highlight, #ffffff);
  font-size: 16px;
  font-weight: 600;
  outline: none;

  &::placeholder {
    color: var(--jc-text-secondary, #858585);
    font-weight: 400;
  }
}

.editor-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.saved-hint {
  font-size: 11px;
  color: var(--jc-color-success, #4ec9b0);
  margin-right: 4px;
}

.saving-hint {
  font-size: 11px;
  color: var(--jc-color-warning, #d7ba7d);
  margin-right: 4px;
}

/* 按钮样式精美化 */
.act-btn {
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-default, #3e3e42);
  color: var(--jc-text-primary, #cccccc);
  font-size: 11px;
  font-weight: 500;
  padding: 4px 10px;
  cursor: pointer;
  border-radius: 4px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  white-space: nowrap;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

  &:hover {
    background: var(--jc-bg-hover, #2a2d2e);
    color: var(--jc-text-highlight, #ffffff);
    border-color: var(--jc-color-accent, #8a58ff);
  }

  &.pri {
    color: #ffffff;
    background: var(--jc-color-accent, #8a58ff);
    border-color: transparent;

    &:hover {
      background: color-mix(in srgb, var(--jc-color-accent, #8a58ff) 80%, white);
      box-shadow: 0 0 8px rgba(138, 88, 255, 0.4);
    }
  }

  .btn-icon {
    font-size: 12px;
  }
}

/* 主工作区布局：由编辑器和TOC并排，高度自适应，禁止外部双滚动 */
.layout {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--jc-bg-app, #1e1e1e);
}

.layout-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: row;
  overflow: hidden;
}

.layout-content {
  flex: 1;
  min-height: 0;
  padding-left: 50px;
  overflow-y: auto;
  /* 编辑器和标题共享此大滚动条 */
  background: var(--jc-bg-app, #1e1e1e);
}

.editor-title-input {
  display: block;
  width: calc(100% - 64px);
  max-width: 900px;
  margin: 32px auto 0 32px;
  border: none;
  background: transparent;
  color: var(--jc-text-highlight, #ffffff);
  font-size: 28px;
  font-weight: 700;
  outline: none;
  padding: 0;

  &::placeholder {
    color: var(--jc-text-secondary, #858585);
    opacity: 0.4;
  }
}

.editor-yiieditor {
  height: auto;
  display: flex !important;
  flex-direction: column !important;
}

/* 精准拉伸编辑器外层布局骨架，使用 > 限制仅穿透至最外侧骨架层，彻底防止侵入 ProseMirror 内部以防污染代码块、折叠列表、表格、卡片等所有组件 */
:deep(.editor-yiieditor),
:deep(.editor-yiieditor > div),
:deep(.editor-yiieditor > div > div),
:deep(.editor-yiieditor > div > div > div),
:deep(.editor-yiieditor > div > div > div > div) {
  display: flex !important;
  flex-direction: column !important;
  flex: 1 !important;
  height: auto !important;
  min-height: 100% !important;
}

/* 确保绝对定位的工具条和气泡菜单在匹配中被重置以排除干扰，保持其本身体积与定位 */
:deep(.editor-yiieditor .bubble-menu),
:deep(.editor-yiieditor .floating-menu),
:deep(.editor-yiieditor [style*="position: absolute"]),
:deep(.editor-yiieditor [style*="position: fixed"]) {
  display: block !important;
  height: auto !important;
  min-height: 0 !important;
}

/* TipTap / ProseMirror 自定义内容排版 */
:deep(.ProseMirror) {
  min-height: 100%;

  outline: none;
  color: var(--jc-text-primary, #cccccc);
  font-size: 14px;
  line-height: 1.8;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;

  h1 {
    font-size: 1.85em;
    font-weight: 700;
    color: var(--jc-text-highlight, #ffffff);
    margin: 0.8em 0 0.4em;
    border-bottom: 1px solid var(--jc-border-default, #3e3e42);
    padding-bottom: 0.25em;
  }

  h2 {
    font-size: 1.5em;
    font-weight: 600;
    color: var(--jc-text-highlight, #ffffff);
    margin: 0.7em 0 0.3em;
  }

  h3 {
    font-size: 1.25em;
    font-weight: 600;
    color: var(--jc-text-highlight, #ffffff);
    margin: 0.6em 0 0.25em;
  }

  h4 {
    font-size: 1.1em;
    font-weight: 600;
    color: var(--jc-text-highlight, #ffffff);
    margin: 0.5em 0 0.2em;
  }

  p {
    margin: 0.6em 0;
  }

  strong {
    font-weight: 700;
    color: var(--jc-text-highlight, #ffffff);
  }

  em {
    font-style: italic;
  }

  u {
    text-decoration: underline;
  }

  s {
    text-decoration: line-through;
  }

  code {
    background: var(--jc-bg-input, #3c3c3c);
    color: var(--jc-color-success, #4ec9b0);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: 'Cascadia Code', Consolas, monospace;
    font-size: 0.9em;
  }

  mark {
    background: rgba(215, 186, 125, 0.25);
    color: var(--jc-text-highlight, #ffffff);
    padding: 2px 4px;
    border-radius: 3px;
  }

  blockquote {
    border-left: 4px solid var(--jc-color-accent, #8a58ff);
    margin: 0.8em 0;
    padding: 0.4em 1.2em;
    background: var(--jc-bg-elevated, #252526);
    color: var(--jc-text-secondary, #858585);
    border-radius: 0 4px 4px 0;
  }

  pre {
    background: var(--jc-bg-elevated, #252526);
    border: 1px solid var(--jc-border-default, #3e3e42);
    border-radius: 6px;
    padding: 16px;
    margin: 1em 0;
    overflow-x: auto;

    code {
      background: none;
      padding: 0;
      color: var(--jc-text-primary, #cccccc);
      font-family: 'Cascadia Code', Consolas, monospace;
      font-size: 0.88em;
    }
  }

  hr {
    border: none;
    border-top: 1px solid var(--jc-border-default, #3e3e42);
    margin: 1.5em 0;
  }

  /* 精美表格及单元格排版，自适应宽度 */
  table {
    border-collapse: collapse;
    margin: 1.5em 0;
    width: auto !important;
    max-width: 100%;
    table-layout: fixed;

    td,
    th {
      min-width: 100px;
      border: 1px solid var(--jc-border-default, #3e3e42);
      padding: 8px 12px;
      vertical-align: top;
      background: var(--jc-bg-elevated, #252526);
      color: var(--jc-text-primary, #cccccc);
    }

    th {
      font-weight: 600;
      background: var(--jc-bg-selected, #37373d);
      color: var(--jc-text-highlight, #ffffff);
    }
  }

  a {
    color: var(--jc-color-accent, #8a58ff);
    text-decoration: underline;
    cursor: pointer;

    &:hover {
      color: color-mix(in srgb, var(--jc-color-accent, #8a58ff) 80%, white);
    }
  }

  /* 笔记内链 span 样式设计 */
  .note-link {
    color: #58a6ff;
    font-weight: 500;
    text-decoration: none;
    border-bottom: 1px dashed #58a6ff;
    padding: 1px 3px;
    margin: 0 2px;
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.2s ease;

    &:hover {
      background: rgba(88, 166, 255, 0.15);
      border-bottom-style: solid;
      color: #79c0ff;
    }

    &::before {
      content: '📄 ';
      font-size: 0.9em;
    }
  }

  img {
    max-width: 100%;
    height: auto;
    border-radius: 6px;
    margin: 1em 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  table {
    border-collapse: collapse;
    margin: 1em 0;
    width: 100%;

    th,
    td {
      border: 1px solid var(--jc-border-strong, #555555);
      padding: 8px 12px;
      min-width: 60px;
    }

    th {
      background: var(--jc-bg-elevated, #252526);
      font-weight: 600;
      color: var(--jc-text-highlight, #ffffff);
    }

    td {
      background: var(--jc-bg-app, #1e1e1e);
    }
  }
}

/* Footer & 标签设计 */
.editor-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 8px 16px;
  background: var(--jc-bg-elevated, #252526);
  border-top: 1px solid var(--jc-border-default, #3e3e42);
  flex-shrink: 0;
}

.tag-area {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 12px;
}

.tag-input {
  width: 180px;
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: 4px;
  color: var(--jc-text-primary, #cccccc);
  font-size: 11px;
  outline: none;
  padding: 4px 8px;
  transition: border-color 0.2s;

  &:focus {
    border-color: var(--jc-color-accent, #8a58ff);
  }

  &::placeholder {
    color: var(--jc-text-secondary, #858585);
    opacity: 0.6;
  }
}

.inline-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}

.inline-tags-hint {
  font-size: 10px;
  color: var(--jc-text-secondary, #858585);
}

.inline-tag-chip {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 4px;
  background: rgba(78, 201, 176, 0.12);
  color: #4ec9b0;
  cursor: pointer;
  transition: background 0.15s, transform 0.1s;

  &:hover {
    background: rgba(78, 201, 176, 0.22);
    transform: translateY(-1px);
  }
}

.char-count {
  font-size: 11px;
  color: var(--jc-text-secondary, #858585);
  white-space: nowrap;
}

/* 屏蔽 YIITAP 内部类 editor-content 的 padding，防止边缘挤压 */
:deep(.yii-editor-content),
:deep(.editor-content) {
  padding: 0 !important;
}

/* 合并至 Footer 的动作按钮样式 */
.footer-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.footer-btn {
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-default, #3e3e42);
  color: var(--jc-text-primary, #cccccc);
  font-size: 10px;
  padding: 2px 8px;
  cursor: pointer;
  border-radius: 4px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
  transition: all 0.15s ease;

  &:hover {
    background: var(--jc-bg-hover, #2a2d2e);
    color: var(--jc-text-highlight, #ffffff);
    border-color: var(--jc-color-accent, #8a58ff);
  }

  &.pri {
    color: #ffffff;
    background: var(--jc-color-accent, #8a58ff);
    border-color: transparent;

    &:hover {
      background: color-mix(in srgb, var(--jc-color-accent, #8a58ff) 85%, white);
    }
  }
}

/* Custom Context Menu Layout */
.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
}

.ctx-menu {
  position: fixed;
  background: var(--jc-bg-elevated, #252526);
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: 6px;
  padding: 4px 0;
  min-width: 170px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
}

.ctx-title {
  padding: 4px 12px;
  font-size: 10px;
  font-weight: 600;
  color: var(--jc-text-secondary, #858585);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  margin-bottom: 2px;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  font-size: 12px;
  cursor: pointer;
  color: var(--jc-text-primary, #cccccc);
  white-space: nowrap;
  transition: all 0.1s;

  &:hover {
    background: var(--jc-bg-selected, #37373d);
    color: var(--jc-color-accent, #8a58ff);
  }
}

.ctx-divider {
  height: 1px;
  background: var(--jc-border-default, #3e3e42);
  margin: 4px 8px;
}

.ctx-icon {
  color: var(--jc-color-success, #4ec9b0);
  font-weight: bold;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;

  &.plus {
    color: var(--jc-color-accent, #8a58ff);
  }
}

/* 强制代码块及绘图工具栏水平一行排开，使用最高特异性选择器覆盖层级 column 限制 */
:deep(.editor-yiieditor .o-code-block-view .code-block-toolbar) {
  display: flex !important;
  flex-direction: row !important;
  flex-wrap: nowrap !important;
  align-items: center !important;
  justify-content: space-between !important;
}

:deep(.editor-yiieditor .o-code-block-view .code-block-toolbar .wrap) {
  display: flex !important;
  flex-direction: row !important;
  flex-wrap: nowrap !important;
  align-items: center !important;
}
</style>
