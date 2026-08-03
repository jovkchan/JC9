<script setup lang="ts">
import { useAutomationStore } from '@/stores/automation'
import { PALETTE_SECTIONS, getBlockDef, getBlockColor, getBlockLabel } from '@/components/automation/blocks/palette'

const store = useAutomationStore()
</script>

<template>
  <section class="block-palette">
    <div class="bp-header">
      <span class="bp-title">积木</span>
      <span class="bp-sub">点击添加到画布</span>
    </div>
    <div class="bp-body">
      <div v-for="sec in PALETTE_SECTIONS" :key="sec.key" class="bp-sec">
        <div class="bp-sec-label">{{ sec.label }}</div>
        <div
          v-for="t in sec.blocks"
          :key="t"
          class="bp-item"
          :title="getBlockDef(t)?.fields.map(f => f.label).join(' / ') || ''"
          @click="store.addNode(t)"
        >
          <span class="bp-dot" :style="{ background: getBlockColor(t) }"></span>
          <span class="bp-name">{{ getBlockLabel(t) }}</span>
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
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.bp-sec-label {
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
</style>
