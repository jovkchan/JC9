<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue'
import { useNotesStore } from '@/stores/notes'
import type { Note } from '@/types/notes'
import { useEditor, EditorContent } from '@tiptap/vue-3'
import { Markdown } from '@tiptap/markdown'
import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import Link from '@tiptap/extension-link'
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

const lowlight = createLowlight(common)

const store = useNotesStore()

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

let saveTimer: ReturnType<typeof setTimeout> | null = null

const tags = ref<string[]>([])
function syncTags() {
  tags.value = tagInput.value
    .split(/[,，]/)
    .map(t => t.trim())
    .filter(Boolean)
}
syncTags()

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
    Markdown.configure({
      html: true,
      link: true,
      tightLists: false,
    }),
    Underline,
    Highlight.configure({ multicolor: true }),
    Link.configure({
      openOnClick: false,
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
      placeholder: '开始写点什么...',
      emptyEditorClass: 'is-editor-empty',
    }),
  ],
  editorProps: {
    attributes: {
      class: 'jc9-tiptap-editor',
    },
    clipboardTextSerializer: (slice) => {
      // 只序列化选中的部分，而非整篇文档
      try {
        const json = slice.toJSON()
        const md = editor.value?.markdown?.serialize(json) ?? ''
        return md
      } catch { return '' }
    },
  },
  onUpdate: () => {
    scheduleSave()
  },
})

// Expose getMarkdown for parent to use
function getMarkdown(): string {
  return editor.value?.getMarkdown() ?? title.value
}

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

async function doSave() {
  const md = editor.value?.getMarkdown() ?? ''
  if (!title.value.trim() && !md.trim()) return
  saving.value = true
  syncTags()

  if (editNoteId.value) {
    const existing = store.notes.find(n => n.id === editNoteId.value)
    if (!existing) { saving.value = false; return }
    const note: Note = {
      ...existing,
      title: title.value,
      content: md,
      format: 'markdown',
      tags: tags.value,
      updatedAt: new Date().toISOString(),
    }
    await store.saveNote(note)
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
  lastSaved.value = new Date().toLocaleTimeString()
  saving.value = false
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    doSave()
  }
}

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
        <button v-if="existingNote" class="cancel-btn" @click="emit('cancel')">✕</button>
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
        <button class="tb-btn" @click="execCmd('undo')" title="撤销">↩</button>
        <button class="tb-btn" @click="execCmd('redo')" title="重做">↪</button>
      </div>
    </div>

    <!-- TipTap WYSIWYG content area -->
    <div class="tiptap-wrapper">
      <EditorContent :editor="editor" />
    </div>

    <div class="editor-footer">
      <input
        v-model="tagInput"
        class="tag-input"
        placeholder="标签, 用逗号分隔"
      />
      <span class="char-count">{{ editor?.getText().length ?? 0 }} 字</span>
    </div>
  </div>
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
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.tag-input {
  flex: 1;
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

.char-count {
  font-size: 10px;
  color: var(--jc-text-secondary);
  white-space: nowrap;
}
</style>
