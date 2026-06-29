<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useProjectStore } from '@/stores/project'
import ActivityCalendar from './ActivityCalendar.vue'
import type { Note } from '@/types/notes'

const store = useNotesStore()
const projectStore = useProjectStore()

const newGroupName = ref('')
const showingNewGroup = ref(false)
const expandedGroups = ref<Set<string>>(new Set())
const editingGroupId = ref('')
const editingGroupName = ref('')
const calendarOpen = ref(false)
const showTrash = ref(false)

const searchQuery = computed({
  get: () => store.searchQuery,
  set: (val) => store.searchQuery = val
})
const filterDate = computed({
  get: () => store.filterDate,
  set: (val) => store.filterDate = val
})
const listTab = computed({
  get: () => store.listTab,
  set: (val) => store.listTab = val
})
const selectedTag = computed({
  get: () => store.selectedTag,
  set: (val) => store.selectedTag = val
})

// ── Context menu ──
const ctxShow = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxNote = ref<Note | null>(null)

// ── Rename ──
const renameShow = ref(false)
const renameValue = ref('')
const renameNoteId = ref('')

// ── Delete confirm ──
const deleteConfirmShow = ref(false)
const deleteNoteId = ref('')
const deleteNoteTitle = ref('')

onMounted(async () => {
  await store.loadGroups()
  await store.loadAllNotes()
})

function toggleGroup(id: string) {
  if (expandedGroups.value.has(id)) { expandedGroups.value.delete(id) }
  else { expandedGroups.value.add(id); store.selectedGroupId = id; store.loadNotes(id) }
}

async function handleAddGroup() {
  const n = newGroupName.value.trim()
  if (!n) return
  await store.addGroup(n)
  newGroupName.value = ''; showingNewGroup.value = false
}

async function confirmRenameGroup() {
  const g = store.groups.find(x => x.id === editingGroupId.value)
  if (!g) return
  const n = editingGroupName.value.trim()
  if (!n) return
  g.name = n; await store.updateGroup(g); editingGroupId.value = ''
}

function handleNewNote() { store.openNoteTab('') }
function handleOpenNote(noteId: string) { store.selectedNoteId = noteId; store.openNoteTab(noteId) }
function handleSelectDate(date: string | null) { filterDate.value = date }



function openCtx(e: MouseEvent, note: Note) {
  e.preventDefault(); e.stopPropagation()
  ctxPos.value = { x: e.clientX, y: e.clientY }; ctxNote.value = note; ctxShow.value = true
}
function closeCtx() { ctxShow.value = false }

// 重置过滤
function clearAllFilters() {
  store.selectedGroupId = null
  store.selectedTag = null
  store.filterDate = null
  store.searchQuery = ''
}

function openAiAssistant() {
  projectStore.openTool('ai-helper', 'AI 助手')
}

function ctxEdit() {
  if (ctxNote.value) { store.selectedNoteId = ctxNote.value.id; store.openNoteTab(ctxNote.value.id) }
  closeCtx()
}
function ctxCopy() { if (ctxNote.value) store.copyContent(ctxNote.value.id); closeCtx() }
function ctxTogglePin() { if (ctxNote.value) store.togglePin(ctxNote.value.id); closeCtx() }
function ctxToggleArchive() { if (ctxNote.value) store.toggleArchive(ctxNote.value.id); closeCtx() }
function ctxRename() {
  if (ctxNote.value) { renameNoteId.value = ctxNote.value.id; renameValue.value = ctxNote.value.title; renameShow.value = true }
  closeCtx()
}
async function confirmRenameNote() {
  const n = renameValue.value.trim()
  if (!n) { renameShow.value = false; return }
  const note = store.notes.find(x => x.id === renameNoteId.value)
  if (note) { note.title = n; note.updatedAt = new Date().toISOString(); await store.saveNote(note) }
  renameShow.value = false
}
function ctxDelete() {
  if (ctxNote.value) { deleteNoteId.value = ctxNote.value.id; deleteNoteTitle.value = ctxNote.value.title || '无标题'; deleteConfirmShow.value = true }
  closeCtx()
}
async function confirmDelete() { await store.removeNote(deleteNoteId.value); deleteConfirmShow.value = false }

// ── Computed ──

