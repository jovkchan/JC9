<script setup lang="ts">
import { ref, watch } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const input = ref('data_10\ndata_2\ndata_1\ndata_2\n   \ndata_5')
const output = ref('')
const delimiterMode = ref<'split' | 'join'>('join')

// 拆分与合并参数
const splitChar = ref(',')
const joinChar = ref(', ')
const prefixChar = ref("'")
const suffixChar = ref("'")

function getLines(): string[] {
  if (!input.value) return []
  return input.value.split('\n')
}

// 1. 基础处理：去除首尾空格
function trimLines() {
  const lines = getLines()
  input.value = lines.map(line => line.trim()).join('\n')
  processLines()
}

// 2. 基础处理：去除空行
function removeEmptyLines() {
  const lines = getLines()
  input.value = lines.filter(line => line.trim() !== '').join('\n')
  processLines()
}

// 3. 基础处理：去除重复行
function removeDuplicateLines() {
  const lines = getLines()
  const unique = [...new Set(lines)]
  input.value = unique.join('\n')
  processLines()
}

// 4. 排序操作
function sortLines(direction: 'asc' | 'desc' | 'natural' | 'shuffle') {
  let lines = getLines()
  if (lines.length === 0) return

  if (direction === 'asc') {
    lines.sort()
  } else if (direction === 'desc') {
    lines.sort().reverse()
  } else if (direction === 'natural') {
    lines.sort((a, b) => a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' }))
  } else if (direction === 'shuffle') {
    // Fisher-Yates 随机洗牌算法
    for (let i = lines.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1))
      const temp = lines[i]
      lines[i] = lines[j]
      lines[j] = temp
    }
  }

  input.value = lines.join('\n')
  processLines()
}

// 5. 拆分与合并核心逻辑
function processLines() {
  const lines = getLines()
  if (lines.length === 0) {
    output.value = ''
    return
  }

  if (delimiterMode.value === 'join') {
    // 合并多行为单行，并加前后缀与连接符（如 SQL IN）
    const formatted = lines.map(line => {
      return prefixChar.value + line + suffixChar.value
    })
    output.value = formatted.join(joinChar.value)
  } else {
    // 按指定字符拆分单行文本为多行
    const text = input.value
    const separator = splitChar.value || ','
    // 处理可能的转义，如 \n, \t
    let finalSep = separator
    if (separator === '\\n') finalSep = '\n'
    if (separator === '\\t') finalSep = '\t'
    
    const parts = text.split(finalSep)
    output.value = parts.map(p => p.trim()).filter(p => p !== '').join('\n')
  }
}

// 双向响应
watch([input, delimiterMode, splitChar, joinChar, prefixChar, suffixChar], () => {
  processLines()
}, { immediate: true })

function copyResult() {
  if (!output.value) return
  navigator.clipboard.writeText(output.value)
}

function clearAll() {
  input.value = ''
  output.value = ''
}
</script>

<template>
  <ToolShell title="文本行操作器" subtitle="Text Lines Editor" split>
    <template #actions>
      <JcButton type="primary" size="small" :disabled="!output" @click="copyResult">复制结果</JcButton>
      <JcButton danger size="small" @click="clearAll">清空</JcButton>
    </template>

    <template #left-label>
      <div class="pane-label-row">
        <span>输入源文本 (每行一条数据)</span>
        <div class="quick-actions">
          <JcButton size="small" @click="trimLines">修剪首尾</JcButton>
          <JcButton size="small" @click="removeEmptyLines">去空行</JcButton>
          <JcButton size="small" @click="removeDuplicateLines">去重复</JcButton>
          <JcButton size="small" @click="sortLines('asc')">升序</JcButton>
          <JcButton size="small" @click="sortLines('desc')">降序</JcButton>
          <JcButton size="small" @click="sortLines('natural')">自然序</JcButton>
          <JcButton size="small" @click="sortLines('shuffle')">打乱</JcButton>
        </div>
      </div>
    </template>
    <template #left>
      <JcTextarea v-model="input" mono :spellcheck="false" class="jc-fill" placeholder="请在此粘贴或输入需要操作的多行数据..." />
    </template>

    <template #right-label>输出处理结果</template>
    <template #right>
      <div class="setting-section">
        <div class="section-subtitle">拆分与合并转换</div>
        <div class="tab-choice-row">
          <label class="radio-label">
            <input type="radio" value="join" v-model="delimiterMode" />
            <span>多行合并为单行</span>
          </label>
          <label class="radio-label">
            <input type="radio" value="split" v-model="delimiterMode" />
            <span>单行拆分为多行</span>
          </label>
        </div>

        <div v-if="delimiterMode === 'join'" class="sub-config-grid">
          <div class="field">
            <label>前缀 (如 ')</label>
            <JcInput v-model="prefixChar" size="small" />
          </div>
          <div class="field">
            <label>后缀 (如 ')</label>
            <JcInput v-model="suffixChar" size="small" />
          </div>
          <div class="field">
            <label>连接符</label>
            <JcInput v-model="joinChar" size="small" placeholder="例如 , 或 \n" />
          </div>
        </div>

        <div v-else class="sub-config-grid">
          <div class="field full-width">
            <label>拆分分隔符</label>
            <JcInput v-model="splitChar" size="small" placeholder="例如 , 或 \t" />
          </div>
        </div>
      </div>
      <JcTextarea v-model="output" mono readonly :spellcheck="false" class="jc-fill" placeholder="等待处理..." />
    </template>
  </ToolShell>
</template>

<style scoped lang="scss">
.pane-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  font-size: 11px;
  color: var(--jc-text-secondary);
  text-transform: uppercase;
}
.quick-actions {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
.pane-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 8px;
  text-transform: uppercase;
}
.quick-actions {
  display: flex;
  gap: 4px;
}
.setting-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex-shrink: 0;
}
.section-subtitle {
  font-size: 11px;
  font-weight: 700;
  color: var(--jc-text-primary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-left: 2px solid var(--jc-color-accent);
  padding-left: 6px;
  line-height: 1.2;
}

.tab-choice-row {
  display: flex;
  gap: 16px;
  padding: 4px 0;
}
.radio-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--jc-text-primary);
  cursor: pointer;
  input[type="radio"] {
    accent-color: var(--jc-color-accent);
    margin: 0;
  }
}

.sub-config-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    &.full-width {
      grid-column: span 3;
    }
    label {
      font-size: 10px;
      color: var(--jc-text-secondary);
    }
  }
}
</style>
