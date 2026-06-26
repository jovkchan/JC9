<script setup lang="ts">
import { ref, watch } from 'vue'

const mode = ref<'escape' | 'unescape'>('escape')
const input = ref('')
const output = ref('')
const errorMsg = ref('')

// 转义特殊字符，保护 HTML
function htmlEscape(str: string): string {
  return str.replace(/[&<>"']/g, c => {
    switch (c) {
      case '&': return '&amp;'
      case '<': return '&lt;'
      case '>': return '&gt;'
      case '"': return '&quot;'
      case "'": return '&#39;'
      default: return c
    }
  })
}

// 借用原生 DOM 引擎解析器实现 HTML5 完美反转义
// 能够 100% 还原任何命名实体、十进制和十六进制数字实体（如 &apos;, &#x27;, &#38; 等）
function htmlUnescape(str: string): string {
  const parser = new DOMParser()
  const doc = parser.parseFromString(str, 'text/html')
  return doc.documentElement.textContent || ''
}

function processText() {
  errorMsg.value = ''
  if (!input.value) {
    output.value = ''
    return
  }

  try {
    if (mode.value === 'escape') {
      output.value = htmlEscape(input.value)
    } else {
      output.value = htmlUnescape(input.value)
    }
  } catch (e: any) {
    errorMsg.value = '处理失败: ' + (e.message || '格式错误')
    output.value = ''
  }
}

watch([input, mode], () => {
  processText()
})

function copyResult() {
  if (!output.value) return
  navigator.clipboard.writeText(output.value)
}

function clearAll() {
  input.value = ''
  output.value = ''
  errorMsg.value = ''
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">HTML 实体转义 / 反转义</div>
      <div class="tool-actions">
        <label>模式：</label>
        <select v-model="mode" class="tool-select">
          <option value="escape">转义 (Escape)</option>
          <option value="unescape">反转义 (Unescape)</option>
        </select>
        <button class="tool-btn pri" @click="copyResult" :disabled="!output">复制结果</button>
        <button class="tool-btn err" @click="clearAll">清空</button>
      </div>
    </div>
    <div class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label">输入文本 / 实体代码</div>
        <textarea v-model="input" placeholder="在此粘贴需要转义的 HTML，或者粘贴需要解析反转义的 &amp;lt;div&amp;gt; 实体字符..." spellcheck="false"></textarea>
      </div>
      <div class="editor-pane">
        <div class="pane-label">结果展示</div>
        <textarea v-model="output" readonly placeholder="等待转换..." spellcheck="false" class="readonly-output"></textarea>
      </div>
    </div>
    <div v-if="errorMsg" class="tool-footer-error">{{ errorMsg }}</div>
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
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  padding: 8px;
  outline: none;
  border-radius: 2px;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.readonly-output {
  background: var(--jc-bg-app);
  color: var(--jc-color-success);
}
.tool-footer-error {
  flex-shrink: 0;
  margin-top: 8px;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>
