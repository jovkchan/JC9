import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useStatusStore } from '@/stores/status'
import type { NoteGroup, Note, EditorState, NoteVersion } from '@/types/notes'

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
  const showSearchPanel = ref(false)

  /** 关闭标签时自动保存（直接从 localStorage 读，确保实时生效） */
  function getSaveOnClose(): boolean {
    return localStorage.getItem('notes-save-on-close') === 'true'
  }

  /** 编辑器未保存的内容草稿（用于关闭时自动保存） */
  interface NoteDraft {
    title: string
    content: string
    tags: string[]
  }
  const noteContentDrafts = ref<Record<string, NoteDraft>>({})

  function updateNoteDraft(id: string, draft: NoteDraft) {
    noteContentDrafts.value[id] = draft
  }

  function clearNoteDraft(id: string) {
    delete noteContentDrafts.value[id]
  }

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

  /** 递归获取某分组及其所有后代分组的 ID 集合 */
  function getDescendantGroupIds(groupId: string): Set<string> {
    const ids = new Set<string>([groupId])
    for (const g of groups.value) {
      if (g.parentId && ids.has(g.parentId)) {
        ids.add(g.id)
      }
    }
    // 多轮扫描直到稳定（处理深层嵌套）
    let prev = 0
    while (ids.size > prev) {
      prev = ids.size
      for (const g of groups.value) {
        if (g.parentId && ids.has(g.parentId)) ids.add(g.id)
      }
    }
    return ids
  }

  /** 获取分组面包屑路径（从根到目标） */
  function getGroupPath(groupId: string): NoteGroup[] {
    const path: NoteGroup[] = []
    let current = groups.value.find(g => g.id === groupId)
    while (current) {
      path.unshift(current)
      current = current.parentId ? groups.value.find(g => g.id === current!.parentId) : undefined
    }
    return path
  }

  async function loadGroups() {
    // 重试 3 次，应对 Tauri state 尚未 manage 的时序问题
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        groups.value = await invoke<NoteGroup[]>('get_note_groups')
        return
      } catch (e) {
        if (attempt < 2) {
          await new Promise(r => setTimeout(r, 200 * (attempt + 1)))
        } else {
          console.error(e)
          useStatusStore().pushMessage(`加载分组失败: ${e}`, 'error')
        }
      }
    }
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

    // 3. 分组过滤（包含所有子分组笔记）
    if (selectedGroupId.value && listTab.value === 'notes') {
      const descendantIds = getDescendantGroupIds(selectedGroupId.value)
      list = list.filter(n => n.groupId && descendantIds.has(n.groupId))
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

  async function saveNote(note: Note, createVersion = false) {
    // 自动提取正文中的行内标签并进行合并与去重
    const inlineTags = extractTagsFromContent(note.content)
    const allTags = Array.from(new Set([...note.tags, ...inlineTags]))
    note.tags = allTags

    note.updatedAt = new Date().toISOString()
    try {
      await invoke('save_note', { note, createVersion })
      const existing = notes.value.findIndex(n => n.id === note.id)
      if (existing >= 0) {
        // 原地修改保持对象引用不变，避免触发父组件 watcher 导致光标跳转
        Object.assign(notes.value[existing], note)
      } else {
        notes.value.unshift({ ...note })
      }
      status.setNoteCount(notes.value.filter(n => !n.isDeleted).length)
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

  async function closeNoteTab(id: string) {
    // 关闭标签时自动保存草稿（若开启偏好）
    if (getSaveOnClose() && noteContentDrafts.value[id]) {
      const draft = noteContentDrafts.value[id]
      const existing = notes.value.find(n => n.id === id)
      if (existing) {
        const note: Note = {
          ...existing,
          title: draft.title || '无标题',
          content: draft.content,
          tags: draft.tags,
          updatedAt: new Date().toISOString(),
        }
        try {
          await saveNote(note, false)
        } catch (e) {
          console.error('关闭标签自动保存失败:', e)
        }
      } else {
        // 新建但未保存的笔记
        try {
          const note = await createNote({
            title: draft.title || '无标题',
            content: draft.content,
            format: 'markdown',
            tags: draft.tags,
            groupId: selectedGroupId.value,
            visibility: 'PRIVATE',
          })
          if (note) {
            // 更新 tab id 为实际 id
            const tabIdx = noteTabs.value.findIndex(t => t.id === id)
            if (tabIdx >= 0) noteTabs.value[tabIdx].id = note.id
            // 更新后续操作使用新 id
            id = note.id
          }
        } catch (e) {
          console.error('关闭标签自动创建保存失败:', e)
        }
      }
    }
    clearNoteDraft(id)

    const idx = noteTabs.value.findIndex(t => t.id === id)
    if (idx >= 0) noteTabs.value.splice(idx, 1)
    if (activeNoteTabId.value === id) {
      activeNoteTabId.value = noteTabs.value.length > 0 ? noteTabs.value[noteTabs.value.length - 1].id : null
    }
  }

  // ── 版本历史 ──

  const noteVersions = ref<NoteVersion[]>([])
  const showVersionHistory = ref(false)
  const previewVersionId = ref<string | null>(null)
  const previewVersionData = ref<NoteVersion | null>(null)

  async function loadNoteVersions(noteId: string) {
    try {
      noteVersions.value = await invoke<NoteVersion[]>('get_note_versions', { noteId })
    } catch (e) {
      console.error('加载版本历史失败:', e)
    }
  }

  function openVersionHistory() {
    if (!activeNoteTabId.value) return
    showVersionHistory.value = true
    loadNoteVersions(activeNoteTabId.value)
  }

  function closeVersionHistory() {
    showVersionHistory.value = false
    previewVersionId.value = null
    previewVersionData.value = null
  }

  async function previewNoteVersion(versionId: string) {
    previewVersionId.value = versionId
    try {
      previewVersionData.value = await invoke<NoteVersion>('get_note_version_by_id', { versionId })
    } catch (e) {
      status.pushMessage(`加载版本失败: ${e}`, 'error')
    }
  }

  async function restoreNoteVersion(noteId: string, versionId: string) {
    try {
      const restored = await invoke<Note>('restore_note_version', { noteId, versionId })
      // 更新本地 notes 列表
      const idx = notes.value.findIndex(n => n.id === noteId)
      if (idx >= 0) {
        notes.value[idx] = restored
      }
      // 更新标签页标题
      const tab = noteTabs.value.find(t => t.id === noteId)
      if (tab) tab.title = restored.title || '无标题'
      // 关闭版本面板
      closeVersionHistory()
      status.pushMessage(`已恢复到 v${restored.version}`, 'success')
    } catch (e) {
      status.pushMessage(`恢复失败: ${e}`, 'error')
    }
  }

  // ── 后端变更监听（MCP / 其他窗口修改数据时自动刷新）──
  let listenersInitialized = false

  function setupChangeListeners() {
    if (listenersInitialized) return
    listenersInitialized = true

    listen<{ action: string; id: string }>('notes:changed', async (event) => {
      const { action, id } = event.payload

      // 重新加载笔记列表和分组
      await Promise.all([loadAllNotes(), loadGroups()])

      // 更新已打开标签页的标题
      for (const tab of noteTabs.value) {
        const note = notes.value.find(n => n.id === tab.id)
        if (note) {
          tab.title = note.title || '无标题'
        }
      }

      // 如果笔记被删除且当前在打开的标签中，给出提示
      if ((action === 'deleted' || action === 'soft-deleted') && id) {
        const openTab = noteTabs.value.find(t => t.id === id)
        if (openTab) {
          status.pushMessage(`笔记"${openTab.title}"已被外部删除`, 'warn')
        }
      }

      // 笔记被更新时给出轻提示
      if (action === 'updated' && id) {
        const note = notes.value.find(n => n.id === id)
        if (note) {
          status.pushMessage(`笔记"${note.title}"已更新`, 'info')
        }
      }
    })
  }

  // 初始化监听
  setupChangeListeners()

  return {
    // Filter States
    listTab, searchQuery, filterDate, selectedTag, showSearchPanel,
    // Groups
    groups, selectedGroupId, groupTree,
    loadGroups, addGroup, updateGroup, removeGroup, getGroupPath,
    // Notes
    notes, selectedNoteId, filteredNotes, pinnedNotes, unpinnedNotes,
    loadNotes, loadAllNotes, saveNote, createNote,
    removeNote, restoreNote, permanentlyDeleteNote,
    searchNotes, togglePin, toggleArchive, copyContent,
    // Editor tabs
    noteTabs, activeNoteTabId, openNoteTab, closeNoteTab,
    // Draft (close-to-save)
    getSaveOnClose, noteContentDrafts, updateNoteDraft, clearNoteDraft,
    // Version history
    noteVersions, showVersionHistory, previewVersionId, previewVersionData,
    loadNoteVersions, openVersionHistory, closeVersionHistory,
    previewNoteVersion, restoreNoteVersion,
  }
})
