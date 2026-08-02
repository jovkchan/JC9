<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const originalText = ref('')
const modifiedText = ref('')
const hasCompared = ref(false)

interface DiffLine {
  type: 'equal' | 'delete' | 'insert' | 'empty'
  text: string
  lineNumber?: number
}

const leftLines = ref<DiffLine[]>([])
const rightLines = ref<DiffLine[]>([])
const compareTime = ref(0)
const totalLinesCount = ref(0)

function runDiff() {
  const startTime = performance.now()
  hasCompared.value = true

  const src = originalText.value.split(/\r?\n/)
  const dst = modifiedText.value.split(/\r?\n/)

  totalLinesCount.value = Math.max(src.length, dst.length)

  // 1. 优化：前后缀修剪，去除两端相同行，降低 DP 计算规模
  let prefixCount = 0
  while (prefixCount < src.length && prefixCount < dst.length && src[prefixCount] === dst[prefixCount]) {
    prefixCount++
  }

  let suffixCount = 0
  while (suffixCount < src.length - prefixCount && suffixCount < dst.length - prefixCount && 
         src[src.length - 1 - suffixCount] === dst[dst.length - 1 - suffixCount]) {
    suffixCount++
  }

  const midSrc = src.slice(prefixCount, src.length - suffixCount)
  const midDst = dst.slice(prefixCount, dst.length - suffixCount)

  interface DiffOp {
    type: 'equal' | 'delete' | 'insert'
    text: string
    srcLineIndex?: number
    dstLineIndex?: number
  }

  const midDiffs: DiffOp[] = []

  // 2. 如果中间差异区非常庞大（超过400万乘积），为了防卡死采用贪心退化算法，否则使用标准的 LCS
  if (midSrc.length * midDst.length > 4000000) {
    // 退化算法：按行单纯比对
    let i = 0, j = 0
    while (i < midSrc.length || j < midDst.length) {
      if (i < midSrc.length && j < midDst.length) {
        if (midSrc[i] === midDst[j]) {
          midDiffs.push({ type: 'equal', text: midSrc[i], srcLineIndex: prefixCount + i + 1, dstLineIndex: prefixCount + j + 1 })
          i++
          j++
        } else {
          midDiffs.push({ type: 'delete', text: midSrc[i], srcLineIndex: prefixCount + i + 1 })
          midDiffs.push({ type: 'insert', text: midDst[j], dstLineIndex: prefixCount + j + 1 })
          i++
          j++
        }
      } else if (i < midSrc.length) {
        midDiffs.push({ type: 'delete', text: midSrc[i], srcLineIndex: prefixCount + i + 1 })
        i++
      } else {
        midDiffs.push({ type: 'insert', text: midDst[j], dstLineIndex: prefixCount + j + 1 })
        j++
      }
    }
  } else {
    // LCS 算法
    const dp = Array.from({ length: midSrc.length + 1 }, () => new Int32Array(midDst.length + 1))
    for (let i = 1; i <= midSrc.length; i++) {
      for (let j = 1; j <= midDst.length; j++) {
        if (midSrc[i - 1] === midDst[j - 1]) {
          dp[i][j] = dp[i - 1][j - 1] + 1
        } else {
          dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1])
        }
      }
    }

    // 回溯找出编辑步骤
    let i = midSrc.length
    let j = midDst.length
    const tempOps: DiffOp[] = []

    while (i > 0 || j > 0) {
      if (i > 0 && j > 0 && midSrc[i - 1] === midDst[j - 1]) {
        tempOps.push({
          type: 'equal',
          text: midSrc[i - 1],
          srcLineIndex: prefixCount + i,
          dstLineIndex: prefixCount + j
        })
        i--
        j--
      } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
        tempOps.push({
          type: 'insert',
          text: midDst[j - 1],
          dstLineIndex: prefixCount + j
        })
        j--
      } else {
        tempOps.push({
          type: 'delete',
          text: midSrc[i - 1],
          srcLineIndex: prefixCount + i
        })
        i--
      }
    }
    midDiffs.push(...tempOps.reverse())
  }

  // 3. 组装最终的前中后合并序列
  const finalDiffs: DiffOp[] = []

  // 前缀
  for (let k = 0; k < prefixCount; k++) {
    finalDiffs.push({ type: 'equal', text: src[k], srcLineIndex: k + 1, dstLineIndex: k + 1 })
  }
  // 中间差异
  finalDiffs.push(...midDiffs)
  // 后缀
  for (let k = 0; k < suffixCount; k++) {
    const srcIdx = src.length - suffixCount + k
    const dstIdx = dst.length - suffixCount + k
    finalDiffs.push({ type: 'equal', text: src[srcIdx], srcLineIndex: srcIdx + 1, dstLineIndex: dstIdx + 1 })
  }

  // 4. 双栏对齐排版
  const left: DiffLine[] = []
  const right: DiffLine[] = []
  let k = 0

  while (k < finalDiffs.length) {
    if (finalDiffs[k].type === 'equal') {
      left.push({ type: 'equal', text: finalDiffs[k].text, lineNumber: finalDiffs[k].srcLineIndex })
      right.push({ type: 'equal', text: finalDiffs[k].text, lineNumber: finalDiffs[k].dstLineIndex })
      k++
    } else {
      // 搜集连续的删除与插入
      const deletes: DiffOp[] = []
      const inserts: DiffOp[] = []
      while (k < finalDiffs.length && finalDiffs[k].type !== 'equal') {
        if (finalDiffs[k].type === 'delete') {
          deletes.push(finalDiffs[k])
        } else {
          inserts.push(finalDiffs[k])
        }
        k++
      }

      const maxLen = Math.max(deletes.length, inserts.length)
      for (let m = 0; m < maxLen; m++) {
        if (m < deletes.length) {
          left.push({ type: 'delete', text: deletes[m].text, lineNumber: deletes[m].srcLineIndex })
        } else {
          left.push({ type: 'empty', text: '' })
        }

        if (m < inserts.length) {
          right.push({ type: 'insert', text: inserts[m].text, lineNumber: inserts[m].dstLineIndex })
        } else {
          right.push({ type: 'empty', text: '' })
        }
      }
    }
  }

  leftLines.value = left
  rightLines.value = right
  compareTime.value = parseFloat((performance.now() - startTime).toFixed(1))
}

