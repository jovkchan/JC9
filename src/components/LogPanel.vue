<script setup lang="ts">
import { ref, computed } from 'vue'
import { useProjectStore } from '@/stores/project'
import type { LogStats } from '@/stores/project'

const props = defineProps<{ processId: string }>()
const store = useProjectStore()
const show = ref(false)
const searchText = ref('')
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
  <div class="lp-root">
    <div class="log-panel" v-if="show">
      <div class="lp-header">
        <span class="lp-title">日志</span>
        <button class="lp-close" @click="show=false">✕</button>
      </div>
      <div class="lp-search">
        <input v-model="searchText" placeholder="搜索..." class="lp-input" />
      </div>
      <div class="lp-stats">
        <div :class="['lp-badge',{on:filterLevel==='all'}]" @click="filterBy('all')">全部 {{ total }}</div>
        <div :class="['lp-badge','err',{on:filterLevel==='error'}]" @click="filterBy('error')">错误 {{ stats.error }}</div>
        <div :class="['lp-badge','warn',{on:filterLevel==='warn'}]" @click="filterBy('warn')">警告 {{ stats.warn }}</div>
        <div :class="['lp-badge','info',{on:filterLevel==='info'}]" @click="filterBy('info')">信息 {{ stats.info }}</div>
        <div :class="['lp-badge','dbg',{on:filterLevel==='debug'}]" @click="filterBy('debug')">调试 {{ stats.debug }}</div>
      </div>
      <div style="padding:4px 8px;border-top:1px solid #3e3e42">
        <button class="btn-sm" @click="openFilteredTab">📄 打开筛选结果</button>
      </div>
    </div>
    <div class="lp-collapsed" v-else @click="show=true" title="打开日志面板">📊</div>
  </div>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;
.lp-root { display:flex; flex-shrink:0; }
.log-panel { width:200px; background:var(--jc-bg-panel); border-left:1px solid var(--jc-border-default); display:flex; flex-direction:column; overflow:hidden; }
.lp-collapsed { width:24px; min-width:24px; background:var(--jc-bg-elevated); border-left:1px solid var(--jc-border-default); display:flex; align-items:center; justify-content:center; cursor:pointer; font-size:13px; color:var(--jc-text-secondary); user-select:none;
  &:hover { background:var(--jc-bg-selected); color:var(--jc-text-primary); }
}
.lp-header { display:flex; justify-content:space-between; align-items:center; padding:6px 8px; border-bottom:1px solid var(--jc-border-default); }
.lp-title { font-size:12px; font-weight:600; }
.lp-close { background:none; color:var(--jc-text-secondary); font-size:14px; padding:0 4px; cursor:pointer;
  &:hover { color:var(--jc-color-error); }
}
.lp-search { padding:6px 8px; }
.lp-input { @include input-base; width:100%; padding:4px 6px; font-size:11px; }
.lp-stats { display:flex; flex-direction:column; gap:2px; padding:6px 8px; }
.lp-badge { padding:3px 8px; font-size:11px; cursor:pointer; border-radius:3px;
  &:hover { background:var(--jc-bg-hover); }
  &.on { background:var(--jc-bg-selected); font-weight:600; }
  &.err { color:var(--jc-color-error); }
  &.warn { color:var(--jc-color-warning); }
  &.info { color:var(--jc-color-success); }
  &.dbg { color:var(--jc-text-secondary); }
}
.btn-sm { @include btn-base; padding:4px 10px; font-size:11px; width:100%; text-align:center; }
</style>
