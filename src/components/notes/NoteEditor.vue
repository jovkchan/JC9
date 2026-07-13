<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useExecInTerminal } from '@/composables/useExecInTerminal'
import { invoke } from '@tauri-apps/api/core'
import type { Note } from '@/types/notes'
import { useEditor, EditorContent } from '@tiptap/vue-3'
import { Extension } from '@tiptap/core'
import { Markdown } from '@tiptap/markdown'
import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import Link from '@tiptap/extension-link'
import { mergeAttributes } from '@tiptap/core'

// 自定义链接：jclink:// 渲染为 span（避免浏览器导航），其他正常渲染为 a
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
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import { Table } from '@tiptap/extension-table'
import { TableRow } from '@tiptap/extension-table-row'
import { TableCell } from '@tiptap/extension-table-cell'
import { TableHeader } from '@tiptap/extension-table-header'
import Underline from '@tiptap/extension-underline'
import Highlight from '@tiptap/extension-highlight'
import Placeholder from '@tiptap/extension-placeholder'
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight'
import { common, createLowlight } from 'lowlight'
import { Plugin, PluginKey } from 'prosemirror-state'
import SlashCommandMenu from './SlashCommandMenu.vue'

const lowlight = createLowlight(common)

// ── Slash Command State ──
const slashMenuVisible = ref(false)
const slashMenuRect = ref<{ top: number; left: number } | null>(null)
const slashFromPos = ref<number | null>(null)  // position of '/' in the doc

