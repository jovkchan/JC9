<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useAutomationStore } from '@/stores/automation'
import { useStatusStore } from '@/stores/status'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcModal from '@/components/ui/JcModal.vue'
import JcContextMenu from '@/components/ui/JcContextMenu.vue'
import type { JcContextMenuItem } from '@/components/ui'

const store = useAutomationStore()
const status = useStatusStore()
onMounted(() => store.load())

// ── 导入（完整 JSON，见方案 §4.6）──
const importOpen = ref(false)
const importText = ref('')

async function pickImportFile() {
  try {
    const selected = await open({ filters: [{ name: '工作积木 JSON', extensions: ['json'] }], multiple: false })
    if (selected && typeof selected === 'string') {
      const content = await invoke<string>('read_file_string', { path: selected })
      importText.value = content
      doImport()
    }
  } catch (e) { status.pushMessage(`读取文件失败: ${e}`, 'error') }
}

function doImport() {
  const json = importText.value.trim()
  if (!json) { status.pushMessage('请先粘贴或选择工作积木 JSON', 'warn'); return }
  const a = store.importAutomationJson(json)
  if (a) {
    status.pushMessage(`已导入「${a.name}」`, 'success')
    importOpen.value = false
    importText.value = ''
  } else {
    status.pushMessage('导入失败：不是有效的工作积木 JSON（需含 nodes/edges）', 'error')
  }
}

// ── 列表项右键菜单：复制 / 编辑 / 删除 ──
const ctxShow = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxId = ref('')

const ctxItems: JcContextMenuItem[] = [
  { label: '运行', value: 'run' },
  { label: '编辑', value: 'edit' },
  { label: '复制', value: 'duplicate' },
  { label: '复制 ID', value: 'copy-id' },
  { label: '删除', value: 'delete', danger: true },
]

function openCtx(e: MouseEvent, id: string) {
  e.preventDefault()
  ctxId.value = id
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxShow.value = true
}

function closeCtx() { ctxShow.value = false }

/** 运行态卡片样式：运行中流光，结束/出错用颜色边框区分 */
function runClass(a: { id: string }) {
  const st = store.runStateOf(a.id)
  if (!st) return ''
  if (st.status === 'running') return 'is-running'
  return `st-${st.status}`
}

function onCtxSelect(item: JcContextMenuItem) {
  const id = ctxId.value
  if (!id) return
  if (item.value === 'run') store.run(id)
  else if (item.value === 'edit') store.open(id)
  else if (item.value === 'duplicate') { const c = store.duplicate(id); if (c) store.open(c.id) }
  else if (item.value === 'copy-id') {
    navigator.clipboard.writeText(id)
      .then(() => status.pushMessage(`已复制工作积木 ID：${id}`, 'success'))
      .catch(e => status.pushMessage(`复制失败: ${e}`, 'error'))
  }
  else if (item.value === 'delete') store.remove(id)
  closeCtx()
}
</script>

<template>
  <section class="automation-list">
    <div class="al-header">
      <span class="al-title">工作积木</span>
      <div class="al-header-acts">
        <JcButton size="small" @click="importOpen = true">导入</JcButton>
        <JcButton size="small" type="primary" @click="store.create()">+ 新建</JcButton>
      </div>
    </div>
    <div class="al-search">
      <JcInput beam glow v-model="store.search" placeholder="搜索工作积木" />
    </div>
    <div class="al-body">
      <div
        v-for="a in store.filtered"
        :key="a.id"
        class="al-item"
        :class="[{ on: a.id === store.currentId }, runClass(a)]"
        @contextmenu.prevent="openCtx($event, a.id)"
      >
        <div class="al-item-main">
          <span class="al-item-name">{{ a.name }}</span>
          <span class="al-item-desc">{{ a.description || `${a.nodes.length} 块 · ${a.edges.length} 连线` }}</span>
        </div>
        <button class="al-item-run" title="运行" @click.stop="store.run(a.id)">▶</button>
      </div>
      <div v-if="store.filtered.length === 0" class="al-empty">
        暂无工作积木任务<br />点击「+ 新建」开始搭建
      </div>
    </div>
    <div class="al-foot">
      <span class="al-foot-hint">积木编辑器 · F1a 骨架</span>
    </div>
    <JcContextMenu :show="ctxShow" :x="ctxPos.x" :y="ctxPos.y" :items="ctxItems" @update:show="ctxShow = $event" @select="onCtxSelect" />

    <!-- 导入完整 JSON -->
    <JcModal :open="importOpen" title="导入工作积木" width="520" @update:open="importOpen = $event">
      <JcTextarea v-model="importText" :rows="12" :spellcheck="false" placeholder='粘贴 automation JSON（含 nodes/edges）' />
      <template #footer>
        <JcButton @click="pickImportFile">从文件选择</JcButton>
        <JcButton type="primary" @click="doImport">导入</JcButton>
      </template>
    </JcModal>
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
.al-header-acts {
  display: flex;
  gap: 6px;
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
  position: relative;
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
/* 运行结束态：边框颜色区分 */
.al-item.st-done { border-color: rgba(82, 196, 26, .55); }
.al-item.st-failed { border-color: rgba(255, 77, 79, .65); }
.al-item.st-stopped { border-color: rgba(250, 173, 20, .65); }
/* 运行中：流光描边（遵循系统「动作效果」配置：总开关 html.jc-beam-off 时隐藏；速度用 --jc-beam-duration） */
html.jc-beam-off .al-item.is-running::before { display: none; }
.al-item.is-running::before {
  content: '';
  position: absolute;
  inset: -1px;
  border-radius: inherit;
  padding: 1.5px;
  background: conic-gradient(from var(--jc-al-ang, 0deg), transparent 0 330deg, var(--jc-color-accent, #8a58ff) 360deg);
  -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
  -webkit-mask-composite: xor;
  mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
  mask-composite: exclude;
  animation: al-item-flow var(--jc-beam-duration, 1.6s) linear infinite;
  pointer-events: none;
}
@property --jc-al-ang { syntax: '<angle>'; initial-value: 0deg; inherits: false; }
@keyframes al-item-flow { to { --jc-al-ang: 360deg; } }
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
