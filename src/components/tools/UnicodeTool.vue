<script setup lang="ts">
import { ref, watch } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

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
  <ToolShell title="Unicode / ASCII 转换">
    <template #actions>
      <JcButton size="small" :type="activeTab === 'encode' ? 'primary' : 'default'" @click="activeTab = 'encode'">转为编码 (Encode)</JcButton>
      <JcButton size="small" :type="activeTab === 'decode' ? 'primary' : 'default'" @click="activeTab = 'decode'">还原解码 (Decode)</JcButton>
    </template>

    <div v-if="activeTab === 'encode'" class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label-row">
          <span>原始中文/字符文本</span>
          <JcButton danger size="small" @click="clearAll">清空</JcButton>
        </div>
        <JcTextarea v-model="inputText" mono beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="输入任意中文字符、特殊符号或 Emoji 图像..." />
      </div>

      <div class="results-display-pane">
        <div class="result-item-group">
          <div class="result-lbl-bar">
            <span>Unicode 逃逸串 (\uXXXX)</span>
            <JcButton type="primary" size="small" :disabled="!unicodeEscapeResult" @click="copyResult(unicodeEscapeResult)">复制</JcButton>
          </div>
          <JcTextarea mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" :model-value="unicodeEscapeResult" :rows="3" placeholder="转换结果..." />
        </div>

        <div class="result-item-group">
          <div class="result-lbl-bar">
            <span>HTML 十六进制实体 (&#xXXXX;)</span>
            <JcButton type="primary" size="small" :disabled="!htmlHexResult" @click="copyResult(htmlHexResult)">复制</JcButton>
          </div>
          <JcTextarea mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" :model-value="htmlHexResult" :rows="3" placeholder="转换结果..." />
        </div>

        <div class="result-item-group">
          <div class="result-lbl-bar">
            <span>HTML 十进制实体 (&#DDDD;)</span>
            <JcButton type="primary" size="small" :disabled="!htmlDecimalResult" @click="copyResult(htmlDecimalResult)">复制</JcButton>
          </div>
          <JcTextarea mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" :model-value="htmlDecimalResult" :rows="3" placeholder="转换结果..." />
        </div>
      </div>
    </div>

    <div v-else class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label-row">
          <span>转义编码输入</span>
          <JcButton danger size="small" @click="clearAll">清空</JcButton>
        </div>
        <JcTextarea v-model="inputText" mono beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="粘贴含有 \u4f60\u597d 或 &#30028; 等转义格式的字符串..." />
      </div>
      <div class="editor-pane">
        <div class="pane-label-row">
          <span>还原文本结果</span>
          <JcButton type="primary" size="small" :disabled="!decodedResult" @click="copyResult(decodedResult)">复制结果</JcButton>
        </div>
        <JcTextarea v-model="decodedResult" mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="解码还原内容..." />
      </div>
    </div>
  </ToolShell>
</template>

<style scoped lang="scss">
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
</style>