function showSlashMenu(view: { coordsAtPos: (pos: number) => { top: number; left: number; bottom: number; right: number } }, pos: number) {
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
  const ed = editor.value
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
  const ed = editor.value
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

onMounted(() => document.addEventListener('click', handleJclinkClick))
onBeforeUnmount(() => document.removeEventListener('click', handleJclinkClick))

// ProseMirror plugin: detect '/' at start of line
const slashPluginKey = new PluginKey('slash-command')
const slashPlugin = new Plugin({
  key: slashPluginKey,
  props: {
    handleTextInput(view, from, _to, text) {
      if (text !== '/') return false
      // Check if '/' is at start of line or after whitespace at start of line
      const $pos = view.state.doc.resolve(from)
      const nodeBefore = $pos.nodeBefore
      const isAtLineStart = $pos.parentOffset === 0
      const isAfterSpace = nodeBefore?.isText && nodeBefore.text?.endsWith(' ')
      const isAfterNewline = !nodeBefore
      if (!isAtLineStart && !isAfterSpace && !isAfterNewline) return false
      // Don't trigger inside code blocks
      if (view.state.selection.$head.parent.type.name === 'codeBlock') return false
      showSlashMenu(view, from)  // from 是 / 的插入位置
      return false // let the '/' be typed normally
    },
    handleKeyDown(_view, event) {
      if (!slashMenuVisible.value) return false
      if (event.key === 'Escape') {
        hideSlashMenu()
        return true
      }
      // Don't interfere - menu handles its own keyboard
      if (['ArrowUp', 'ArrowDown', 'Enter'].includes(event.key) && slashMenuVisible.value) {
        return true // absorb these keys, menu handles them
      }
      return false
    },
  },
})

const store = useNotesStore()

const { ctxShow, ctxStyle, runningTerminals, openCtx, closeCtx, execInTerminal, createAndExec } = useExecInTerminal()

function handleEditorContextMenu(e: MouseEvent) {
  const sel = editor.value?.state.selection
  if (sel && !sel.empty) {
    const text = editor.value?.state.doc.textBetween(sel.from, sel.to)
    if (text && text.trim()) {
      openCtx(e, text.trim())
      return
    }
  }
  // 无选中文本 → 弹出 / 命令菜单
  e.preventDefault()
  const view = editor.value?.view
  if (view) {
    const pos = view.posAtCoords({ left: e.clientX, top: e.clientY })
    if (pos) showSlashMenu(view, pos.pos)
  }
}

const props = defineProps<{
  existingNote?: Note | null
}>()

const emit = defineEmits<{
  saved: [note: Note]
  cancel: []
}>()

const title = ref(props.existingNote?.title ?? '')
const tagInput = ref(props.existingNote?.tags.join(', ') ?? '')
const saving = ref(false)
const lastSaved = ref('')
const editNoteId = ref<string | null>(props.existingNote?.id ?? null)
const activePopover = ref<string | null>(null)
/** 标记当前是否正在执行自身发起的保存，防止 watcher 回写编辑器内容导致光标跳转 */
let selfSaving = false

let saveTimer: ReturnType<typeof setTimeout> | null = null

// ── 响应外部笔记变更（MCP 等外部修改后自动刷新编辑器内容）──
watch(() => props.existingNote, (newNote) => {
  if (!newNote) return
  // 自身保存触发的引用变化，不需要回写（内容已在编辑器中）
  if (selfSaving) return
  // 仅在内容确实不同时更新（避免覆盖用户正在编辑的内容）
  if (newNote.title !== title.value) {
    title.value = newNote.title
  }
  if (editNoteId.value !== newNote.id) {
    editNoteId.value = newNote.id
  }
  // 同步标签
  const newTagsStr = newNote.tags.join(', ')
  if (newTagsStr !== tagInput.value) {
    tagInput.value = newTagsStr
    syncTags()
  }
  // 同步编辑器内容（仅在当前编辑器内容与传入内容不同时更新）
  if (editor.value) {
    const currentMd = editor.value.getMarkdown()
    if (newNote.content !== currentMd) {
      // 必须指定 contentType: 'markdown'，否则 setContent 默认按 HTML 解析
      editor.value.commands.setContent(newNote.content, { contentType: 'markdown' })
      lastSaved.value = ''
    }
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

// 实时提取正文中的 #标签（不包含在代码块内的）
const inlineTags = ref<string[]>([])
function extractInlineTags(md: string): string[] {
  // 移除代码块内容后提取 #tag
  const cleaned = md.replace(/```[\s\S]*?```/g, '').replace(/`[^`]+`/g, '')
  const matches = cleaned.match(/#([^\s#.,;:!?()（）\[\]{}]+)/g)
  if (!matches) return []
  return Array.from(new Set(matches.map(m => m.slice(1).trim()).filter(t => t.length >= 1 && t.length <= 40)))
}

// 每 500ms 扫描一次正文内联标签
let tagScanTimer: ReturnType<typeof setInterval> | null = null
function startTagScan() {
  tagScanTimer = setInterval(() => {
    const md = editor.value?.getMarkdown() ?? ''
    inlineTags.value = extractInlineTags(md)
  }, 500)
}
onBeforeUnmount(() => { if (tagScanTimer) clearInterval(tagScanTimer) })

// 合并内联标签到手动标签
function mergeInlineTag(tag: string) {
  if (!tags.value.includes(tag)) {
    tags.value.push(tag)
    tagInput.value = tags.value.join(', ')
  }
}

// 存时自动合并所有内联标签
function mergeAllInlineTags() {
  for (const t of inlineTags.value) {
    if (!tags.value.includes(t)) tags.value.push(t)
  }
  tagInput.value = tags.value.join(', ')
}

// ── 原生 Markdown → TipTap ──
// 用 contentType: 'markdown' 让 Markdown 扩展自行解析
// （包括 Underline 的 ++text++ 和 Highlight 的 ==text== 及嵌套）
const mdContent = props.existingNote?.content ?? ''

const editor = useEditor({
  content: mdContent,
  contentType: 'markdown',
  extensions: [
    StarterKit.configure({
      heading: { levels: [1, 2, 3, 4, 5, 6] },
      codeBlock: false,
      link: false,
      underline: false, // StarterKit 自带 underline，禁用后使用独立扩展
    }),
    Markdown,
    Underline,
    Highlight.configure({ multicolor: true }),
    CustomLink.configure({
      openOnClick: true,
      HTMLAttributes: { rel: 'noopener noreferrer' },
    }),
    Image.configure({ inline: false }),
    TaskList,
    TaskItem.configure({ nested: true }),
    Table.configure({ resizable: true }),
    TableRow,
    TableCell,
    TableHeader,
    CodeBlockLowlight.configure({ lowlight }),
    Placeholder.configure({
      placeholder: '开始写点什么...（输入 / 打开命令菜单）',
      emptyEditorClass: 'is-editor-empty',
    }),
    // Slash command extension
    Extension.create({
      name: 'slashCommand',
      addProseMirrorPlugins() { return [slashPlugin] },
    }),
  ],
  editorProps: {
    attributes: {
      class: 'jc9-tiptap-editor',
    },
    clipboardTextSerializer: (slice) => {
      // 用单 \n 作为块分隔符，避免粘贴时出现大量空行
      return slice.content.textBetween(0, slice.content.size, '\n')
    },
  },
  onUpdate: () => {
    scheduleSave()
  },
})

// 编辑器就绪后启动标签扫描
watch(editor, (ed) => {
  if (ed) startTagScan()
})



// ── Toolbar helpers ──
function execCmd(cmd: string, value?: string) {
  const ed = editor.value
  if (!ed) return
  switch (cmd) {
    case 'bold': ed.chain().focus().toggleBold().run(); break
    case 'italic': ed.chain().focus().toggleItalic().run(); break
    case 'underline': ed.chain().focus().toggleUnderline().run(); break
    case 'strike': ed.chain().focus().toggleStrike().run(); break
    case 'code': ed.chain().focus().toggleCode().run(); break
    case 'heading': ed.chain().focus().toggleHeading({ level: Number(value) as 1|2|3|4|5|6 }).run(); break
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

function isActive(name: string, attrs?: Record<string, string>): boolean {
  return editor.value?.isActive(name, attrs) ?? false
}

function getHeadingLevel(): number {
  for (let i = 1; i <= 6; i++) {
    if (editor.value?.isActive('heading', { level: i })) return i
  }
  return 0
}

// ── Save logic ──
function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(doSave, 500)
}

async function doSave(createVersion = false) {
  const md = editor.value?.getMarkdown() ?? ''
  if (!title.value.trim() && !md.trim()) return
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
        title: title.value,
        content: md,
        format: 'markdown',
        tags: tags.value,
        updatedAt: new Date().toISOString(),
      }
      await store.saveNote(note, createVersion)
      emit('saved', note)
    } else {
      const note = await store.createNote({
        title: title.value,
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

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    doSave(true) // 手动保存 → 创建版本快照
  }
}

async function copyNoteId() {
  const id = editNoteId.value
  if (!id) return
  let port = 8899
  try {
    const config = await invoke<{ port: number; host: string }>('get_note_share_config')
    port = config.port
  } catch { /* 使用默认端口 */ }
  // 始终尝试获取实际 LAN IP（用于局域网其他设备访问）
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

/** 获取本机局域网 IP（优先 WebRTC，兜底 Tauri 命令） */
async function getLanIp(): Promise<string | null> {
  // 方法1：通过 RTCPeerConnection 获取
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

  // 方法2：通过 Tauri 命令获取
  try {
    const ip = await invoke<string>('get_local_ip')
    if (ip !== '127.0.0.1' && !ip.startsWith('169.254.')) return ip
  } catch { /* ignore */ }

  return null
}

// ── 右键菜单：剪切/复制/粘贴 ──
function doCut() { closeCtx(); document.execCommand('cut') }
function doCopy() { closeCtx(); document.execCommand('copy') }
function doPaste() { closeCtx(); editor.value?.chain().focus().run(); document.execCommand('paste') }

watch([title, tagInput], () => scheduleSave())

onBeforeUnmount(() => {
  if (saveTimer) {
    clearTimeout(saveTimer)
    doSave()
  }
  editor.value?.destroy()
})
</script>

<template>
  <div class="editor" :class="{ editing: !!existingNote }">
    <div class="editor-bar">
      <input
        v-model="title"
        class="title-input"
        placeholder="笔记标题..."
        @keydown="handleKeydown"
      />
      <div class="editor-actions">
        <span v-if="lastSaved" class="saved-hint">已保存 {{ lastSaved }}</span>
        <span v-if="saving" class="saving-hint">保存中...</span>
      </div>
    </div>

    <!-- WYSIWYG Toolbar -->
    <div class="tiptap-toolbar">
      <div class="toolbar-group">
        <button class="tb-btn" :class="{ on: isActive('bold') }" @click="execCmd('bold')" title="加粗 Ctrl+B"><b>B</b></button>
        <button class="tb-btn" :class="{ on: isActive('italic') }" @click="execCmd('italic')" title="斜体 Ctrl+I"><i>I</i></button>
        <button class="tb-btn" :class="{ on: isActive('underline') }" @click="execCmd('underline')" title="下划线 Ctrl+U"><u>U</u></button>
        <button class="tb-btn" :class="{ on: isActive('strike') }" @click="execCmd('strike')" title="删除线"><s>S</s></button>
        <button class="tb-btn" :class="{ on: isActive('code') }" @click="execCmd('code')" title="行内代码">&lt;/&gt;</button>
        <button class="tb-btn" :class="{ on: isActive('highlight') }" @click="execCmd('highlight')" title="高亮">≡</button>
      </div>

      <div class="toolbar-group">
        <div class="tb-dropdown" @click="activePopover = activePopover === 'heading' ? null : 'heading'">
          <button class="tb-btn">H{{ getHeadingLevel() || '▼' }}</button>
          <div v-if="activePopover === 'heading'" class="tb-dropdown-menu">
            <button v-for="lvl in 6" :key="lvl" class="tb-dropdown-item" :class="{ on: getHeadingLevel() === lvl }" @click.stop="execCmd('heading', String(lvl))">H{{ lvl }}</button>
            <button class="tb-dropdown-item" :class="{ on: getHeadingLevel() === 0 }" @click.stop="execCmd('heading', '0')">正文</button>
          </div>
        </div>
      </div>

      <div class="toolbar-group">
        <button class="tb-btn" :class="{ on: isActive('bulletList') }" @click="execCmd('bulletList')" title="无序列表">•≡</button>
        <button class="tb-btn" :class="{ on: isActive('orderedList') }" @click="execCmd('orderedList')" title="有序列表">1.</button>
        <button class="tb-btn" :class="{ on: isActive('taskList') }" @click="execCmd('taskList')" title="任务列表">☑</button>
        <button class="tb-btn" :class="{ on: isActive('blockquote') }" @click="execCmd('blockquote')" title="引用">❝</button>
      </div>

      <div class="toolbar-group">
        <button class="tb-btn" :class="{ on: isActive('codeBlock') }" @click="execCmd('codeBlock')" title="代码块">&lt;&gt;</button>
        <button class="tb-btn" @click="execCmd('horizontalRule')" title="分割线">—</button>
        <button class="tb-btn" @click="execCmd('table')" title="插入表格">⊞</button>
        <div v-if="isActive('table')" class="tb-dropdown-inline">
          <button class="tb-btn" @click="execCmd('addRowAfter')" title="插入行">＋行</button>
          <button class="tb-btn" @click="execCmd('addColAfter')" title="插入列">＋列</button>
          <button class="tb-btn" @click="execCmd('deleteTable')" title="删除表格">✕</button>
        </div>
      </div>

      <div class="toolbar-group">
        <button class="tb-btn" @click="execCmd('link')" title="插入链接">🔗</button>
      </div>

      <div class="toolbar-group" style="margin-left:auto">
        <button class="tb-btn" @click="execCmd('undo')" title="撤销 Ctrl+Z">↩</button>
        <button class="tb-btn" @click="execCmd('redo')" title="重做 Ctrl+Shift+Z">↪</button>
      </div>
      <div class="toolbar-group">
        <button class="tb-btn" @click="copyNoteId" title="复制笔记 ID">🔗</button>
        <button class="tb-btn pri" @click="doSave(true)" title="保存并创建版本 Ctrl+Enter" style="font-weight:600">版本保存</button>
        <button class="tb-btn" @click="store.openVersionHistory()" title="版本历史">
          <svg t="1783910028647" viewBox="0 0 1024 1024" width="16" height="16" style="vertical-align:middle">
            <path d="M256 298.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333z" fill="currentColor"></path>
            <path d="M298.666667 489.173333V725.333333a85.333333 85.333333 0 0 0 85.333333 85.333334h256v85.333333H384a170.666667 170.666667 0 0 1-170.666667-170.666667V298.666667h85.333334v42.666666a85.333333 85.333333 0 0 0 85.333333 85.333334h256v85.333333H384a169.898667 169.898667 0 0 1-85.333333-22.826667z" fill="currentColor" opacity=".3"></path>
            <path d="M768 938.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333zM768 554.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333z" fill="currentColor"></path>
          </svg>
        </button>
      </div>
    </div>

    <!-- TipTap WYSIWYG content area -->
    <div class="tiptap-wrapper" @contextmenu="handleEditorContextMenu">
      <EditorContent :editor="editor" />
    </div>

    <!-- Slash Command Menu -->
    <SlashCommandMenu
      :visible="slashMenuVisible"
      :editorRect="slashMenuRect"
      @close="hideSlashMenu"
      @insertLink="slashInsertLink"
      @command="slashExecuteCommand"
    />

    <div class="editor-footer">
      <div class="tag-area">
        <input
          v-model="tagInput"
          class="tag-input"
          placeholder="标签, 用逗号分隔"
        />
        <div v-if="inlineTags.length" class="inline-tags">
          <span class="inline-tags-hint">正文内联:</span>
          <span
            v-for="t in inlineTags.filter(x => !tags.includes(x))" :key="t"
            class="inline-tag-chip"
            @click="mergeInlineTag(t)"
            title="点击添加到标签"
          >#{{ t }} +</span>
        </div>
      </div>
      <span class="char-count">{{ editor?.getText().length ?? 0 }} 字</span>
    </div>
  </div>

  <!-- 右键菜单 -->
  <Teleport to="body">
    <div v-if="ctxShow" class="ctx-overlay" @click="closeCtx" @contextmenu.prevent="closeCtx">
      <div class="ctx-menu" :style="ctxStyle" @click.stop>
        <div class="ctx-item" @click="execCmd('undo')" title="撤销"><span class="ctx-icon">↩</span> 撤销</div>
        <div class="ctx-item" @click="execCmd('redo')" title="重做"><span class="ctx-icon">↪</span> 重做</div>
        <div class="ctx-divider"></div>
        <div class="ctx-item" @click="doCut"><span class="ctx-icon">✂</span> 剪切</div>
        <div class="ctx-item" @click="doCopy"><span class="ctx-icon"><svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M281.6 32h374.464a70.4 70.4 0 0 1 49.792 20.608l201.536 201.536a70.4 70.4 0 0 1 20.608 49.792V806.4a57.6 57.6 0 0 1-57.6 57.6H281.6a57.6 57.6 0 0 1-57.6-57.6V89.6a57.6 57.6 0 0 1 57.6-57.6z m19.2 768h550.4a12.8 12.8 0 0 0 12.8-12.8V303.936a6.4 6.4 0 0 0-0.512-2.496l-1.344-2.048-201.536-201.536a6.4 6.4 0 0 0-4.48-1.856H300.8a12.8 12.8 0 0 0-12.8 12.8v678.4c0 7.04 5.76 12.8 12.8 12.8z"/><path d="M256 160v64H172.8a12.8 12.8 0 0 0-12.8 12.8v678.4c0 7.04 5.76 12.8 12.8 12.8h550.4a12.8 12.8 0 0 0 12.8-12.8V832h64v102.4a57.6 57.6 0 0 1-57.6 57.6H153.6a57.6 57.6 0 0 1-57.6-57.6V217.6a57.6 57.6 0 0 1 57.6-57.6H256zM672 64v211.2c0 7.04 5.76 12.8 12.8 12.8H896v64h-243.2a44.8 44.8 0 0 1-44.8-44.8V64h64z"/></svg></span> 复制</div>
        <div class="ctx-item" @click="doPaste"><span class="ctx-icon">📌</span> 粘贴</div>
        <div class="ctx-divider"></div>
        <div class="ctx-title">发送到终端</div>
        <div
          v-for="t in runningTerminals"
          :key="t.processId"
          class="ctx-item"
          @click="execInTerminal(t.processId)"
        >
          <span class="ctx-icon">▸</span>
          {{ t.name }}
        </div>
        <div class="ctx-item" @click="createAndExec">
          <span class="ctx-icon plus">＋</span>
          新建终端
        </div>
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
  padding: 10px;
  overflow: hidden;

  &.editing {
    background: var(--jc-bg-app);
  }
}

.editor-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.title-input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--jc-text-highlight);
  font-size: 15px;
  font-weight: 600;
  padding: 4px 0;
  outline: none;

  &::placeholder {
    color: var(--jc-text-secondary);
    font-weight: 400;
  }
}

.editor-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.saved-hint {
  font-size: 10px;
  color: var(--jc-color-success);
}

.saving-hint {
  font-size: 10px;
  color: var(--jc-color-warning);
}

.cancel-btn {
  background: none;
  color: var(--jc-text-secondary);
  border: none;
  font-size: 14px;
  cursor: pointer;
  padding: 0 4px;

  &:hover { color: var(--jc-color-error); }
}

// ── WYSIWYG Toolbar ──
.tiptap-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px;
  padding: 6px 8px;
  margin: 6px 0 0;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  flex-shrink: 0;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 1px;
  padding: 0 4px;
  border-right: 1px solid var(--jc-border-default);

  &:last-child {
    border-right: none;
  }
}

.tb-btn {
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  font-size: 12px;
  padding: 3px 6px;
  cursor: pointer;
  border-radius: 3px;
  font-family: inherit;
  white-space: nowrap;

  &:hover {
    background: var(--jc-bg-hover);
    color: var(--jc-text-highlight);
  }

  &.on {
    background: var(--jc-bg-selected);
    color: var(--jc-color-accent);
  }
}

.tb-dropdown {
  position: relative;
}

.tb-dropdown-menu {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 100;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  box-shadow: var(--jc-shadow-menu);
  min-width: 60px;
}

.tb-dropdown-item {
  display: block;
  width: 100%;
  background: none;
  border: none;
  color: var(--jc-text-primary);
  font-size: 12px;
  padding: 4px 12px;
  cursor: pointer;
  text-align: left;

  &:hover { background: var(--jc-bg-hover); }
  &.on { color: var(--jc-color-accent); font-weight: 600; }
}

.tb-dropdown-inline {
  display: inline-flex;
  gap: 1px;
}

// ── TipTap Content ──
.tiptap-wrapper {
  flex: 1;
  min-height: 0;
  margin: 4px 0;
  overflow-y: auto;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  background: var(--jc-bg-app);
}

// TipTap's ProseMirror content area
:deep(.ProseMirror) {
  min-height: 100%;
  padding: 16px;
  outline: none;
  color: var(--jc-text-primary);
  font-size: 14px;
  line-height: 1.7;
  font-family: 'Segoe UI', system-ui, sans-serif;

  // Headings
  h1 { font-size: 1.8em; font-weight: 700; color: var(--jc-text-highlight); margin: 0.6em 0 0.3em; border-bottom: 1px solid var(--jc-border-default); padding-bottom: 0.2em; }
  h2 { font-size: 1.5em; font-weight: 600; color: var(--jc-text-highlight); margin: 0.5em 0 0.25em; }
  h3 { font-size: 1.25em; font-weight: 600; color: var(--jc-text-highlight); margin: 0.4em 0 0.2em; }
  h4 { font-size: 1.1em; font-weight: 600; color: var(--jc-text-highlight); margin: 0.3em 0 0.15em; }
  h5, h6 { font-size: 1em; font-weight: 600; color: var(--jc-text-secondary); }

  // Inline
  p { margin: 0.4em 0; }
  strong { font-weight: 700; color: var(--jc-text-highlight); }
  em { font-style: italic; }
  u { text-decoration: underline; }
  s { text-decoration: line-through; }
  code {
    background: var(--jc-bg-input);
    color: var(--jc-color-success);
    padding: 2px 5px;
    border-radius: 3px;
    font-family: 'Cascadia Code', Consolas, monospace;
    font-size: 0.9em;
  }
  mark {
    background: color-mix(in srgb, var(--jc-color-favorite) 30%, transparent);
    color: var(--jc-text-primary);
    padding: 1px 3px;
    border-radius: 2px;
  }

  // Lists
  ul, ol { padding-left: 1.5em; margin: 0.3em 0; }
  li { margin: 0.15em 0; }
  ul li { list-style-type: disc; }
  ol li { list-style-type: decimal; }

  // Task list
  ul[data-type="taskList"] {
    padding-left: 0;
    list-style: none;
    li {
      display: flex;
      align-items: flex-start;
      gap: 6px;
      label { flex-shrink: 0; margin-top: 0.2em; }
      div { flex: 1; }
    }
  }

  // Blockquote
  blockquote {
    border-left: 3px solid var(--jc-color-accent);
    margin: 0.5em 0;
    padding: 0.3em 1em;
    background: var(--jc-bg-input);
    color: var(--jc-text-secondary);
    font-style: italic;
  }

  // Code block
  pre {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    border-radius: 4px;
    padding: 12px;
    margin: 0.5em 0;
    overflow-x: auto;
    code {
      background: none;
      padding: 0;
      color: var(--jc-text-primary);
      font-family: 'Cascadia Code', Consolas, monospace;
      font-size: 0.85em;
    }
  }

  // Horizontal rule
  hr {
    border: none;
    border-top: 1px solid var(--jc-border-default);
    margin: 1em 0;
  }

  // Links
  a {
    color: var(--jc-color-accent);
    text-decoration: underline;
    cursor: pointer;
    &:hover { color: var(--jc-color-accent-hover); }
  }
  // 笔记内部链接
  .note-link {
    color: #58a6ff;
    text-decoration: none;
    border-bottom: 1px dashed #58a6ff;
    padding: 0 1px;
    cursor: pointer;
    &:hover { background: rgba(88,166,255,0.1); border-bottom-style: solid; }
    &::before { content: '📝 '; font-size: 0.85em; }
  }

  // Images
  img {
    max-width: 100%;
    height: auto;
    border-radius: 4px;
    margin: 0.5em 0;
  }

  // Tables
  table {
    border-collapse: collapse;
    margin: 0.5em 0;
    width: 100%;
    overflow: visible;
    th, td {
      border: 1px solid var(--jc-border-strong);
      padding: 6px 10px;
      text-align: left;
      min-width: 60px;
      position: relative;
    }
    th {
      background: var(--jc-bg-elevated);
      font-weight: 600;
      color: var(--jc-text-highlight);
    }
    td { background: var(--jc-bg-app); }
  }

  // Table column resize handle
  .tableWrapper {
    overflow-x: auto;
  }
  .column-resize-handle {
    position: absolute;
    right: -2px;
    top: 0;
    bottom: 0;
    width: 4px;
    background: var(--jc-color-accent);
    pointer-events: none;
    z-index: 20;
    opacity: 0.6;
  }
  .resize-cursor {
    cursor: col-resize;
  }

  // Placeholder
  p.is-editor-empty:first-child::before {
    content: attr(data-placeholder);
    float: left;
    color: var(--jc-text-secondary);
    pointer-events: none;
    height: 0;
    opacity: 0.4;
  }

  // Selected node
  .ProseMirror-selectednode {
    outline: 2px solid var(--jc-color-accent);
    outline-offset: 2px;
  }

  // Table selection
  .selectedCell {
    background: var(--jc-bg-selected);
  }
}

.editor-footer {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 8px;
  flex-shrink: 0;
}

.tag-area {
  flex: 1;
  min-width: 0;
}

.tag-input {
  width: 100%;
  background: transparent;
  border: none;
  color: var(--jc-text-secondary);
  font-size: 11px;
  outline: none;
  padding: 2px 0;

  &::placeholder {
    color: var(--jc-text-secondary);
    opacity: 0.5;
  }
}

.inline-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  padding-top: 2px;
}

.inline-tags-hint {
  font-size: 9px;
  color: var(--jc-text-secondary);
  opacity: 0.6;
}

.inline-tag-chip {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  background: rgba(63, 185, 128, 0.12);
  color: #3fb950;
  cursor: pointer;
  transition: background .15s;
  &:hover { background: rgba(63, 185, 128, 0.25); }
}

.char-count {
  font-size: 10px;
  color: var(--jc-text-secondary);
  white-space: nowrap;
}

// 右键菜单
.ctx-overlay {
  position: fixed; inset: 0; z-index: 10000;
}
.ctx-menu {
  position: fixed;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  padding: 4px 0;
  min-width: 160px;
  box-shadow: 0 4px 16px rgba(0,0,0,.3);
}
.ctx-title {
  padding: 4px 12px;
  font-size: 10px;
  color: var(--jc-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--jc-border-default);
  margin-bottom: 2px;
}
.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  font-size: 12px;
  cursor: pointer;
  color: var(--jc-text-primary);
  white-space: nowrap;
  &:hover {
    background: var(--jc-bg-selected);
    color: var(--jc-color-accent);
  }
}
.ctx-divider {
  height: 1px;
  background: var(--jc-border-default);
  margin: 2px 8px;
}
.ctx-icon {
  color: var(--jc-color-success);
  font-weight: bold;
  &.plus { color: var(--jc-color-accent); }
}
</style>