const calendarNotes = computed(() => store.notes.map(n => ({ createdAt: n.createdAt, updatedAt: n.updatedAt })))

const rootGroups = computed(() => store.groups.filter(g => !g.parentId))

// Apply search/filter to store notes for display in groups
function applyFilters(notes: Note[]) {
  let list = notes.filter(n => !n.isDeleted)
  if (listTab.value === 'starred') list = list.filter(n => n.isPinned)
  if (listTab.value !== 'archived') list = list.filter(n => !n.isArchived)
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter(n => n.title.toLowerCase().includes(q) || n.content.toLowerCase().includes(q) || n.tags.some(t => t.toLowerCase().includes(q)))
  }
  if (filterDate.value) list = list.filter(n => (n.updatedAt || n.createdAt).slice(0, 10) === filterDate.value)
  return list
}

const starredNotes = computed(() => store.notes.filter(n => !n.isDeleted && n.isPinned))
const trashedNotes = computed(() => store.notes.filter(n => n.isDeleted))

const tagCloud = computed(() => {
  const map: Record<string, number> = {}
  for (const n of store.notes) { if (!n.isDeleted) for (const t of n.tags) map[t] = (map[t] || 0) + 1 }
  return Object.entries(map).sort((a, b) => b[1] - a[1])
})

function selectTag(tag: string) { selectedTag.value = selectedTag.value === tag ? null : tag }

onMounted(() => document.addEventListener('click', closeCtx))
</script>

