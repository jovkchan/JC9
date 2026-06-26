<script setup lang="ts">
import { ref } from 'vue'

const mode = ref<'encode' | 'decode'>('encode')
const input = ref('')
const output = ref('')
const errorMsg = ref('')

function processText() {
  errorMsg.value = ''
  if (!input.value) {
    output.value = ''
    return
  }
  try {
    if (mode.value === 'encode') {
      // 使用 btoa 并处理 UTF-8 字符集
      const bytes = new TextEncoder().encode(input.value)
      let binString = ''
      bytes.forEach(b => {
        binString += String.fromCharCode(b)
      })
      output.value = btoa(binString)
    } else {
      // 解码并处理 UTF-8
      const binString = atob(input.value.trim())
      const len = binString.length
      const bytes = new Uint8Array(len)
      for (let i = 0; i < len; i++) {
        bytes[i] = binString.charCodeAt(i)
      }
      output.value = new TextDecoder().decode(bytes)
    }
  } catch (e: any) {
    errorMsg.value = e.message || '转换错误，请检查输入字符是否为合法的 Base64 编码'
    output.value = ''
  }
}

function switchMode(newMode: 'encode' | 'decode') {
  mode.value = newMode
  const temp = input.value
  input.value = output.value
  output.value = temp
  processText()
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
      <div class="tool-title">Base64 转换器</div>
      <div class="tool-actions">
        <div class="toggle-group">
          <button :class="{ active: mode === 'encode' }" @click="switchMode('encode')">文字 ➔ Base64</button>
          <button :class="{ active: mode === 'decode' }" @click="switchMode('decode')">Base64 ➔ 文字</button>
        </div>
        <button class="tool-btn pri" @click="processText">转换</button>
        <button class="tool-btn" @click="copyResult" :disabled="!output">复制结果</button>
        <button class="tool-btn err" @click="clearAll">清空</button>
      </div>
    </div>
    <div class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label">{{ mode === 'encode' ? '原始内容 (文本)' : 'Base64 编码串' }}</div>
        <textarea v-model="input" @input="processText" :placeholder="mode === 'encode' ? '在此输入普通文本...' : '在此粘贴 Base64 串...'" spellcheck="false"></textarea>
      </div>
      <div class="editor-pane">
        <div class="pane-label">{{ mode === 'encode' ? 'Base64 编码串' : '恢复文本 (原始内容)' }}</div>
        <textarea v-model="output" readonly :placeholder="mode === 'encode' ? '等待编码...' : '等待解码...'" spellcheck="false" class="readonly-output"></textarea>
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
}
.toggle-group {
  display: flex;
  border: 1px solid var(--jc-border-strong);
  overflow: hidden;
  button {
    background: var(--jc-bg-btn);
    color: var(--jc-text-secondary);
    border: none;
    padding: 4px 10px;
    font-size: 11px;
    cursor: pointer;
    &.active {
      background: var(--jc-color-accent);
      color: var(--jc-color-white);
    }
  }
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
