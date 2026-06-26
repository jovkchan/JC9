<script setup lang="ts">
import { ref, watch } from 'vue'

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
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">文本行操作器 (Text Lines Editor)</div>
      <div class="tool-actions">
        <button class="tool-btn pri" @click="copyResult" :disabled="!output">复制结果</button>
        <button class="tool-btn err" @click="clearAll">清空</button>
      </div>
    </div>

    <div class="tool-body-split">
      <!-- 左侧输入与快捷操作栏 -->
      <div class="editor-pane flex-fill">
        <div class="pane-label-row">
          <span>输入源文本 (每行一条数据)</span>
          <div class="quick-actions">
            <button class="action-btn" @click="trimLines">修剪首尾</button>
            <button class="action-btn" @click="removeEmptyLines">去空行</button>
            <button class="action-btn" @click="removeDuplicateLines">去重复</button>
            <button class="action-btn" @click="sortLines('asc')">升序</button>
            <button class="action-btn" @click="sortLines('desc')">降序</button>
            <button class="action-btn" @click="sortLines('natural')">自然序</button>
            <button class="action-btn" @click="sortLines('shuffle')">打乱</button>
          </div>
        </div>
        <textarea v-model="input" placeholder="请在此粘贴或输入需要操作的多行数据..." spellcheck="false" class="flex-grow"></textarea>
      </div>

      <!-- 右侧转换与输出控制栏 -->
      <div class="control-and-result-pane">
        <!-- 转换规则设置 -->
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

          <!-- 合并模式选项 -->
          <div v-if="delimiterMode === 'join'" class="sub-config-grid">
            <div class="field">
              <label>前缀 (如 ')</label>
              <input v-model="prefixChar" class="char-input" />
            </div>
            <div class="field">
              <label>后缀 (如 ')</label>
              <input v-model="suffixChar" class="char-input" />
            </div>
            <div class="field">
              <label>连接符</label>
              <input v-model="joinChar" class="char-input" placeholder="例如 , 或 \n" />
            </div>
          </div>

          <!-- 拆分模式选项 -->
          <div v-else class="sub-config-grid">
            <div class="field full-width">
              <label>拆分分隔符</label>
              <input v-model="splitChar" class="char-input full" placeholder="例如 , 或 \t" />
            </div>
          </div>
        </div>

        <div class="setting-section flex-fill flex flex-col mt-10">
          <div class="section-subtitle">输出处理结果</div>
          <textarea v-model="output" readonly placeholder="等待处理..." spellcheck="false" class="readonly-output result-area code-font"></textarea>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.tool-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  padding: 12px;
  background: var(--jc-bg-app);
  overflow: hidden;
}
.tool-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  flex-shrink: 0;
}
.tool-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}
.tool-actions {
  display: flex;
  gap: 8px;
}
.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 4px 12px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 2px;
  transition: all 0.2s;
  &:hover:not(:disabled) {
    background: var(--jc-bg-btn-hover);
  }
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  &.pri {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    &:hover {
      background: var(--jc-color-accent-hover);
    }
  }
  &.err {
    &:hover {
      background: var(--jc-color-error);
      color: var(--jc-color-white);
    }
  }
}
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 16px;
  min-height: 0;
}

/* 左侧输入 */
.editor-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  padding: 10px;
  border-radius: 4px;
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
.action-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: 1px solid var(--jc-border-strong);
  padding: 1px 6px;
  font-size: 10px;
  border-radius: 2px;
  cursor: pointer;
  transition: all 0.15s;
  &:hover {
    background: var(--jc-bg-btn-hover);
    border-color: var(--jc-color-accent);
  }
}
.flex-grow {
  flex: 1;
}

/* 右侧面板 */
.control-and-result-pane {
  display: flex;
  flex-direction: column;
  flex: 0 0 340px;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 14px;
  border-radius: 4px;
  gap: 12px;
}

.setting-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  &.flex-fill {
    flex: 1;
    min-height: 0;
  }
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

.char-input {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 3px 6px;
  font-size: 11px;
  outline: none;
  border-radius: 2px;
  text-align: center;
  height: 24px;
  
  &.full {
    text-align: left;
    width: 100%;
  }
}

textarea {
  width: 100%;
  resize: none;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-size: 12px;
  padding: 8px;
  outline: none;
  border-radius: 2px;
  &:focus {
    border-color: var(--jc-color-accent);
  }
  &.code-font {
    font-family: 'Cascadia Code', Consolas, monospace;
    font-size: 11px;
  }
}
.readonly-output {
  background: var(--jc-bg-app);
  color: var(--jc-color-success);
}
.result-area {
  flex: 1;
}
.flex { display: flex; }
.flex-col { flex-direction: column; }
.mt-10 { margin-top: 10px; }
</style>