<template>
  <aside class="note-sidebar">
    <!-- Header: title + calendar toggle -->
    <div class="ns-header">
      <span class="ns-title">笔记</span>
      <button class="ns-cal-btn" @click="calendarOpen = !calendarOpen" :title="calendarOpen ? '收起日历' : '展开日历'">
        {{ calendarOpen ? '▴' : '▾' }}
      </button>
    </div>

    <ActivityCalendar v-if="calendarOpen" :notes="calendarNotes" @select-date="handleSelectDate" />

    <!-- Search + actions row (replaces old 全部笔记 header) -->
    <div class="ns-search-row">
      <span class="ns-search-hint-text">按 Ctrl + F 全局搜索</span>
      <button class="ns-btn icon" title="新建笔记" @click="handleNewNote">
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8"
          stroke-linecap="round">
          <path d="M8 3v10M3 8h10" />
        </svg>
      </button>
      <button class="ns-btn icon" :class="{ on: showTrash }" @click="showTrash = !showTrash" title="回收站">
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.3"
          stroke-linecap="round" stroke-linejoin="round">
          <path
            d="M2 4h12M5.33 4V2.67a1.33 1.33 0 0 1 1.34-1.34h2.66a1.33 1.33 0 0 1 1.34 1.34V4m2 0v9.33a1.33 1.33 0 0 1-1.34 1.34H4.67a1.33 1.33 0 0 1-1.34-1.34V4h9.34z" />
          <path d="M6.67 7.33v4M9.33 7.33v4" />
        </svg>
      </button>
    </div>

    <!-- Tabs -->
    <div class="ns-tabs">
      <div :class="['ns-tab', { on: listTab === 'notes' }]" @click="listTab = 'notes'">笔记</div>
      <div :class="['ns-tab', { on: listTab === 'tags' }]" @click="listTab = 'tags'">标签</div>
      <div :class="['ns-tab', { on: listTab === 'starred' }]" @click="listTab = 'starred'">星标</div>
      <div :class="['ns-tab', { on: listTab === 'archived' }]" @click="listTab = 'archived'">归档</div>
    </div>

    <div v-if="filterDate" class="ns-filter-hint">日期: {{ filterDate }} <button class="ns-filter-clr"
        @click="filterDate = null">✕</button></div>

    <!-- Tag cloud -->
    <div v-if="listTab === 'tags'" class="ns-tags">
      <div v-for="[tag, count] in tagCloud" :key="tag" class="ns-tag" :class="{ sel: selectedTag === tag }"
        @click="selectTag(tag)">
        <span class="ns-tag-name"># {{ tag }}</span><span class="ns-tag-count">{{ count }}</span>
      </div>
      <div v-if="tagCloud.length === 0" class="ns-empty">暂无标签</div>
    </div>

    <!-- Trash view -->
    <div v-if="showTrash" class="ns-tree">
      <div class="ns-trash-header">
        <span>回收站 ({{ trashedNotes.length }})</span>
        <button class="ns-back-btn" @click="showTrash = false">← 返回</button>
      </div>
      <div v-if="trashedNotes.length === 0" class="ns-empty">回收站为空</div>
      <div v-for="n in trashedNotes" :key="n.id" class="ns-item trash-item">
        <span class="ns-label">{{ n.title || '无标题' }}</span>
        <button class="ns-restore-btn" @click.stop="store.restoreNote(n.id)" title="恢复">恢复</button>
        <button class="ns-del" style="display:inline"
          @click.stop="deleteNoteId = n.id; deleteNoteTitle = n.title || '无标题'; deleteConfirmShow = true"
          title="永久删除">✕</button>
      </div>
    </div>

    <!-- Notes list (with groups) -->
    <div v-show="listTab === 'notes'" class="ns-tree">

      <template v-for="g in rootGroups" :key="g.id">
        <div class="ns-item group" :class="{ sel: store.selectedGroupId === g.id }">
          <template v-if="editingGroupId === g.id">
            <input class="ns-edit-input" v-model="editingGroupName" @keyup.enter="confirmRenameGroup"
              @keyup.escape="editingGroupId = ''" @blur="confirmRenameGroup" @click.stop autofocus />
          </template>
          <template v-else>
            <span class="ns-arrow" @click="toggleGroup(g.id)">{{ expandedGroups.has(g.id) ? '▾' : '▸' }}</span>
            <span class="ns-label" @click="toggleGroup(g.id)">{{ g.name }}</span>
            <span class="ns-count">{{applyFilters(store.notes.filter(n => n.groupId === g.id)).length}}</span>
            <button class="ns-del" @click.stop="store.removeGroup(g.id)" title="删除组">✕</button>
          </template>
        </div>
        <template v-if="expandedGroups.has(g.id)">
          <div v-for="n in applyFilters(store.notes.filter(x => x.groupId === g.id))" :key="n.id" class="ns-item note"
            :class="{ sel: store.selectedNoteId === n.id, pinned: n.isPinned }" @click="handleOpenNote(n.id)"
            @contextmenu="openCtx($event, n)" :title="n.title">
            <span class="ns-dot"></span><span class="ns-label">{{ n.title || '无标题' }}</span>
          </div>
        </template>
      </template>

      <div class="ns-item add-group" @click="showingNewGroup = true">
        <template v-if="showingNewGroup">
          <input v-model="newGroupName" class="ns-add-input" placeholder="组名..." @keyup.enter="handleAddGroup"
            @keyup.escape="showingNewGroup = false" @blur="handleAddGroup" @click.stop autofocus />
        </template>
        <template v-else><span class="ns-label">+ 新建笔记组</span></template>
      </div>
    </div>

    <!-- Starred list (flat, no groups) -->
    <div v-show="listTab === 'starred'" class="ns-tree">
      <div v-if="starredNotes.length === 0" class="ns-empty">暂无星标笔记</div>
      <div v-for="n in starredNotes" :key="n.id" class="ns-item note" :class="{ sel: store.selectedNoteId === n.id }"
        @click="handleOpenNote(n.id)" @contextmenu="openCtx($event, n)" :title="n.title">
        <span class="ns-dot"></span><span class="ns-label">{{ n.title || '无标题' }}</span>
      </div>
    </div>

    <!-- Archived list (flat) -->
    <div v-show="listTab === 'archived'" class="ns-tree">
      <div v-if="store.notes.filter(n => !n.isDeleted && n.isArchived).length === 0" class="ns-empty">暂无归档笔记</div>
      <div v-for="n in store.notes.filter(x => !x.isDeleted && x.isArchived)" :key="n.id" class="ns-item note"
        :class="{ sel: store.selectedNoteId === n.id }" @click="handleOpenNote(n.id)" @contextmenu="openCtx($event, n)"
        :title="n.title">
        <span class="ns-dot"></span><span class="ns-label">{{ n.title || '无标题' }}</span>
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div v-if="ctxShow" class="ctx" :style="{ left: ctxPos.x + 'px', top: ctxPos.y + 'px' }" @click.stop>
        <div class="ci" @click="ctxEdit">编辑</div>
        <div class="ci" @click="ctxCopy">复制内容</div>
        <div class="ci" @click="ctxTogglePin">{{ ctxNote?.isPinned ? '★ 取消星标' : '☆ 设为星标' }}</div>
        <div class="ci" @click="ctxToggleArchive">{{ ctxNote?.isArchived ? '取消归档' : '归档' }}</div>
        <div class="ci" @click="ctxRename">重命名</div>
        <div class="ci" style="color:var(--jc-color-error)" @click="ctxDelete">删除</div>
      </div>
    </Teleport>

    <!-- Rename modal -->
    <Teleport to="body">
      <div v-if="renameShow" class="mbg" @click.self="renameShow = false">
        <div class="mw" style="min-width:320px">
          <div class="mt">重命名笔记</div>
          <div class="mb">
            <div class="fld"><label>新名称</label><input v-model="renameValue" @keyup.enter="confirmRenameNote"
                autofocus />
            </div>
            <div class="acts"><button class="btn" @click="renameShow = false">取消</button><button class="btn pri"
                @click="confirmRenameNote">保存</button></div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Delete confirm -->
    <Teleport to="body">
      <div v-if="deleteConfirmShow" class="mbg" @click.self="deleteConfirmShow = false">
        <div class="mw" style="min-width:320px">
          <div class="mt">删除笔记</div>
          <div class="mb">
            <p style="color:var(--jc-text-secondary);font-size:12px">确定要删除笔记「{{ deleteNoteTitle }}」吗？<br /><span
                style="font-size:11px">删除后可在回收站恢复</span></p>
            <div class="acts"><button class="btn" @click="deleteConfirmShow = false">取消</button><button class="btn pri"
                style="background:var(--jc-color-error)" @click="confirmDelete">删除</button></div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Sidebar Footer -->
    <div class="ns-footer-bar">
      <button class="ns-footer-btn" @click="store.showSettings = true" title="设置笔记">设置</button>
      <button class="ns-footer-btn" @click="openAiAssistant" title="AI 助理">AI 助理</button>
      <button class="ns-footer-btn" @click="clearAllFilters" title="重置过滤">重置</button>
    </div>
  </aside>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;

