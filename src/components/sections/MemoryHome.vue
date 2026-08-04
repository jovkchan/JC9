<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '@/stores/project'
import { useStatusStore } from '@/stores/status'
import JcInput from '@/components/ui/JcInput.vue'
import JcButton from '@/components/ui/JcButton.vue'

interface Memory {
  id: string; scope: string; topicKey: string; title: string; content: string
  memoryType: string; tags: string[]; createdAt: string; updatedAt: string
}

const store = useProjectStore()
const status = useStatusStore()

const memoryList = ref<Memory[]>([])
const memorySearch = ref('')
const memoryPage = ref(1)
const memoryPageSize = 20
const memoryTotal = ref(0)
const memoryTotalPages = computed(() => Math.max(1, Math.ceil(memoryTotal.value / memoryPageSize)))
const compressSelected = ref<string[]>([])

const allMemoryScopes = ref<string[]>([])
const memoryScope = ref('')

function openMemory(m: Memory) {
  store.openMemoryTab({
    id: m.id,
    title: m.title,
    content: m.content,
    type: m.memoryType,
    scope: m.scope,
  })
}

async function loadMemoryList() {
  try {
    const result = await invoke<{ items: Memory[]; total: number }>('get_memories', {
      search: memorySearch.value,
      page: memoryPage.value,
      pageSize: memoryPageSize,
      scope: memoryScope.value,
    })
    memoryList.value = result.items
    memoryTotal.value = result.total
    if (!memoryScope.value) {
      const scopes = new Set<string>()
      for (const m of result.items) { if (m.scope) scopes.add(m.scope) }
      allMemoryScopes.value = Array.from(scopes).sort()
    } else if (allMemoryScopes.value.length === 0) {
      const full = await invoke<{ items: Memory[]; total: number }>('get_memories', { search: '', page: 1, pageSize: 1, scope: '' })
      const all = await invoke<{ items: Memory[]; total: number }>('get_memories', { search: '', page: 1, pageSize: Math.max(full.total, 1), scope: '' })
      const scopes = new Set<string>()
      for (const m of all.items) { if (m.scope) scopes.add(m.scope) }
      allMemoryScopes.value = Array.from(scopes).sort()
    }
  } catch { memoryList.value = []; memoryTotal.value = 0 }
}

function searchMemory() { memoryPage.value = 1; compressSelected.value = []; loadMemoryList() }

function goToPage(page: number) {
  if (page < 1 || page > memoryTotalPages.value) return
  memoryPage.value = page; compressSelected.value = []; loadMemoryList()
}

async function deleteMemory(id: string) {
  try {
    await invoke('delete_memory', { id })
    status.pushMessage('记忆已删除', 'success')
    await loadMemoryList()
  } catch (e: any) { status.pushMessage('删除失败: ' + e, 'error') }
}

function openCreateMemory() {
  const id = crypto.randomUUID()
  store.openMemoryTab({ id, title: '', content: '', type: 'discovery', scope: memoryScope.value, topicKey: '' }, true)
}

async function compressMemories() {
  const ids = compressSelected.value
  if (ids.length < 2) return
  try {
    await invoke('compress_memories', { ids })
    status.pushMessage(`已压缩 ${ids.length} 条记忆`, 'success')
    compressSelected.value = []
    await loadMemoryList()
  } catch (e: any) { status.pushMessage('压缩失败: ' + e, 'error') }
}

onMounted(() => { loadMemoryList() })
</script>

