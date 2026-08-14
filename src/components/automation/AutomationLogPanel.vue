<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useAutomationStore } from '@/stores/automation'
import { getBlockColor } from '@/components/automation/blocks/palette'
import type { RunLog } from '@/types/automation'
import JcButton from '@/components/ui/JcButton.vue'
import JcEmpty from '@/components/ui/JcEmpty.vue'
import JcSegmented from '@/components/ui/JcSegmented.vue'
import BlockGraph from './editor/BlockGraph.vue'

const store = useAutomationStore()
const selectedId = ref<string | null>(null)

onMounted(() => store.logsLoad())

const selected = computed<RunLog | null>(() => store.logs.find(x => x.id === selectedId.value) ?? null)

/** 标签页：运行时（图形化积木） | 日志 */
const tab = ref<'run' | 'log'>('run')
const tabOptions = [
  { label: '运行时', value: 'run' },
  { label: '日志', value: 'log' },
]
// 开始运行自动切到「运行时」
watch(() => store.liveRunId, (v) => { if (v) tab.value = 'run' })

/** 正在运行的自动化（供图形积木渲染） */
const runningGraph = computed<{ nodes: import('@/types/automation').BlockNode[]; edges: import('@/types/automation').Edge[] } | null>(() => {
  for (const id in store.runState) {
    if (store.runState[id].status === 'running') {
      const a = store.automations.find(x => x.id === id)
      if (a) return { nodes: a.nodes, edges: a.edges }
    }
  }
  return null
})
/** 已完成积木 id 集合（图形高亮） */
const doneIds = computed(() => [...new Set(store.liveSteps.filter(s => s.status === 'ok').map(s => s.blockId))])
/** 实时命令输出区：内容追加后自动滚动到底部跟随最新 */
const outEl = ref<HTMLPreElement | null>(null)
watch(() => store.liveOutput, () => {
  nextTick(() => { if (outEl.value) outEl.value.scrollTop = outEl.value.scrollHeight })
})
function pad(n: number) { return String(n).padStart(2, '0') }
function fmtTime(ms: number) {
  const d = new Date(ms)
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}
function fmtDur(ms: number) {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}
function fmtStatus(s: RunLog['status']) {
  return s === 'done' ? '成功' : s === 'failed' ? '失败' : '已停止'
}
function statusCls(s: RunLog['status']) {
  return s === 'done' ? 'ok' : s === 'failed' ? 'err' : 'stop'
}
function blockColor(type: string) {
  return getBlockColor(type)
}
</script>