.note-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.ns-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 10px;
  border-bottom: 1px solid var(--jc-border-default);
}

.ns-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}

.ns-cal-btn {
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  font-size: 12px;
  cursor: pointer;
  padding: 0 4px;

  &:hover {
    color: var(--jc-color-accent)
  }
}

.ns-search-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--jc-border-default);
}

.ns-search-hint-text {
  flex: 1;
  font-size: 10px;
  color: var(--jc-text-secondary);
  opacity: 0.6;
  user-select: none;
}

.ns-btn {
  @include btn-base;
  font-size: 11px;
  padding: 2px 10px;
}

.ns-btn.icon {
  padding: 3px 6px;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  color: var(--jc-text-secondary);

  &:hover {
    color: var(--jc-text-primary)
  }

  &.on {
    color: var(--jc-color-accent)
  }
}

.ns-trash-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 10px;
  font-size: 11px;
  color: var(--jc-text-secondary);
  border-bottom: 1px solid var(--jc-border-default);
}

.ns-back-btn {
  background: none;
  border: none;
  color: var(--jc-color-accent);
  font-size: 11px;
  cursor: pointer;

  &:hover {
    text-decoration: underline
  }
}

.ns-restore-btn {
  background: none;
  border: none;
  color: var(--jc-color-success);
  font-size: 11px;
  cursor: pointer;
  padding: 0 6px;

  &:hover {
    text-decoration: underline
  }
}

.trash-item {
  color: var(--jc-text-secondary);
  opacity: 0.7;
}

.ns-cal-toggle {
  display: none;
}

.ns-search {
  padding: 6px 10px;

  input {
    width: 100%;
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-primary);
    font-size: 11px;
    padding: 4px 8px;
    outline: none;
    border-radius: 3px;

    &:focus {
      border-color: var(--jc-color-accent)
    }

    &::placeholder {
      color: var(--jc-text-secondary)
    }
  }
}

.ns-tabs {
  display: flex;
  border-bottom: 1px solid var(--jc-border-default);
}

