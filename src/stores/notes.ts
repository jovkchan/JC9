import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useStatusStore } from '@/stores/status'
import type { NoteGroup, Note, EditorState } from '@/types/notes'

function genId(): string {
  return crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

export const useNotesStore = defineStore('notes', () => {
  const status = useStatusStore()

  // ── Filter States ──
  const listTab = ref<'notes' | 'tags' | 'starred' | 'archived'>('notes')
  const searchQuery = ref('')
  const filterDate = ref<string | null>(null)
  const selectedTag = ref<string | null>(null)
  const showSettings = ref(false)
  const showSearchPanel = ref(false)

  // ── Groups ──
  const groups = ref<NoteGroup[]>([])
  const selectedGroupId = ref<string | null>(null)

  const groupTree = computed(() => {
    const roots = groups.value.filter(g => !g.parentId)
    return roots.map(r => ({ ...r, children: getChildren(r.id) }))
  })

  function getChildren(parentId: string): (NoteGroup & { children: NoteGroup[] })[] {
    return groups.value
      .filter(g => g.parentId === parentId)
      .map(g => ({ ...g, children: getChildren(g.id) }))
      .sort((a, b) => a.sortOrder - b.sortOrder)
  }

  async function loadGroups() {
    try {
      groups.value = await invoke<NoteGroup[]>('get_note_groups')
    } catch (e) { console.error(e) }
  }

  async function addGroup(name: string, parentId: string | null = null) {
    const now = new Date().toISOString()
    const group: NoteGroup = {
      id: genId(),
      name,
      parentId,
      sortOrder: groups.value.length,
      createdAt: now,
      updatedAt: now,
    }
    try {
      await invoke('save_note_group', { group })
      groups.value.push(group)
      status.pushMessage(`笔记组「${name}」已创建`, 'success')
    } catch (e) {
      status.pushMessage(`创建笔记组失败: ${e}`, 'error')
    }
  }

  async function updateGroup(group: NoteGroup) {
    group.updatedAt = new Date().toISOString()
    try {
      await invoke('save_note_group', { group })
      const i = groups.value.findIndex(g => g.id === group.id)
      if (i !== -1) Object.assign(groups.value[i], group)
    } catch (e) { console.error(e) }
  }

  async function removeGroup(id: string): Promise<boolean> {
    const g = groups.value.find(x => x.id === id)
    if (!g) return false
    const hasNotes = notes.value.filter(n => n.groupId === id && !n.isDeleted).length > 0
    if (hasNotes) {
      status.pushMessage(`笔记组「${g.name}」下有笔记，无法删除`, 'warn')
      return false
    }
    try {
      await invoke('delete_note_group', { id })
      groups.value = groups.value.filter(g => g.id !== id)
      if (selectedGroupId.value === id) selectedGroupId.value = null
      status.pushMessage(`笔记组「${g.name}」已删除`, 'success')
      return true
    } catch (e) {
      status.pushMessage(`删除笔记组失败: ${e}`, 'error')
      return false
    }
  }

  // ── Notes ──
  const notes = ref<Note[]>([])
  const selectedNoteId = ref<string | null>(null)

  // 联合过滤逻辑
  const filteredNotes = computed(() => {
    let list = notes.value.filter(n => !n.isDeleted)

    // 1. 星标过滤
    if (listTab.value === 'starred') {
      list = list.filter(n => n.isPinned)
    }

    // 2. 归档过滤（默认不显示已归档，归档 Tab 除外）
    if (listTab.value === 'archived') {
      list = list.filter(n => n.isArchived)
    } else {
      list = list.filter(n => !n.isArchived)
    }

    // 3. 分组过滤
    if (selectedGroupId.value && listTab.value === 'notes') {
      list = list.filter(n => n.groupId === selectedGroupId.value)
    }

    // 4. 标签过滤 (标签Tab中选中的标签)
    if (selectedTag.value && listTab.value === 'tags') {
      list = list.filter(n => n.tags.includes(selectedTag.value!))
    }

    // 5. 权重评分 AND 过滤算法（支持空格分隔多词、is:pinned/is:archived/tag: 等语法）
    if (searchQuery.value.trim()) {
      const rawQuery = searchQuery.value.trim().toLowerCase()
      const queryWords = rawQuery.split(/\s+/).filter(Boolean)

      const matchedWithScores = list.map(note => {
        let score = 0
        let matchesAll = true

        for (const word of queryWords) {
          // 判定特殊指令
          if (word === 'is:pinned' || word === 'is:starred') {
            if (!note.isPinned) matchesAll = false
            continue
          }
          if (word === 'is:archived') {
            if (!note.isArchived) matchesAll = false
            continue
          }
          if (word.startsWith('tag:')) {
            const tagVal = word.slice(4)
            if (!note.tags.some(t => t.toLowerCase() === tagVal)) matchesAll = false
            continue
          }

          // 模糊匹配逻辑
          const titleIdx = note.title.toLowerCase().indexOf(word)
          const contentIdx = note.content.toLowerCase().indexOf(word)
          const hasTag = note.tags.some(t => t.toLowerCase().includes(word))

          if (titleIdx === -1 && contentIdx === -1 && !hasTag) {
            matchesAll = false
            break // 必须全部词都匹配才满足
          }

          // 加权评分
          if (titleIdx !== -1) {
            score += 100 // 标题优先匹配，赋予极高权值
            if (titleIdx === 0) score += 50 // 标题首词前缀匹配额外加权
          }
          if (hasTag) {
            score += 30 // 标签匹配次之
          }
          if (contentIdx !== -1) {
            score += 10 // 正文匹配分值最低
          }
        }

        return { note, score, matchesAll }
      })

      // 仅保留满足全部检索词的笔记，并按得分从高到低排序（若分值相同，按更新时间倒序）
      list = matchedWithScores
        .filter(x => x.matchesAll)
        .sort((a, b) => {
          if (b.score !== a.score) return b.score - a.score
          const timeA = new Date(a.note.updatedAt || a.note.createdAt).getTime()
          const timeB = new Date(b.note.updatedAt || b.note.createdAt).getTime()
          return timeB - timeA
        })
        .map(x => x.note)
    }

    // 6. 日历日期过滤
    if (filterDate.value) {
      list = list.filter(n => (n.updatedAt || n.createdAt).slice(0, 10) === filterDate.value)
    }

    return list
  })

  const pinnedNotes = computed(() => filteredNotes.value.filter(n => n.isPinned))
  const unpinnedNotes = computed(() => filteredNotes.value.filter(n => !n.isPinned))

  async function loadNotes(groupId?: string | null) {
    try {
      notes.value = await invoke<Note[]>('get_notes', { groupId: groupId ?? undefined })
      status.setNoteCount(notes.value.filter(n => !n.isDeleted).length)
    } catch (e) { console.error(e) }
  }

  async function loadAllNotes() {
    try {
      notes.value = await invoke<Note[]>('get_notes', { groupId: undefined })
      status.setNoteCount(notes.value.filter(n => !n.isDeleted).length)
    } catch (e) { console.error(e) }
  }

  // 正则提取内容中的标签 (#标签名)
  function extractTagsFromContent(content: string): string[] {
    const matches = content.match(/#([^\s#]+)/g)
    if (!matches) return []
    return Array.from(new Set(matches.map(m => m.slice(1).trim()).filter(Boolean)))
  }

  async function saveNote(note: Note) {
    // 自动提取正文中的行内标签并进行合并与去重
    const inlineTags = extractTagsFromContent(note.content)
    const allTags = Array.from(new Set([...note.tags, ...inlineTags]))
    note.tags = allTags

    note.updatedAt = new Date().toISOString()
    try {
      await invoke('save_note', { note })
      const existing = notes.value.findIndex(n => n.id === note.id)
      if (existing >= 0) {
        notes.value[existing] = { ...note }
      } else {
        notes.value.unshift({ ...note })
      }
      status.setNoteCount(notes.value.filter(n => !n.isDeleted).length)
      status.pushMessage('笔记已保存', 'success')
    } catch (e) {
      status.pushMessage(`保存笔记失败: ${e}`, 'error')
    }
  }

  async function createNote(editorState: EditorState): Promise<Note | null> {
    const now = new Date().toISOString()
    // 提取正文里的行内标签
    const inlineTags = extractTagsFromContent(editorState.content)
    const allTags = Array.from(new Set([...editorState.tags, ...inlineTags]))

    const note: Note = {
      id: genId(),
      groupId: selectedGroupId.value,
      title: editorState.title,
      content: editorState.content,
      format: editorState.format,
      isPinned: false,
      tags: allTags,
      visibility: editorState.visibility,
      sortOrder: 0,
      version: 1,
      isDeleted: false,
      isArchived: false,
      createdAt: now,
      updatedAt: now,
    }
    try {
      await invoke('save_note', { note })
      notes.value.unshift({ ...note })
      status.setNoteCount(notes.value.filter(n => !n.isDeleted).length)
      status.pushMessage('笔记已创建', 'success')
      return note
    } catch (e) {
      status.pushMessage(`创建笔记失败: ${e}`, 'error')
      return null
    }
  }

  async function removeNote(id: string) {
    try {
      await invoke('delete_note', { id, permanent: false })
      const existing = notes.value.find(n => n.id === id)
      if (existing) {
        existing.isDeleted = true
        existing.updatedAt = new Date().toISOString()
      }
      status.setNoteCount(notes.value.filter(n => !n.isDeleted).length)
      status.pushMessage('笔记已移到回收站')
    } catch (e) {
      status.pushMessage(`删除笔记失败: ${e}`, 'error')
    }
  }

  async function restoreNote(id: string) {
    const note = notes.value.find(n => n.id === id)
    if (!note) return
    note.isDeleted = false
    note.updatedAt = new Date().toISOString()
    try {
      await invoke('save_note', { note })
      status.setNoteCount(notes.value.filter(n => !n.isDeleted).length)
      status.pushMessage('笔记已恢复')
    } catch (e) {
      status.pushMessage(`恢复笔记失败: ${e}`, 'error')
    }
  }

  async function permanentlyDeleteNote(id: string) {
    try {
      await invoke('delete_note', { id, permanent: true })
      notes.value = notes.value.filter(n => n.id !== id)
      status.setNoteCount(notes.value.filter(n => !n.isDeleted).length)
      status.pushMessage('笔记已永久删除')
    } catch (e) {
      status.pushMessage(`删除笔记失败: ${e}`, 'error')
    }
  }

  async function searchNotes(query: string): Promise<Note[]> {
    try {
      return await invoke<Note[]>('search_notes', { query })
    } catch (e) { console.error(e); return [] }
  }

  async function togglePin(id: string) {
    const note = notes.value.find(n => n.id === id)
    if (!note) return
    note.isPinned = !note.isPinned
    note.updatedAt = new Date().toISOString()
    try { await invoke('save_note', { note }) } catch (e) { console.error(e) }
  }

  async function toggleArchive(id: string) {
    const note = notes.value.find(n => n.id === id)
    if (!note) return
    note.isArchived = !note.isArchived
    note.updatedAt = new Date().toISOString()
    try {
      await invoke('save_note', { note })
      status.pushMessage(note.isArchived ? '笔记已归档' : '已取消归档', 'success')
    } catch (e) { console.error(e) }
  }

  function copyContent(id: string) {
    const note = notes.value.find(n => n.id === id)
    if (!note) return
    navigator.clipboard.writeText(note.content).then(() => {
      status.pushMessage('内容已复制', 'success')
    }).catch(() => {
      status.pushMessage('复制失败', 'error')
    })
  }

  // ── Editor Tabs ──

  interface NoteTab { id: string; title: string }

  const noteTabs = ref<NoteTab[]>([])
  const activeNoteTabId = ref<string | null>(null)

  function openNoteTab(noteId: string) {
    const existing = noteTabs.value.findIndex(t => t.id === noteId)
    if (existing >= 0) {
      activeNoteTabId.value = noteId
      return
    }
    const note = notes.value.find(n => n.id === noteId)
    noteTabs.value.push({ id: noteId, title: note?.title || '无标题' })
    activeNoteTabId.value = noteId
  }

  function closeNoteTab(id: string) {
    const idx = noteTabs.value.findIndex(t => t.id === id)
    if (idx >= 0) noteTabs.value.splice(idx, 1)
    if (activeNoteTabId.value === id) {
      activeNoteTabId.value = noteTabs.value.length > 0 ? noteTabs.value[noteTabs.value.length - 1].id : null
    }
  }

  return {
    // Filter States
    listTab, searchQuery, filterDate, selectedTag, showSettings, showSearchPanel,
    // Groups
    groups, selectedGroupId, groupTree,
    loadGroups, addGroup, updateGroup, removeGroup,
    // Notes
    notes, selectedNoteId, filteredNotes, pinnedNotes, unpinnedNotes,
    loadNotes, loadAllNotes, saveNote, createNote,
    removeNote, restoreNote, permanentlyDeleteNote,
    searchNotes, togglePin, toggleArchive, copyContent,
    // Editor tabs
    noteTabs, activeNoteTabId, openNoteTab, closeNoteTab,
  }
})
