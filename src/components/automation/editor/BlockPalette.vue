<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAutomationStore } from '@/stores/automation'
import { PALETTE_SECTIONS, getBlockDef, getBlockColor, getBlockLabel, defaultsFromFields } from '@/components/automation/blocks/palette'
import { BLOCK_W, blockHeight, blockSummary } from '@/components/automation/blocks/summary'
import { BLOCK_HELP, PLANNED_BLOCKS } from '@/components/automation/blocks/help'
import JcSegmented from '@/components/ui/JcSegmented.vue'
import type { JcSegmentedOption } from '@/components/ui/JcSegmented.vue'

const store = useAutomationStore()

const tab = ref<'blocks' | 'help'>('blocks')
const TABS: JcSegmentedOption[] = [
  { label: '积木', value: 'blocks' },
  { label: '帮助', value: 'help' },
]

/** 帮助页展示的积木类型（按调色板顺序） */
const helpTypes = computed(() => PALETTE_SECTIONS.flatMap(s => s.blocks))
function helpOf(t: string) { return BLOCK_HELP[t] }
function hasIn(t: string) { return (getBlockDef(t)?.inputs.length ?? 0) > 0 }
function hasOut(t: string) { return (getBlockDef(t)?.outputs.length ?? 0) > 0 }
function isCredOut(t: string) { return getBlockDef(t)?.outputs.some(o => o.dataType === 'credential') ?? false }

/** 缺失/规划积木按分区分组 */
const plannedGroups = computed(() => {
  const m = new Map<string, typeof PLANNED_BLOCKS>()
  for (const p of PLANNED_BLOCKS) {
    const arr = m.get(p.section) ?? []
    arr.push(p)
    m.set(p.section, arr)
  }
  return [...m.entries()]
})

// ── 拖拽到画布添加（位置用户指定；落点由编辑器转换并添加）──
const ghost = ref<{ type: string; x: number; y: number } | null>(null)
/** 拖拽预览：与放置后实际块一致的大小（画布缩放 × 块尺寸）与样式 */
const ghostStyle = computed(() => {
  if (!ghost.value) return {}
  const def = getBlockDef(ghost.value.type)
  const cfg = defaultsFromFields(def?.fields ?? [])
  const sc = store.canvasScale || 1
  return {
    width: Math.round(BLOCK_W * sc) + 'px',
    height: Math.round(blockHeight(ghost.value.type, cfg) * sc) + 'px',
  }
})
const ghostSummary = computed(() => {
  if (!ghost.value) return []
  const def = getBlockDef(ghost.value.type)
  return blockSummary(ghost.value.type, defaultsFromFields(def?.fields ?? []))
})
let moveFn: ((e: PointerEvent) => void) | null = null
let upFn: ((e: PointerEvent) => void) | null = null
function onItemDown(type: string, e: PointerEvent) {
  if (e.button !== 0) return
  e.preventDefault()
  document.body.style.userSelect = 'none' // 屏蔽拖拽时的文字选择
  ghost.value = { type, x: e.clientX + 12, y: e.clientY + 12 }
  moveFn = (ev: PointerEvent) => { ghost.value = { type, x: ev.clientX + 12, y: ev.clientY + 12 } }
  upFn = (ev: PointerEvent) => {
    document.removeEventListener('pointermove', moveFn!)
    document.removeEventListener('pointerup', upFn!)
    document.body.style.userSelect = ''
    moveFn = null; upFn = null
    ghost.value = null
    store.dropBlock({ type, clientX: ev.clientX, clientY: ev.clientY })
  }
  document.addEventListener('pointermove', moveFn)
  document.addEventListener('pointerup', upFn)
}
</script>

