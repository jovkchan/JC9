<script setup lang="ts">
import { ref } from 'vue'

const input = ref('')
const output = ref('')
const errorMsg = ref('')
const indentSize = ref(2)

function formatJson() {
  errorMsg.value = ''
  if (!input.value.trim()) {
    output.value = ''
    return
  }
  try {
    const parsed = JSON.parse(input.value)
    output.value = JSON.stringify(parsed, null, indentSize.value)
  } catch (e: any) {
    errorMsg.value = e.message || 'JSON 格式非法'
    output.value = ''
  }
}

function minifyJson() {
  errorMsg.value = ''
  if (!input.value.trim()) {
    output.value = ''
    return
  }
  try {
    const parsed = JSON.parse(input.value)
    output.value = JSON.stringify(parsed)
  } catch (e: any) {
    errorMsg.value = e.message || 'JSON 格式非法'
    output.value = ''
  }
}

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
      <div class="tool-title">JSON 格式化器</div>
      <div class="tool-actions">
        <label>缩进：</label>
        <select v-model="indentSize" @change="formatJson" class="tool-select">
          <option :value="2">2 空格</option>
          <option :value="4">4 空格</option>
        </select>
        <button class="tool-btn pri" @click="formatJson">格式化</button>
        <button class="tool-btn" @click="minifyJson">压缩 (Minify)</button>
        <button class="tool-btn" @click="copyResult" :disabled="!output">复制结果</button>
        <button class="tool-btn err" @click="clearAll">清空</button>
      </div>
    </div>
    <div class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label">原始 JSON</div>
        <textarea v-model="input" @input="formatJson" placeholder="在此粘贴 JSON 文本..." spellcheck="false"></textarea>
      </div>
      <div class="editor-pane">
        <div class="pane-label">格式化结果</div>
        <textarea v-model="output" readonly placeholder="等待格式化..." spellcheck="false" class="readonly-output"></textarea>
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
}
.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 4px 12px;
  font-size: 11px;
  cursor: pointer;
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
