<script setup lang="ts">
import { ref, onMounted, computed, nextTick, reactive } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { invoke } from '@tauri-apps/api/core'
import ActivityCalendar from './ActivityCalendar.vue'
import type { Note, NoteGroup } from '@/types/notes'

const store = useNotesStore()

const newGroupName = ref('')
const showingNewGroup = ref(false)
const showingNewSubGroup = ref(false)
const newGroupParentId = ref<string | null>(null)
const expandedGroups = ref<Set<string>>(new Set())
const editingGroupId = ref('')
const editingGroupName = ref('')
const calendarOpen = ref(false)
const showTrash = ref(false)
const readNoteIds = reactive(new Set<string>(JSON.parse(localStorage.getItem('jc9_read_notes') || '[]')))

function markRead(noteId: string) {
  readNoteIds.add(noteId)
  localStorage.setItem('jc9_read_notes', JSON.stringify([...readNoteIds]))
}

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
const ctxBottom = ref(0)
const ctxNote = ref<Note | null>(null)
const ctxShowMove = ref(false)
const ctxUpward = ref(false)

// ── Group context menu ──
const groupCtxShow = ref(false)
const groupCtxPos = ref({ x: 0, y: 0 })
const groupCtxGroupId = ref('')
const groupCtxUpward = ref(false)
const groupCtxBottom = ref(0)

function openGroupCtx(e: MouseEvent, groupId: string) {
  e.preventDefault(); e.stopPropagation()
  const { y, upward, bottom } = smartPosY(e, 170)
  groupCtxPos.value = { x: e.clientX, y }
  groupCtxBottom.value = bottom
  groupCtxGroupId.value = groupId
  groupCtxUpward.value = upward
  groupCtxShow.value = true
}
function closeGroupCtx() { groupCtxShow.value = false }

function ctxNewSubGroup() {
  const gid = groupCtxGroupId.value
  closeGroupCtx()
  newGroupName.value = ''
  newGroupParentId.value = gid
  showingNewSubGroup.value = true
  nextTick(() => {
    const el = document.querySelector<HTMLInputElement>('.ns-add-sub-input')
    el?.focus()
  })
}

function ctxRenameGroup() {
  const g = store.groups.find(x => x.id === groupCtxGroupId.value)
  if (!g) return closeGroupCtx()
  editingGroupId.value = g.id
  editingGroupName.value = g.name
  closeGroupCtx()
  nextTick(() => {
    const el = document.querySelector<HTMLInputElement>('.ns-edit-input')
    el?.focus()
    el?.select()
  })
}

function ctxNewNoteInGroup() {
  store.selectedGroupId = groupCtxGroupId.value
  store.openNoteTab('')
  closeGroupCtx()
}

function ctxDelGroup() {
  store.removeGroup(groupCtxGroupId.value)
  closeGroupCtx()
}

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
  else { expandedGroups.value.add(id); store.selectedGroupId = id }
  // flatGroupTree 自动响应 expandedGroups 变化重新展平
}

async function handleAddGroup() {
  const n = newGroupName.value.trim()
  if (!n) return
  await store.addGroup(n, newGroupParentId.value)
  newGroupName.value = ''; showingNewGroup.value = false; showingNewSubGroup.value = false
  newGroupParentId.value = null
}

async function confirmRenameGroup() {
  const g = store.groups.find(x => x.id === editingGroupId.value)
  if (!g) return
  const n = editingGroupName.value.trim()
  if (!n) return
  g.name = n; await store.updateGroup(g); editingGroupId.value = ''
}

function handleNewNote() { store.openNoteTab('') }
function handleRefresh() { store.loadAllNotes(); store.loadGroups() }
function handleOpenNote(noteId: string) { store.selectedNoteId = noteId; store.openNoteTab(noteId); markRead(noteId) }
function handleSelectDate(date: string | null) { filterDate.value = date }

function smartPosY(e: MouseEvent, menuH: number): { y: number; upward: boolean; bottom: number } {
  const vh = window.innerHeight
  const below = vh - e.clientY
  const upward = below < menuH && e.clientY > menuH
  return { y: upward ? e.clientY - 4 : e.clientY, upward, bottom: vh - e.clientY }
}