function clearAll() {
  originalText.value = ''
  modifiedText.value = ''
  leftLines.value = []
  rightLines.value = []
  hasCompared.value = false
}
</script>

<template>
  <ToolShell title="代码差异对比" subtitle="Diff">
    <template #actions>
      <span v-if="hasCompared" class="time-stat">
        对比完成，最大行数 {{ totalLinesCount }}，耗时 {{ compareTime }} ms
      </span>
      <JcButton type="primary" size="small" @click="runDiff" :disabled="!originalText.trim() && !modifiedText.trim()">开始对比</JcButton>
      <JcButton size="small" danger ghost @click="clearAll">清空</JcButton>
    </template>

    <!-- 输入对比文本区域 -->
    <div v-if="!hasCompared" class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label">原始文本 (Original)</div>
        <JcTextarea
          v-model="originalText"
          mono
          :spellcheck="false"
          class="jc-fill"
          placeholder="在此粘贴原始的文本、代码或配置文件..."
        />
      </div>
      <div class="editor-pane">
        <div class="pane-label">修改后文本 (Modified)</div>
        <JcTextarea
          v-model="modifiedText"
          mono
          :spellcheck="false"
          class="jc-fill"
          placeholder="在此粘贴修改后的文本、代码或配置文件..."
        />
      </div>
    </div>

    <!-- 对比结果视图 -->
    <div v-else class="diff-viewer-wrap">
      <div class="diff-header-bar">
        <div class="diff-column-header">原始文件</div>
        <div class="diff-column-header">修改后文件</div>
      </div>
      <div class="diff-scroller">
        <table class="diff-table">
          <tbody>
            <tr v-for="(line, idx) in leftLines" :key="'dl-'+idx" class="diff-row">
              <!-- 左侧 (原始) -->
              <td class="diff-line-num" :class="line.type">
                {{ line.lineNumber !== undefined ? line.lineNumber : '' }}
              </td>
              <td class="diff-line-content" :class="line.type">
                <span class="diff-indicator">{{ line.type === 'delete' ? '-' : '' }}</span>
                <pre>{{ line.text }}</pre>
              </td>
              <!-- 右侧 (修改) -->
              <td class="diff-line-num" :class="rightLines[idx].type">
                {{ rightLines[idx].lineNumber !== undefined ? rightLines[idx].lineNumber : '' }}
              </td>
              <td class="diff-line-content" :class="rightLines[idx].type">
                <span class="diff-indicator">{{ rightLines[idx].type === 'insert' ? '+' : '' }}</span>
                <pre>{{ rightLines[idx].text }}</pre>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="diff-footer">
        <JcButton size="small" @click="hasCompared = false">返回编辑</JcButton>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped lang="scss">