<template>
  <section class="block-palette">
    <div class="bp-header">
      <JcSegmented :model-value="tab" :options="TABS" @update:model-value="(v) => (tab = v as 'blocks' | 'help')" />
    </div>

    <!-- 拖拽到画布的浮动预览（虚线 + 半透明，与实际放置后一致大小） -->
    <div v-if="ghost" class="bp-ghost" :style="{ left: ghost.x + 'px', top: ghost.y + 'px', ...ghostStyle }">
      <div class="bp-ghost-body">
        <div class="bp-ghost-title">{{ getBlockLabel(ghost.type) }}</div>
        <div v-for="(l, i) in ghostSummary" :key="i" class="bp-ghost-line">{{ l }}</div>
      </div>
    </div>

    <!-- 标签页 1：积木调色板 -->
    <div v-if="tab === 'blocks'" class="bp-body">
      <div v-for="sec in PALETTE_SECTIONS" :key="sec.key" class="bp-sec">
        <div class="bp-sec-label">{{ sec.label }}</div>
        <div
          v-for="t in sec.blocks"
          :key="t"
          class="bp-item"
          :title="getBlockDef(t)?.fields.map(f => f.label).join(' / ') || ''"
          @pointerdown="onItemDown(t, $event)"
        >
          <span class="bp-dot" :style="{ background: getBlockColor(t) }"></span>
          <span class="bp-name">{{ getBlockLabel(t) }}</span>
        </div>
      </div>
    </div>

    <!-- 标签页 2：帮助系统 -->
    <div v-else class="bp-help">
      <div class="bp-help-intro">各积木的用途、适用场景、下游关联与组合方式。新增积木会同步更新。</div>

      <div v-for="t in helpTypes" :key="t" class="bp-help-card">
        <div class="bp-help-fig">
          <span v-if="hasIn(t)" class="bp-fig-port bp-fig-in"></span>
          <span class="bp-fig-block" :style="{ borderColor: getBlockColor(t) }">{{ getBlockLabel(t) }}</span>
          <span v-if="hasOut(t)" class="bp-fig-port bp-fig-out" :class="{ 'is-cred': isCredOut(t) }"></span>
        </div>
        <dl v-if="helpOf(t)" class="bp-help-dl">
          <div class="bp-help-row"><dt>用途</dt><dd>{{ helpOf(t)!.usage }}</dd></div>
          <div class="bp-help-row"><dt>适用</dt><dd>{{ helpOf(t)!.when }}</dd></div>
          <div class="bp-help-row"><dt>下游</dt><dd>{{ helpOf(t)!.downstream }}</dd></div>
          <div class="bp-help-row"><dt>组合</dt>
            <dd>
              <ul class="bp-help-list">
                <li v-for="(c, i) in helpOf(t)!.combos" :key="i">{{ c }}</li>
              </ul>
            </dd>
          </div>
        </dl>
        <div v-else class="bp-help-missing">帮助待补充</div>
      </div>

      <!-- 缺失 / 规划中积木 -->
      <div class="bp-plan">
        <div class="bp-plan-title">规划中 / 缺失积木</div>
        <div v-for="[sec, list] in plannedGroups" :key="sec" class="bp-plan-sec">
          <div class="bp-plan-sec-label">{{ sec }}</div>
          <div v-for="p in list" :key="p.name" class="bp-plan-row">
            <span class="bp-plan-name">{{ p.name }}</span>
            <span class="bp-plan-note">{{ p.note }}</span>
            <span v-if="p.milestone" class="bp-plan-ms">{{ p.milestone }}</span>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped lang="scss">