function openCtx(e: MouseEvent, note: Note) {
  e.preventDefault(); e.stopPropagation()
  const { y, upward, bottom } = smartPosY(e, 220)
  ctxPos.value = { x: e.clientX, y }; ctxBottom.value = bottom
  ctxNote.value = note; ctxUpward.value = upward; ctxShow.value = true
}
function closeCtx() { ctxShow.value = false; ctxShowMove.value = false }




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
async function moveNoteToGroup(groupId: string | null) {
  if (!ctxNote.value) return
  try {
    await invoke('move_note', { noteId: ctxNote.value.id, groupId })
    await store.loadAllNotes()  // 加载全部笔记，而非单个分组
    closeCtx()
  } catch (e) { console.error(e) }
}
async function confirmDelete() { await store.removeNote(deleteNoteId.value); deleteConfirmShow.value = false }

// ── Computed ──

const calendarNotes = computed(() => store.notes.map(n => ({ createdAt: n.createdAt, updatedAt: n.updatedAt })))

// 递归展平分组树，返回 TreeItem[]，分组→子分组→笔记
type TreeItem = { kind: 'group'; group: NoteGroup; depth: number } | { kind: 'note'; note: Note; depth: number }

const flatGroupTree = computed(() => {
  const result: TreeItem[] = []
  function walk(parentId: string | null, depth: number) {
    const children = store.groups.filter(g => g.parentId === parentId).sort((a, b) => a.sortOrder - b.sortOrder)
    for (const g of children) {
      result.push({ kind: 'group', group: g, depth })
      if (expandedGroups.value.has(g.id)) {
        // 先子分组，后笔记
        walk(g.id, depth + 1)
        // 该分组的直属笔记放最后
        for (const n of getGroupNotes(g.id)) {
          result.push({ kind: 'note', note: n, depth: depth + 1 })
        }
      }
    }
  }
  walk(null, 0)
  return result
})

// 获取某分组下的笔记（不递归子分组）
function getGroupNotes(groupId: string) {
  return applyFilters(store.notes.filter(n => n.groupId === groupId))
}

// 获取分组下的所有笔记（递归包含子分组的笔记）
function getAllGroupNotes(groupId: string): Note[] {
  const direct = getGroupNotes(groupId)
  const childGroups = store.groups.filter(g => g.parentId === groupId)
  const childNotes = childGroups.flatMap(g => getAllGroupNotes(g.id))
  return [...direct, ...childNotes]
}

// 移动分组菜单用：递归展平全部分组（始终展开），含 depth 用于缩进
const groupTreeForMove = computed(() => {
  const result: { group: NoteGroup; depth: number }[] = []
  function walk(parentId: string | null, depth: number) {
    const children = store.groups.filter(g => g.parentId === parentId).sort((a, b) => a.sortOrder - b.sortOrder)
    for (const g of children) {
      result.push({ group: g, depth })
      walk(g.id, depth + 1)
    }
  }
  walk(null, 0)
  return result
})

// 二级菜单（移动到分组）智能定位：响应主菜单位置和翻转方向
const subMenuStyle = computed(() => {
  const x = ctxPos.value.x
  const y = ctxPos.value.y
  const vw = window.innerWidth
  const vh = window.innerHeight
  const menuW = 150
  const menuH = Math.min(groupTreeForMove.value.length * 24 + 8, 320)

  // 水平：右侧空间不够则翻到左侧
  const rightSpace = vw - x - 135
  const left = rightSpace < menuW ? (x - menuW - 4) : (x + 135)

  // 垂直：主菜单向上时子菜单也向上；否则下方空间不够也向上
  const below = ctxUpward.value ? -1 : (vh - y)
  const above = ctxUpward.value ? (y - 4) : y
  const flipUp = below < menuH && above > menuH
  const top = flipUp ? Math.max(4, y - menuH + 4) : Math.min(y, vh - menuH - 4)

  return { left: `${left}px`, top: `${top}px` }
})

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
onMounted(() => document.addEventListener('click', closeGroupCtx))
</script>

