<script setup lang="ts">
import { ref, computed } from 'vue'
import { useProjectStore } from '@/stores/project'
import type { LogStats } from '@/stores/project'

const props = defineProps<{ processId: string }>()
const store = useProjectStore()
const filterLevel = ref<'all'|'error'|'warn'|'info'|'debug'>('all')

const stats = computed<LogStats>(() => store.logStatsMap[props.processId] ?? { error: 0, warn: 0, info: 0, debug: 0 })
const total = computed(() => stats.value.error + stats.value.warn + stats.value.info + stats.value.debug)

function filterBy(level: typeof filterLevel.value) { filterLevel.value = level }
function openFilteredTab() {
  const buf = store.getOutput(props.processId)
  if (!buf.length) return
  const text = new TextDecoder().decode(new Uint8Array(buf))
  const lines = text.split(/\r?\n/)
  const filtered = filterLevel.value === 'all' ? lines : lines.filter(l => {
    const u = l.toUpperCase()
    if (filterLevel.value === 'error') return /\bERROR\b|\bFATAL\b|\bPANIC\b/.test(u)
    if (filterLevel.value === 'warn') return /\bWARN(ING)?\b/.test(u)
    if (filterLevel.value === 'debug') return /\bDEBUG\b/.test(u)
    if (filterLevel.value === 'info') return /\bINFO\b|\bTRACE\b/.test(u) && !/\bERROR\b|\bFATAL\b|\bPANIC\b|\bWARN(ING)?\b|\bDEBUG\b/.test(u)
    return true
  })
  const content = filtered.join('\n')
  const label = `日志: ${filterLevel.value}`
  store.openDocFromText(props.processId, label, content)
}
</script>

<template>
  <div class="log-panel">
    <div class="lp-header">
      <span class="lp-title"> 日志统计</span>
    </div>
    <div class="lp-body">
      <div class="lp-stats">
        <div :class="['lp-badge',{on:filterLevel==='all'}]" @click="filterBy('all')">全部 {{ total }}</div>
        <div :class="['lp-badge','err',{on:filterLevel==='error'}]" @click="filterBy('error')">错误 {{ stats.error }}</div>
        <div :class="['lp-badge','warn',{on:filterLevel==='warn'}]" @click="filterBy('warn')">警告 {{ stats.warn }}</div>
        <div :class="['lp-badge','info',{on:filterLevel==='info'}]" @click="filterBy('info')">信息 {{ stats.info }}</div>
        <div :class="['lp-badge','dbg',{on:filterLevel==='debug'}]" @click="filterBy('debug')">调试 {{ stats.debug }}</div>
      </div>
      <button class="btn-sm" @click="openFilteredTab">打开筛选结果</button>
    </div>
  </div>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;

.log-panel { display:flex; flex-direction:column; background:var(--jc-bg-panel); border-left:1px solid var(--jc-border-default); width:200px; flex-shrink:0; }
.lp-header { padding:8px 10px; border-bottom:1px solid var(--jc-border-default); }
.lp-title { font-size:12px; font-weight:600; color:var(--jc-text-highlight); }
.lp-body { padding:8px 10px; display:flex; flex-direction:column; gap:6px; overflow-y:auto; flex:1; }
.lp-stats { display:flex; flex-direction:column; gap:2px; }
.lp-badge { padding:3px 8px; font-size:11px; cursor:pointer; border-radius:3px;
  &:hover { background:var(--jc-bg-hover); }
  &.on { background:var(--jc-bg-selected); font-weight:600; }
  &.err { color:var(--jc-color-error); }
  &.warn { color:var(--jc-color-warning); }
  &.info { color:var(--jc-color-success); }
  &.dbg { color:var(--jc-text-secondary); }
}
.btn-sm { @include btn-base; padding:4px 10px; font-size:11px; text-align:center; }
</style>
