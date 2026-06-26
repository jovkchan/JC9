<script setup lang="ts">
import { ref, watch } from 'vue'

const activeTab = ref<'encode' | 'decode'>('encode')
const inputText = ref('')

// 编码输出结果
const unicodeEscapeResult = ref('')
const htmlDecimalResult = ref('')
const htmlHexResult = ref('')

// 解码输出结果
const decodedResult = ref('')

// 万能编码逻辑
function encodeText() {
  const val = inputText.value
  if (!val) {
    unicodeEscapeResult.value = ''
    htmlDecimalResult.value = ''
    htmlHexResult.value = ''
    return
  }

  // 1. Unicode 16位逃逸序列（传统 \uXXXX，包括代理对）
  let escapeStr = ''
  for (let i = 0; i < val.length; i++) {
    escapeStr += '\\u' + val.charCodeAt(i).toString(16).padStart(4, '0')
  }
  unicodeEscapeResult.value = escapeStr

  // 2. HTML 十进制实体（支持大字集码点）
  htmlDecimalResult.value = [...val].map(c => `&#${c.codePointAt(0)};`).join('')

  // 3. HTML 十六进制实体（支持大字集码点）
  htmlHexResult.value = [...val].map(c => `&#x${c.codePointAt(0)!.toString(16)};`).join('')
}

// 万能解码逻辑
function decodeText() {
  const val = inputText.value
  if (!val) {
    decodedResult.value = ''
    return
  }

  let res = val
  // 1. 解析 HTML 十六进制实体 &#xXXXX;
  res = res.replace(/&#x([0-9a-fA-F]+);/g, (_, hex) => {
    try { return String.fromCodePoint(parseInt(hex, 16)) } catch { return _ }
  })
  // 2. 解析 HTML 十进制实体 &#DDDD;
  res = res.replace(/&#(\d+);/g, (_, dec) => {
    try { return String.fromCodePoint(parseInt(dec, 10)) } catch { return _ }
  })
  // 3. 解析大括号型 Unicode 逃逸 \u{XXXX}
  res = res.replace(/\\u\{([0-9a-fA-F]+)\}/g, (_, hex) => {
    try { return String.fromCodePoint(parseInt(hex, 16)) } catch { return _ }
  })
  // 4. 解析标准 \uXXXX 逃逸
  res = res.replace(/\\u([0-9a-fA-F]{4})/g, (_, hex) => {
    try { return String.fromCharCode(parseInt(hex, 16)) } catch { return _ }
  })

  decodedResult.value = res
}

watch([inputText, activeTab], () => {
  if (activeTab.value === 'encode') {
    encodeText()
  } else {
    decodeText()
  }
})

function copyResult(text: string) {
  if (!text) return
  navigator.clipboard.writeText(text)
}

function clearAll() {
  inputText.value = ''
  unicodeEscapeResult.value = ''
  htmlDecimalResult.value = ''
  htmlHexResult.value = ''
  decodedResult.value = ''
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">Unicode / ASCII 转换</div>
      <div class="tool-actions-tabs">
        <button :class="['tab-btn', { on: activeTab === 'encode' }]" @click="activeTab = 'encode'">转为编码 (Encode)</button>
        <button :class="['tab-btn', { on: activeTab === 'decode' }]" @click="activeTab = 'decode'">还原解码 (Decode)</button>
      </div>
    </div>

    <!-- Tab 1: 转为 Unicode 编码 -->
    <div v-if="activeTab === 'encode'" class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label-row">
          <span>原始中文/字符文本</span>
          <button class="tool-btn err small" @click="clearAll">清空</button>
        </div>
        <textarea v-model="inputText" placeholder="输入任意中文字符、特殊符号或 Emoji 图像..." spellcheck="false"></textarea>
      </div>

      <div class="results-display-pane">
        <div class="result-item-group">
          <div class="result-lbl-bar">
            <span>Unicode 逃逸串 (\uXXXX)</span>
            <button class="tool-btn pri small" @click="copyResult(unicodeEscapeResult)" :disabled="!unicodeEscapeResult">复制</button>
          </div>
          <textarea readonly class="readonly-output-box" :value="unicodeEscapeResult" placeholder="转换结果..."></textarea>
        </div>

        <div class="result-item-group">
          <div class="result-lbl-bar">
            <span>HTML 十六进制实体 (&#xXXXX;)</span>
            <button class="tool-btn pri small" @click="copyResult(htmlHexResult)" :disabled="!htmlHexResult">复制</button>
          </div>
          <textarea readonly class="readonly-output-box" :value="htmlHexResult" placeholder="转换结果..."></textarea>
        </div>

        <div class="result-item-group">
          <div class="result-lbl-bar">
            <span>HTML 十进制实体 (&#DDDD;)</span>
            <button class="tool-btn pri small" @click="copyResult(htmlDecimalResult)" :disabled="!htmlDecimalResult">复制</button>
          </div>
          <textarea readonly class="readonly-output-box" :value="htmlDecimalResult" placeholder="转换结果..."></textarea>
        </div>
      </div>
    </div>

    <!-- Tab 2: 还原解码 -->
    <div v-else class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label-row">
          <span>转义编码输入</span>
          <button class="tool-btn err small" @click="clearAll">清空</button>
        </div>
        <textarea v-model="inputText" placeholder="粘贴含有 \u4f60\u597d 或 &#30028; 等转义格式的字符串..." spellcheck="false"></textarea>
      </div>
      <div class="editor-pane">
        <div class="pane-label-row">
          <span>还原文本结果</span>
          <button class="tool-btn pri small" @click="copyResult(decodedResult)" :disabled="!decodedResult">复制结果</button>
        </div>
        <textarea v-model="decodedResult" readonly placeholder="解码还原内容..." spellcheck="false" class="readonly-output"></textarea>
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
.tool-actions-tabs {
  display: flex;
  background: var(--jc-bg-elevated);
  border-radius: 4px;
  padding: 2px;
  border: 1px solid var(--jc-border-default);
}
.tab-btn {
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  padding: 4px 12px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 3px;
  transition: all 0.2s;
  &.on {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
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
.pane-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
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
.results-display-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
}
.result-item-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.result-lbl-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
}
.readonly-output-box {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 11px;
  height: 60px;
  resize: none;
  padding: 6px;
  outline: none;
  border-radius: 3px;
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
  &.small {
    padding: 2px 8px;
    font-size: 10px;
  }
}
</style>