.ns-tab {
  flex: 1;
  text-align: center;
  padding: 5px 0;
  font-size: 11px;
  cursor: pointer;
  color: var(--jc-text-secondary);
  border-bottom: 2px solid transparent;

  &:hover {
    color: var(--jc-text-primary)
  }

  &.on {
    color: var(--jc-color-accent);
    border-bottom-color: var(--jc-color-accent)
  }
}

.ns-filter-hint {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 10px;
  font-size: 11px;
  color: var(--jc-color-success);
  background: #006d3222;
}

.ns-filter-clr {
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  font-size: 12px;
  cursor: pointer;

  &:hover {
    color: var(--jc-color-error)
  }
}

.ns-tags {
  flex: 1;
  overflow-y: auto;
  padding: 6px 10px;
}

.ns-tag {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 3px 6px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 3px;
  color: var(--jc-text-secondary);

  &:hover {
    background: var(--jc-bg-hover)
  }

  &.sel {
    color: var(--jc-color-success);
    background: var(--jc-bg-selected)
  }
}

.ns-tag-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ns-tag-count {
  font-size: 9px;
  background: var(--jc-bg-btn);
  padding: 0 4px;
  border-radius: 3px;
}

.ns-empty {
  padding: 20px;
  text-align: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
}

.ns-tree {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.ns-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  font-size: 12px;
  cursor: pointer;
  color: var(--jc-text-secondary);

  &:hover {
    background: var(--jc-bg-hover)
  }

  &.sel {
    background: var(--jc-bg-selected);
    color: var(--jc-text-primary)
  }

  .ns-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--jc-text-secondary);
    flex-shrink: 0
  }

  &.pinned .ns-dot {
    background: var(--jc-color-favorite)
  }
}

.ns-arrow {
  font-size: 9px;
  color: var(--jc-text-secondary);
  width: 12px;
  flex-shrink: 0
}

.ns-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap
}

.ns-count {
  font-size: 9px;
  color: var(--jc-text-secondary);
  background: var(--jc-bg-btn);
  padding: 0 4px;
  border-radius: 3px;
  flex-shrink: 0
}

.ns-del {
  display: none;
  background: none;
  color: var(--jc-text-secondary);
  font-size: 11px;
  padding: 0 4px;
  cursor: pointer;

  &:hover {
    color: var(--jc-color-error)
  }
}

.ns-item:hover .ns-del {
  display: inline
}

.ns-edit-input {
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-color-accent);
  color: var(--jc-text-primary);
  padding: 1px 4px;
  font-size: 12px;
  outline: none
}

.ns-add-input {
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-color-accent);
  color: var(--jc-text-primary);
  padding: 2px 6px;
  font-size: 12px;
  outline: none
}

.add-group {
  color: var(--jc-text-secondary);
  border-top: 1px solid var(--jc-border-default);
  margin-top: 4px;
  padding-top: 8px;

  &:hover {
    color: var(--jc-color-success)
  }
}

.ctx {
  @include ctx-menu;
  min-width: 130px
}

.ci {
  @include ctx-item
}

.mbg {
  position: fixed;
  inset: 0;
  background: var(--jc-bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000
}

.mw {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  min-width: 400px;
  box-shadow: var(--jc-shadow-modal)
}

.mt {
  background: var(--jc-bg-panel);
  padding: 10px 16px;
  font-size: 14px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  border-bottom: 1px solid var(--jc-border-default)
}

.mb {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px
}

.fld {
  display: flex;
  flex-direction: column;
  gap: 4px;

  label {
    font-size: 11px;
    color: var(--jc-text-secondary)
  }

  input {
    @include input-base
  }
}

.acts {
  display: flex;
  gap: 6px;
  justify-content: flex-end
}

.btn {
  @include btn-base
}

.btn.pri {
  @include btn-primary
}

.ns-footer-bar {
  display: flex;
  background: var(--jc-bg-panel);
  border-top: 1px solid var(--jc-border-default);
  padding: 6px;
  gap: 4px;
  flex-shrink: 0;
}

.ns-footer-btn {
  flex: 1;
  background: transparent;
  border: 1px solid var(--jc-border-default);
  color: var(--jc-text-secondary);
  font-size: 10px;
  padding: 4px 0;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  white-space: nowrap;

  &:hover {
    background: var(--jc-bg-hover);
    color: var(--jc-text-primary);
    border-color: var(--jc-border-strong);
  }
}
</style>
