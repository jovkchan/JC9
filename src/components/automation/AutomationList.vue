<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAutomationStore } from '@/stores/automation'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcContextMenu from '@/components/ui/JcContextMenu.vue'
import type { JcContextMenuItem } from '@/components/ui'

const store = useAutomationStore()
onMounted(() => store.load())

// ── 列表项右键菜单：复制 / 编辑 / 删除 ──
const ctxShow = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxId = ref('')

const ctxItems: JcContextMenuItem[] = [
  { label: '运行', value: 'run', icon: '▶' },
  { label: '编辑', value: 'edit', icon: '✏️' },
  { label: '复制', value: 'duplicate', icon: '📄' },
  { label: '删除', value: 'delete', icon: '🗑️', danger: true },
]

function openCtx(e: MouseEvent, id: string) {
  e.preventDefault()
  ctxId.value = id
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxShow.value = true
}

function closeCtx() { ctxShow.value = false }

function onCtxSelect(item: JcContextMenuItem) {
  const id = ctxId.value
  if (!id) return
  if (item.value === 'run') store.run(id)
  else if (item.value === 'edit') store.open(id)
  else if (item.value === 'duplicate') { const c = store.duplicate(id); if (c) store.open(c.id) }
  else if (item.value === 'delete') store.remove(id)
  closeCtx()
}
</script>

<template>
  <section class="automation-list">
    <div class="al-header">
      <span class="al-title">自动化</span>
      <JcButton size="small" type="primary" @click="store.create()">+ 新建</JcButton>
    </div>
    <div class="al-search">
      <JcInput beam glow v-model="store.search" placeholder="搜索自动化" />
    </div>
    <div class="al-body">
      <div
        v-for="a in store.filtered"
        :key="a.id"
        class="al-item"
        :class="{ on: a.id === store.currentId }"
        @contextmenu.prevent="openCtx($event, a.id)"
      >
        <div class="al-item-main">
          <span class="al-item-name">{{ a.name }}</span>
          <span class="al-item-desc">{{ a.description || `${a.nodes.length} 块 · ${a.edges.length} 连线` }}</span>
        </div>
        <button class="al-item-run" title="运行" @click.stop="store.run(a.id)">▶</button>
      </div>
      <div v-if="store.filtered.length === 0" class="al-empty">
        暂无自动化任务<br />点击「+ 新建」开始搭建
      </div>
    </div>
    <div class="al-foot">
      <span class="al-foot-hint">积木编辑器 · F1a 骨架</span>
    </div>
    <JcContextMenu :show="ctxShow" :x="ctxPos.x" :y="ctxPos.y" :items="ctxItems" @update:show="ctxShow = $event" @select="onCtxSelect" />
  </section>
</template>

<style scoped lang="scss">
.automation-list {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.al-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px 6px;
}
.al-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--jc-text-primary);
}
.al-search {
  padding: 0 12px 8px;
}
.al-body {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.al-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: context-menu;
  background: var(--jc-bg-input);
  border: 1px solid transparent;
  transition: background 0.15s, border-color 0.15s;
}
.al-item:hover {
  background: var(--jc-bg-hover);
}
.al-item.on {
  border-color: var(--jc-color-accent);
  background: var(--jc-bg-hover);
}
.al-item-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.al-item-name {
  font-size: 12px;
  color: var(--jc-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.al-item-desc {
  font-size: 11px;
  color: var(--jc-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.al-item-run {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  line-height: 22px;
  text-align: center;
  font-size: 11px;
  color: var(--jc-color-success, #52c41a);
  background: transparent;
  border: 1px solid transparent;
  border-radius: 5px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s, background 0.15s, color 0.15s;
}
.al-item:hover .al-item-run { opacity: 1; }
.al-item-run:hover { background: var(--jc-color-success, #52c41a); color: #fff; }
.al-empty {
  padding: 32px 12px;
  text-align: center;
  font-size: 12px;
  color: var(--jc-text-secondary);
  line-height: 1.8;
}
.al-foot {
  padding: 8px 12px;
  border-top: 1px solid var(--jc-border-default);
}
.al-foot-hint {
  font-size: 11px;
  color: var(--jc-text-tertiary);
}
</style>