<template>
  <aside class="note-sidebar">
    <!-- Header: title + calendar toggle -->
    <div class="ns-header">
      <span class="ns-title">日历</span>
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
      <button class="ns-btn icon" title="刷新列表" @click="handleRefresh">
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M1 8a7 7 0 0 1 13.2-3.2M15 8a7 7 0 0 1-13.2 3.2"/>
          <path d="M11 1.5V5h-3.5M5 14.5V11h3.5"/>
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

    <!-- Notes list (with recursive group tree) -->
    <div v-show="listTab === 'notes'" class="ns-tree">

      <template v-for="item in flatGroupTree" :key="item.kind === 'group' ? item.group.id : item.note.id">
        <!-- 分组行 -->
        <div v-if="item.kind === 'group'" class="ns-item group" :class="{ sel: store.selectedGroupId === item.group.id }"
          :style="{ paddingLeft: (8 + item.depth * 16) + 'px' }"
          @contextmenu="openGroupCtx($event, item.group.id)">
          <template v-if="editingGroupId === item.group.id">
            <input class="ns-edit-input" v-model="editingGroupName" @keyup.enter="confirmRenameGroup"
              @keyup.escape="editingGroupId = ''" @blur="confirmRenameGroup" @click.stop autofocus />
          </template>
          <template v-else>
            <span class="ns-arrow" @click="toggleGroup(item.group.id)">{{ expandedGroups.has(item.group.id) ? '▾' : '▸' }}</span>
            <span class="ns-label" @click="toggleGroup(item.group.id)">{{ item.group.name }}</span>
            <span class="ns-count">{{ getAllGroupNotes(item.group.id).length }}</span>
            <button class="ns-del" @click.stop="store.removeGroup(item.group.id)" title="删除组">✕</button>
          </template>
        </div>
        <!-- 笔记行 -->
        <div v-else class="ns-item note"
          :class="{ sel: store.selectedNoteId === item.note.id, pinned: item.note.isPinned }"
          :style="{ paddingLeft: (24 + item.depth * 16) + 'px' }"
          @click="handleOpenNote(item.note.id)" @contextmenu="openCtx($event, item.note)" :title="item.note.title">
          <span class="ns-dot" :class="[item.note.content ? 'has-content' : 'empty', readNoteIds.has(item.note.id) ? 'read' : '']"></span><span class="ns-label">{{ item.note.title || '无标题' }}</span>
        </div>
      </template>

      <div class="ns-item add-group" @click="showingNewGroup = true">
        <template v-if="showingNewGroup || showingNewSubGroup">
          <input v-model="newGroupName" class="ns-add-input ns-add-sub-input" :placeholder="showingNewSubGroup ? '子组名...' : '组名...'" @keyup.enter="handleAddGroup"
            @keyup.escape="showingNewGroup = false; showingNewSubGroup = false" @blur="handleAddGroup" @click.stop autofocus />
        </template>
        <template v-else><span class="ns-label">+ 新建笔记组</span></template>
      </div>
    </div>

    <!-- Starred list (flat, no groups) -->
    <div v-show="listTab === 'starred'" class="ns-tree">
      <div v-if="starredNotes.length === 0" class="ns-empty">暂无星标笔记</div>
      <div v-for="n in starredNotes" :key="n.id" class="ns-item note" :class="{ sel: store.selectedNoteId === n.id }"
        @click="handleOpenNote(n.id)" @contextmenu="openCtx($event, n)" :title="n.title">
        <span class="ns-dot" :class="[n.content ? 'has-content' : 'empty', readNoteIds.has(n.id) ? 'read' : '']"></span><span class="ns-label">{{ n.title || '无标题' }}</span>
      </div>
    </div>

    <!-- Archived list (flat) -->
    <div v-show="listTab === 'archived'" class="ns-tree">
      <div v-if="store.notes.filter(n => !n.isDeleted && n.isArchived).length === 0" class="ns-empty">暂无归档笔记</div>
      <div v-for="n in store.notes.filter(x => !x.isDeleted && x.isArchived)" :key="n.id" class="ns-item note"
        :class="{ sel: store.selectedNoteId === n.id }" @click="handleOpenNote(n.id)" @contextmenu="openCtx($event, n)"
        :title="n.title">
        <span class="ns-dot" :class="[n.content ? 'has-content' : 'empty', readNoteIds.has(n.id) ? 'read' : '']"></span><span class="ns-label">{{ n.title || '无标题' }}</span>
      </div>
    </div>

    <!-- Group context menu -->
    <Teleport to="body">
      <div v-if="groupCtxShow" class="ctx" :class="{ upward: groupCtxUpward }"
        :style="groupCtxUpward
          ? { left: groupCtxPos.x + 'px', bottom: groupCtxBottom + 'px' }
          : { left: groupCtxPos.x + 'px', top: groupCtxPos.y + 'px' }"
        @click.stop>
        <div class="ci" @click="ctxNewNoteInGroup">📝 新建笔记</div>
        <div class="ci" @click="ctxNewSubGroup">📁 新建子分组</div>
        <div class="ci" @click="ctxRenameGroup">✏️ 重命名</div>
        <div class="ci" style="color:var(--jc-color-error)" @click="ctxDelGroup">🗑 删除分组</div>
      </div>
    </Teleport>

    <!-- Context menu -->
    <Teleport to="body">
      <div v-if="ctxShow" class="ctx" :class="{ upward: ctxUpward }"
        :style="ctxUpward
          ? { left: ctxPos.x + 'px', bottom: ctxBottom + 'px' }
          : { left: ctxPos.x + 'px', top: ctxPos.y + 'px' }"
        @click.stop>
        <div class="ci" @click="ctxEdit">编辑</div>
        <div class="ci" @click="ctxCopy">复制内容</div>
        <div class="ci" @click="ctxTogglePin">{{ ctxNote?.isPinned ? '★ 取消星标' : '☆ 设为星标' }}</div>
        <div class="ci" @click="ctxToggleArchive">{{ ctxNote?.isArchived ? '取消归档' : '归档' }}</div>
        <div class="ci" @click="ctxRename">重命名</div>
        <div class="ci" style="display:flex;align-items:center;justify-content:space-between" @mouseenter="ctxShowMove = true">
          移动到分组 <span style="font-size:10px">▸</span>
        </div>
        <div class="ci" style="color:var(--jc-color-error)" @click="ctxDelete">删除</div>
      </div>
    </Teleport>

    <!-- 移动分组子菜单（递归树结构，智能翻转） -->
    <Teleport to="body">
      <div
        v-if="ctxShow && ctxShowMove"
        class="ctx ctx-sub"
        :style="subMenuStyle"
        @mouseleave="ctxShowMove = false"
      >
        <template v-for="item in groupTreeForMove" :key="item.group.id">
          <div class="ci" :style="{ paddingLeft: (8 + item.depth * 14) + 'px' }" @click="moveNoteToGroup(item.group.id)">
            {{ item.depth > 0 ? '└ ' : '' }}📁 {{ item.group.name }}
          </div>
        </template>
      </div>
    </Teleport>

    <!-- Rename modal -->
    <Teleport to="body">
      <div v-if="renameShow" class="mbg" @mousedown.self="renameShow = false">
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
      <div v-if="deleteConfirmShow" class="mbg" @mousedown.self="deleteConfirmShow = false">
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
    flex-shrink: 0;
    transition: background .2s;

    &.has-content { background: #3fb950; }
    &.has-content.read { background: #58a6ff; }
    &.empty { background: var(--jc-text-secondary); opacity: 0.5; }
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

<style lang="scss">
// 右键菜单样式 — 不能 scoped，因为 Teleport 到 body 后 DOM 脱离组件作用域
@use "@/styles/mixins.scss" as *;
.ctx {
  @include ctx-menu;
  min-width: 130px;
}
.ctx.upward {
  // 从底部向上展开：菜单项顺序反转
  display: flex;
  flex-direction: column-reverse;
}
.ctx-sub {
  min-width: 130px;
  z-index: 10001;
}
.ci {
  @include ctx-item;
}
</style>