<template>
  <div class="alogs">
    <div class="alogs-head">
      <span class="alogs-title">执行预览</span>
      <div class="alogs-acts">
        <JcSegmented :options="tabOptions" :model-value="tab" @update:model-value="tab = $event as 'run' | 'log'" />
        <JcButton size="small" @click="store.logsLoad(true)">刷新</JcButton>
      </div>
    </div>

    <!-- 运行时标签：图形化积木（连线 + 运行高亮）+ 下方实时日志浮动 -->
    <div v-show="tab === 'run'" class="alogs-view alogs-runview">
      <div class="alogs-graph">
        <BlockGraph
          v-if="runningGraph"
          :nodes="runningGraph.nodes"
          :edges="runningGraph.edges"
          :active-id="store.currentBlockId"
          :fail-id="store.failBlockId"
          :done-ids="doneIds"
        />
        <div v-else class="alogs-empty">
          <JcEmpty description="点击「运行」后这里以积木连线显示运行进度，运行到哪里哪块亮起" />
        </div>
      </div>
      <!-- 实时日志：运行时页签下方浮动出现 -->
      <div v-if="store.liveSteps.length || store.liveOutput" class="alogs-runlog">
        <div class="alogs-runlog-head">实时日志</div>
        <!-- 实时命令输出（仿终端）：长命令执行中滚动输出，不再假死 -->
        <pre ref="outEl" v-if="store.liveOutput" class="alogs-out">{{ store.liveOutput }}</pre>
        <div class="alogs-runlog-body">
          <div v-for="(s, i) in store.liveSteps" :key="'r' + i" class="alogs-ll" :class="s.status">
            <span class="alogs-ll-name">{{ s.name }}</span>
            <span v-if="s.detail" class="alogs-ll-detail">{{ s.detail }}</span>
            <span v-if="s.auth" class="alogs-ll-auth" :title="`凭据：${s.auth}`">鉴权 {{ s.auth }}</span>
            <span v-if="s.exitCode !== null" class="alogs-ll-code">码 {{ s.exitCode }}</span>
            <span class="alogs-ll-dur">{{ fmtDur(s.durationMs) }}</span>
            <div v-if="s.stdoutTail" class="alogs-ll-out mono">{{ s.stdoutTail }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 日志标签：历史列表 + 步骤详情 -->
    <div v-show="tab === 'log'" class="alogs-view alogs-log">
      <div class="alogs-body">
        <div v-if="store.logs.length === 0" class="alogs-empty">
          <JcEmpty description="暂无历史执行记录，运行自动化后这里会展示每个积木的执行日志" />
        </div>
        <div v-else class="alogs-list">
          <div
            v-for="r in store.logs"
            :key="r.id"
            class="alogs-run"
            :class="{ on: r.id === selectedId }"
            @click="selectedId = r.id"
          >
            <span class="alogs-run-status" :class="statusCls(r.status)">{{ fmtStatus(r.status) }}</span>
            <div class="alogs-run-main">
              <span class="alogs-run-name">{{ r.automationName }}</span>
              <span class="alogs-run-meta">{{ fmtTime(r.startedAt) }} · {{ fmtDur(r.durationMs) }} · {{ r.steps.length }} 步</span>
            </div>
            <span v-if="r.error" class="alogs-run-err" :title="r.error">{{ r.error }}</span>
          </div>
        </div>
      </div>
      <div v-if="selected" class="alogs-detail">
        <div class="alogs-detail-head">
          <span class="alogs-detail-title">{{ selected.automationName }} · {{ fmtStatus(selected.status) }}</span>
          <span class="alogs-detail-meta">{{ fmtTime(selected.startedAt) }} → {{ fmtTime(selected.endedAt) }} · 共 {{ selected.durationMs }}ms</span>
        </div>
        <div class="alogs-steps">
          <div v-for="s in selected.steps" :key="s.index" class="alogs-step" :class="s.status">
            <span class="alogs-step-dot" :style="{ background: blockColor(s.blockType) }"></span>
            <span class="alogs-step-idx">{{ s.index }}</span>
            <span class="alogs-step-name">{{ s.name }}</span>
            <span class="alogs-step-status">{{ s.status === 'ok' ? 'OK' : 'FAIL' }}</span>
            <span class="alogs-step-dur">{{ fmtDur(s.durationMs) }}</span>
            <span v-if="s.exitCode !== null" class="alogs-step-code">码 {{ s.exitCode }}</span>
            <span v-if="s.auth" class="alogs-step-auth" :title="`凭据：${s.auth}`">鉴权 {{ s.auth }}</span>
            <div class="alogs-step-detail">
              <div v-if="s.detail" class="alogs-step-line"><span class="lbl">执行</span>{{ s.detail }}</div>
              <div v-if="s.cwd" class="alogs-step-line"><span class="lbl">目录</span>{{ s.cwd }}</div>
              <div v-if="s.stdoutTail" class="alogs-step-line"><span class="lbl">输出</span><span class="mono">{{ s.stdoutTail }}</span></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.alogs {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
}
.alogs-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 14px 6px;
  .alogs-title { font-size: 13px; font-weight: 600; color: var(--jc-text-primary); }
  .alogs-acts {
    display: flex;
    align-items: center;
    gap: 8px;
  }
}
/* 标签视图容器 */
.alogs-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
/* 运行时页签：积木图区（flex:1，实时日志浮动覆盖其上，不挤压高度） */
.alogs-runview {
  position: relative;
}
.alogs-graph {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
/* 实时日志：独立浮动面板（覆盖预览底部，不与上方执行预览产生高度堆叠） */
.alogs-runlog {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 5;
  max-height: 240px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-top: 1px solid var(--jc-border-color);
  background: var(--jc-bg-panel, #252526);
  box-shadow: 0 -6px 16px rgba(0, 0, 0, 0.22);
  .alogs-runlog-head {
    padding: 8px 14px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--jc-text-primary);
    border-bottom: 1px solid var(--jc-border-color);
    flex-shrink: 0;
  }
  /* 实时命令输出流（仿终端） */
  .alogs-out {
    flex-shrink: 0;
    max-height: 160px;
    overflow: auto;
    scrollbar-gutter: stable;
    margin: 0;
    padding: 6px 10px;
    font-family: ui-monospace, Consolas, 'Courier New', monospace;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--jc-text-secondary);
    background: var(--jc-bg-input, #2b2b2e);
    border-bottom: 1px solid var(--jc-border-color);
    white-space: pre-wrap;
    word-break: break-all;
  }
  .alogs-runlog-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px 10px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
}
/* 实时日志行（运行时浮动 + 共用） */
.alogs-ll {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px 8px;
  font-size: 11px;
  padding: 4px 6px;
  border-radius: 4px;
  background: var(--jc-bg-elevated);
  &.fail { outline: 1px solid rgba(255, 77, 79, .35); }
  .alogs-ll-name { color: var(--jc-text-primary); font-weight: 500; }
  .alogs-ll-detail { color: var(--jc-text-secondary); flex-basis: 100%; }
  .alogs-ll-auth { color: var(--jc-color-warning); }
  .alogs-ll-code,
  .alogs-ll-dur { color: var(--jc-text-tertiary); }
  .alogs-ll-out { flex-basis: 100%; color: var(--jc-text-secondary); word-break: break-all; }
}
.alogs-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 10px;
  display: flex;
  flex-direction: column;
}
.alogs-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--jc-text-tertiary);
}
.alogs-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.alogs-run {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  border: 1px solid transparent;
  &:hover { background: var(--jc-bg-hover); }
  &.on { border-color: var(--jc-color-accent); background: var(--jc-bg-hover); }
  .alogs-run-status {
    font-size: 11px;
    padding: 1px 8px;
    border-radius: 10px;
    flex-shrink: 0;
    &.ok { background: rgba(82,196,26,.15); color: var(--jc-color-success); }
    &.err { background: rgba(255,77,79,.15); color: var(--jc-color-error); }
    &.stop { background: rgba(250,173,20,.15); color: var(--jc-color-warning); }
  }
  .alogs-run-main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .alogs-run-name { font-size: 12px; color: var(--jc-text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .alogs-run-meta { font-size: 11px; color: var(--jc-text-tertiary); }
  .alogs-run-err { font-size: 11px; color: var(--jc-color-error); max-width: 40%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
}
.alogs-detail {
  border-top: 1px solid var(--jc-border-color);
  height: 45%;
  min-height: 160px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  .alogs-detail-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 14px;
    .alogs-detail-title { font-size: 12px; font-weight: 600; color: var(--jc-text-primary); }
    .alogs-detail-meta { font-size: 11px; color: var(--jc-text-tertiary); }
  }
}
.alogs-steps {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.alogs-step {
  display: flex;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 4px 8px;
  padding: 6px 8px;
  border-radius: 6px;
  background: var(--jc-bg-elevated);
  font-size: 12px;
  &.fail { outline: 1px solid rgba(255,77,79,.35); }
  .alogs-step-dot { width: 8px; height: 8px; border-radius: 50%; margin-top: 4px; flex-shrink: 0; }
  .alogs-step-idx { color: var(--jc-text-tertiary); font-size: 11px; min-width: 22px; text-align: right; }
  .alogs-step-name { color: var(--jc-text-primary); font-weight: 500; }
  .alogs-step-status { font-size: 11px; &.fail { color: var(--jc-color-error); } }
  .alogs-step-dur { font-size: 11px; color: var(--jc-text-tertiary); }
  .alogs-step-code { font-size: 11px; color: var(--jc-text-tertiary); }
  .alogs-step-auth { font-size: 11px; color: var(--jc-color-warning); }
  .alogs-step-detail {
    flex-basis: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    .alogs-step-line {
      font-size: 11px;
      color: var(--jc-text-secondary);
      display: flex;
      gap: 6px;
      .lbl { color: var(--jc-text-tertiary); flex-shrink: 0; }
      .mono { font-family: var(--jc-font-mono, ui-monospace, monospace); word-break: break-all; }
    }
  }
}
</style>