.time-stat {
  font-size: 11px;
  color: var(--jc-text-secondary);
}
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 12px;
  min-height: 0;
}
.editor-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  padding: 8px;
  border-radius: 4px;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 6px;
  text-transform: uppercase;
}

// Diff viewer layout
.diff-viewer-wrap {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  border-radius: 4px;
  overflow: hidden;
}
.diff-header-bar {
  display: grid;
  grid-template-columns: 1fr 1fr;
  background: var(--jc-bg-elevated);
  border-bottom: 1px solid var(--jc-border-default);
  flex-shrink: 0;
}
.diff-column-header {
  padding: 6px 12px;
  font-size: 11px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  text-align: center;
  &:first-child {
    border-right: 1px solid var(--jc-border-default);
  }
}
.diff-scroller {
  flex: 1;
  overflow: auto;
}
.diff-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
}
.diff-row {
  display: grid;
  grid-template-columns: 45px 1fr 45px 1fr;
  border-bottom: 1px solid rgba(255, 255, 255, 0.02);
}
.diff-line-num {
  text-align: right;
  padding-right: 8px;
  color: var(--jc-text-secondary);
  background: var(--jc-bg-elevated);
  user-select: none;
  border-right: 1px solid var(--jc-border-default);
  line-height: 20px;
  height: 20px;

  &.delete {
    background: rgba(244, 71, 71, 0.25);
    color: #ff8888;
  }
  &.insert {
    background: rgba(78, 201, 176, 0.25);
    color: #a3ffd6;
  }
  &.empty {
    background: var(--jc-bg-elevated);
    opacity: 0.3;
  }
}
.diff-line-content {
  padding: 0 4px;
  white-space: pre;
  display: flex;
  align-items: center;
  line-height: 20px;
  height: 20px;
  overflow: hidden;
  &:nth-child(2) {
    border-right: 1px solid var(--jc-border-default);
  }

  pre {
    margin: 0;
    font-family: inherit;
    font-size: inherit;
    overflow-x: auto;
    width: 100%;
    scrollbar-width: none;
    &::-webkit-scrollbar {
      display: none;
    }
  }

  .diff-indicator {
    width: 12px;
    display: inline-block;
    color: var(--jc-text-secondary);
    user-select: none;
    flex-shrink: 0;
  }

  &.delete {
    background: rgba(244, 71, 71, 0.15);
    color: #ff8888;
    .diff-indicator {
      color: #ff5555;
    }
  }
  &.insert {
    background: rgba(78, 201, 176, 0.15);
    color: #a3ffd6;
    .diff-indicator {
      color: #4ec9b0;
    }
  }
  &.empty {
    background: rgba(255, 255, 255, 0.02);
  }
}
.diff-footer {
  padding: 8px 12px;
  border-top: 1px solid var(--jc-border-default);
  background: var(--jc-bg-elevated);
  display: flex;
  justify-content: flex-start;
  flex-shrink: 0;
}
</style>
