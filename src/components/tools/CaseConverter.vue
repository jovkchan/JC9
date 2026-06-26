<script setup lang="ts">
import { ref, watch } from 'vue'

const input = ref('user_name')
const output = ref('')
const convertType = ref<'camel' | 'pascal' | 'snake' | 'kebab' | 'constant'>('camel')

function toWords(str: string): string[] {
  // 先在驼峰位置加空格断词，再把中下划线转空格，最后拆分
  const s = str
    .replace(/([A-Z])/g, ' $1')
    .replace(/[-_]/g, ' ')
    .trim()
  return s.toLowerCase().split(/\s+/).filter(Boolean)
}

function convertValue(str: string, type: typeof convertType.value): string {
  const words = toWords(str)
  if (words.length === 0) return ''

  switch (type) {
    case 'camel':
      return words
        .map((w, i) => i === 0 ? w : w.charAt(0).toUpperCase() + w.slice(1))
        .join('')
    case 'pascal':
      return words
        .map(w => w.charAt(0).toUpperCase() + w.slice(1))
        .join('')
    case 'snake':
      return words.join('_')
    case 'kebab':
      return words.join('-')
    case 'constant':
      return words.map(w => w.toUpperCase()).join('_')
    default:
      return str
  }
}

function processConvert() {
  if (!input.value) {
    output.value = ''
    return
  }
  
  // 支持按行批量处理，适应 DTO 字段批量改写
  const lines = input.value.split('\n')
  const converted = lines.map(line => {
    // 提取可能的前后空白，仅对核心词汇进行格式转换，保留代码行的缩进
    const match = line.match(/^(\s*)(.*?)(\s*)$/)
    if (match) {
      const leadingSpace = match[1]
      const coreWord = match[2]
      const trailingSpace = match[3]
      return leadingSpace + convertValue(coreWord, convertType.value) + trailingSpace
    }
    return convertValue(line, convertType.value)
  })
  
  output.value = converted.join('\n')
}

watch([input, convertType], () => {
  processConvert()
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
      <div class="tool-title">命名风格转换器 (Case Converter)</div>
      <div class="tool-actions">
        <label>转换风格：</label>
        <select v-model="convertType" class="tool-select">
          <option value="camel">userName (小驼峰)</option>
          <option value="pascal">UserName (大驼峰/帕斯卡)</option>
          <option value="snake">user_name (下划线/蛇形)</option>
          <option value="kebab">user-name (中划线/烤串)</option>
          <option value="constant">USER_NAME (常量大写)</option>
        </select>
        <button class="tool-btn pri" @click="copyResult" :disabled="!output">复制结果</button>
        <button class="tool-btn err" @click="clearAll">清空</button>
      </div>
    </div>
    
    <div class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label">输入原始文本 (支持多行批量)</div>
        <textarea v-model="input" placeholder="输入要转换风格的变量名，如 user_name 或 userName..." spellcheck="false"></textarea>
      </div>
      <div class="editor-pane">
        <div class="pane-label">转换后结果</div>
        <textarea v-model="output" readonly placeholder="等待转换..." spellcheck="false" class="readonly-output code-font"></textarea>
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
  align-items: center;
  gap: 8px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}
.tool-select {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 3px 6px;
  font-size: 11px;
  outline: none;
  border-radius: 2px;
  cursor: pointer;
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
textarea {
  flex: 1;
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
    font-size: 12px;
  }
}
.readonly-output {
  background: var(--jc-bg-app);
  color: var(--jc-color-success);
}
</style>
