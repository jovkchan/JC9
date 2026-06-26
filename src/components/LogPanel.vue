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
  <button class="lp-toggle" @click="show=!show" :title="show?'收起日志':'打开日志'">{{ show?'▶':'📊' }}</button>
</template>

<style scoped>
.log-panel { width:200px; min-width:200px; background:#252526; border-left:1px solid #3e3e42; display:flex; flex-direction:column; overflow:hidden; }
.lp-header { display:flex; justify-content:space-between; align-items:center; padding:6px 8px; border-bottom:1px solid #3e3e42; }
.lp-title { font-size:12px; font-weight:600; }
.lp-close { background:none; color:#858585; font-size:14px; padding:0 4px; cursor:pointer; }
.lp-close:hover { color:#f44747; }
.lp-search { padding:6px 8px; }
.lp-input { width:100%; background:#3c3c3c; border:1px solid #555; padding:4px 6px; color:#ccc; font-size:11px; }
.lp-input:focus { border-color:#007acc; }
.lp-stats { display:flex; flex-direction:column; gap:2px; padding:6px 8px; }
.lp-badge { padding:3px 8px; font-size:11px; cursor:pointer; border-radius:3px; }
.lp-badge:hover { background:#2a2d2e; }
.lp-badge.on { background:#37373d; font-weight:600; }
.lp-badge.err { color:#f44747; }
.lp-badge.warn { color:#d7ba7d; }
.lp-badge.info { color:#4ec9b0; }
.lp-badge.dbg { color:#858585; }
.btn-sm { background:#3c3c3c; color:#ccc; padding:4px 10px; font-size:11px; width:100%; text-align:center; }
.btn-sm:hover { background:#4c4c4c; }
.lp-toggle { position:absolute; right:0; top:50%; transform:translateY(-50%); background:#2d2d30; border:1px solid #3e3e42; color:#858585; padding:4px 6px; font-size:14px; cursor:pointer; z-index:10; }
.lp-toggle:hover { background:#37373d; color:#ccc; }
</style>