.block-palette {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.bp-header {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px 6px;
}
.bp-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--jc-text-primary);
}
.bp-sub {
  font-size: 11px;
  color: var(--jc-text-tertiary);
}
.bp-body {
  flex: 1;
  overflow-y: auto;
  padding: 0 12px 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.bp-sec {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
}
.bp-sec-label {
  grid-column: 1 / -1;
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 2px;
}
.bp-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: 6px;
  cursor: pointer;
  user-select: none;
  -webkit-user-select: none;
  background: var(--jc-bg-input);
  border: 1px solid transparent;
  transition: background 0.15s, border-color 0.15s, transform 0.1s;
}
.bp-item:hover {
  background: var(--jc-bg-hover);
  border-color: var(--jc-border-default);
}
.bp-item:active {
  transform: scale(0.98);
}
/* 拖拽浮动预览：虚线 + 半透明，明确是「放置预览态」（无左侧色条，全边框统一） */
.bp-ghost {
  position: fixed;
  z-index: 3000;
  pointer-events: none;
  display: flex;
  align-items: stretch;
  border-radius: 8px;
  overflow: hidden;
  background: var(--jc-bg-elevated);
  border: 1px dashed rgba(138, 88, 255, 0.75);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4);
  opacity: 0.9;
  .bp-ghost-body { display: flex; flex-direction: column; justify-content: center; gap: 2px; padding: 6px 12px; min-width: 0; }
  .bp-ghost-title { font-size: 13px; font-weight: 500; color: var(--jc-text-primary); white-space: nowrap; }
  .bp-ghost-line { font-size: 11px; color: var(--jc-text-secondary); white-space: nowrap; }
}
.bp-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.bp-name {
  font-size: 12px;
  color: var(--jc-text-primary);
}

/* ── 帮助系统 ── */
.bp-help {
  flex: 1;
  overflow-y: auto;
  padding: 0 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.bp-help-intro {
  font-size: 11px;
  color: var(--jc-text-tertiary);
  line-height: 1.5;
}
.bp-help-card {
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: 8px;
  padding: 10px;
  background: var(--jc-bg-input, #3c3c3c);
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.bp-help-fig {
  display: flex;
  align-items: center;
  gap: 6px;
}
.bp-fig-block {
  flex: 1;
  min-width: 0;
  text-align: center;
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-primary);
  border: 2px solid;
  border-radius: 6px;
  padding: 6px 8px;
  background: var(--jc-bg-elevated, #2d2d30);
}
.bp-fig-port {
  width: 12px;
  height: 6px;
  border-radius: 0 3px 3px 0;
  flex-shrink: 0;
  background: var(--jc-color-accent, #8a58ff);
}
.bp-fig-in {
  border-radius: 3px 0 0 3px;
}
.bp-fig-out.is-cred {
  background: #faad14;
}
.bp-help-dl {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
}
.bp-help-row {
  display: flex;
  gap: 8px;
  font-size: 12px;
  line-height: 1.55;
}
.bp-help-row dt {
  flex-shrink: 0;
  width: 34px;
  color: var(--jc-text-secondary, #aaa);
  font-weight: 600;
}
.bp-help-row dd {
  margin: 0;
  color: var(--jc-text-primary, #e6e6e6);
  min-width: 0;
}
.bp-help-list {
  margin: 0;
  padding-left: 16px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.bp-help-missing {
  font-size: 12px;
  color: var(--jc-text-tertiary, #858585);
}

/* ── 缺失 / 规划中 ── */
.bp-plan {
  border: 1px dashed var(--jc-border-strong, #555);
  border-radius: 8px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.bp-plan-title {
  font-size: 12px;
  font-weight: 700;
  color: var(--jc-color-warning, #ff9c6e);
}
.bp-plan-sec {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.bp-plan-sec-label {
  font-size: 11px;
  color: var(--jc-text-secondary, #aaa);
  font-weight: 600;
}
.bp-plan-row {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: 12px;
}
.bp-plan-name {
  flex-shrink: 0;
  color: var(--jc-text-primary, #e6e6e6);
}
.bp-plan-note {
  flex: 1;
  min-width: 0;
  color: var(--jc-text-secondary, #aaa);
  font-size: 11px;
}
.bp-plan-ms {
  flex-shrink: 0;
  font-size: 10px;
  color: var(--jc-color-accent, #8a58ff);
  border: 1px solid var(--jc-color-accent, #8a58ff);
  border-radius: 4px;
  padding: 0 4px;
}
</style>