<template>
  <div class="memory-home">
    <div class="memory-header">
      <div style="display:flex;align-items:center;justify-content:space-between">
        <span style="font-size:11px;color:var(--jc-text-secondary)">记忆管理</span>
        <button class="memory-create-btn" @click="openCreateMemory">+</button>
      </div>
      <div class="memory-scope-tabs">
        <span :class="['scope-tab', { active: !memoryScope }]" @click="memoryScope = ''; memoryPage = 1; loadMemoryList()">全部</span>
        <span v-for="sc in allMemoryScopes" :key="sc" :class="['scope-tab', { active: memoryScope === sc }]"
          @click="memoryScope = sc; memoryPage = 1; loadMemoryList()">{{ sc }}</span>
      </div>
      <div class="memory-search-row">
        <JcInput beam glow v-model="memorySearch" placeholder="搜索..." style="flex:1;min-width:0" @keyup.enter="searchMemory" />
        <JcButton size="small" @click="searchMemory">搜索</JcButton>
        <JcButton v-if="memorySearch" size="small" @click="memorySearch = ''; searchMemory()">清除</JcButton>
      </div>
    </div>

    <div class="memory-list">
      <div class="memory-list-bar">
        <span>{{ memoryTotal }} 条</span>
        <JcButton v-if="compressSelected.length > 1" size="small" @click="compressMemories">🗜 压缩 {{ compressSelected.length }}</JcButton>
      </div>

      <div v-if="memoryList.length === 0" class="memory-empty">{{ memorySearch ? '无匹配' : '暂无记忆' }}</div>

      <div v-for="m in memoryList" :key="m.id" class="memory-item">
        <input type="checkbox" :value="m.id" v-model="compressSelected" class="memory-checkbox" @click.stop />
        <div class="memory-item-body" @click="openMemory(m)">
          <span class="memory-item-title">{{ m.title }}</span>
          <div class="memory-item-tags">
            <span class="mem-tag type">{{ m.memoryType }}</span>
            <span v-if="m.scope && !memoryScope" class="mem-tag scope">{{ m.scope }}</span>
            <span v-if="m.topicKey" class="mem-tag topic">#{{ m.topicKey }}</span>
          </div>
        </div>
        <button class="memory-del-btn" @click.stop="deleteMemory(m.id)" title="删除">
          <svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M301.382 184.46h46.545v1.722h-46.545v-1.722z m186.135 0h46.546v1.722h-46.546v-1.722zM208.244 1024h605.091l93.091-837.818H720.105l-46.406 744.727h-46.546l46.406-744.727H534.063v744.727h-46.546V186.182H347.974l46.452 744.727h-46.545l-46.453-744.727H115.153z m465.408-839.54h46.546v1.722h-46.546v-1.722z m280.53-91.37c0-46.545-23.32-46.545-23.32-46.545H627.154S627.153 0 580.608 0H440.972c-46.546 0-46.546 46.545-46.546 46.545H93.137s-23.319 0-23.319 46.546c0 46.545 23.32 46.545 23.32 46.545h837.725s23.319 0 23.319-46.545z"/></svg>
        </button>
      </div>

      <div v-if="memoryTotalPages > 1" class="memory-pagination">
        <button class="page-btn" :disabled="memoryPage <= 1" @click="goToPage(memoryPage - 1)">‹</button>
        <span v-for="p in memoryTotalPages" :key="p">
          <button v-if="p === 1 || p === memoryTotalPages || Math.abs(p - memoryPage) <= 2"
            :class="['page-btn', { active: p === memoryPage }]" @click="goToPage(p)">{{ p }}</button>
          <span v-else-if="p === memoryPage - 3 || p === memoryPage + 3" class="page-ellipsis">…</span>
        </span>
        <button class="page-btn" :disabled="memoryPage >= memoryTotalPages" @click="goToPage(memoryPage + 1)">›</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.memory-home {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: var(--jc-bg-panel);
  padding: 8px;
  gap: 6px;
}
.memory-header { flex-shrink: 0; }
.memory-scope-tabs { display: flex; gap: 4px; flex-wrap: wrap; }
.scope-tab { font-size: 11px; padding: 3px 10px; border-radius: 4px; cursor: pointer; color: var(--jc-text-secondary); background: var(--jc-bg-hover); }
.scope-tab.active { color: var(--jc-color-white); background: var(--jc-color-accent); }
.memory-search-row { display: flex; gap: 6px; margin-top: 6px; }
.memory-search-input { flex: 1; padding: 4px 8px; font-size: 12px; border: 1px solid var(--jc-border-default); border-radius: 4px; background: var(--jc-bg-app); color: var(--jc-text-primary); }
.memory-search-btn, .memory-clear-btn { font-size: 11px; padding: 4px 10px; cursor: pointer; border: none; border-radius: 4px; background: var(--jc-color-accent); color: var(--jc-color-white); }
.memory-clear-btn { background: var(--jc-bg-btn); }
.memory-list { flex: 1; overflow-y: auto; min-height: 0; }
.memory-list-bar { font-size: 11px; color: var(--jc-text-secondary); margin-bottom: 4px; display: flex; align-items: center; justify-content: space-between; }
.memory-compress-btn { font-size: 10px; padding: 2px 6px; cursor: pointer; border: none; border-radius: 3px; background: var(--jc-color-accent); color: var(--jc-color-white); }
.memory-empty { font-size: 12px; color: var(--jc-text-secondary); padding: 12px; }
.memory-create-btn { font-size:16px; width:24px; height:24px; display:flex; align-items:center; justify-content:center; border:none; border-radius:4px; cursor:pointer; background:var(--jc-color-accent); color:var(--jc-color-white); line-height:1; }
.memory-item { display: flex; align-items: center; gap: 6px; padding: 5px 8px; border-radius: 4px; }
.memory-item:hover { background: var(--jc-bg-hover); }
.memory-checkbox { flex-shrink: 0; accent-color: var(--jc-color-accent); }
.memory-item-body { flex: 1; min-width: 0; cursor: pointer; }
.memory-item-title { font-weight: 500; font-size: 12px; display: block; }
.memory-item-tags { display: flex; gap: 4px; margin-top: 2px; flex-wrap: wrap; }
.mem-tag { font-size: 10px; padding: 1px 6px; border-radius: 3px; }
.mem-tag.type { color: var(--jc-color-success); background: rgba(35,199,120,0.1); }
.mem-tag.scope { color: #58a6ff; background: rgba(88,166,255,0.1); }
.mem-tag.topic { color: var(--jc-text-secondary); }
.memory-del-btn { flex-shrink: 0; background: none; border: none; color: var(--jc-text-secondary); cursor: pointer; padding: 2px 4px; }
.memory-del-btn:hover { color: var(--jc-color-error); }
.memory-pagination { display: flex; align-items: center; justify-content: center; gap: 4px; margin-top: 8px; }
.page-btn { font-size: 11px; padding: 2px 8px; cursor: pointer; border: 1px solid var(--jc-border-default); border-radius: 3px; background: var(--jc-bg-btn); color: var(--jc-text-primary); }
.page-btn.active { background: var(--jc-color-accent); color: var(--jc-color-white); border-color: var(--jc-color-accent); }
.page-btn:disabled { opacity: 0.5; cursor: default; }
.page-ellipsis { color: var(--jc-text-secondary); padding: 0 2px; font-size: 11px; }
</style>
