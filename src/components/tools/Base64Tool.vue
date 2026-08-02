<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSegmented from '@/components/ui/JcSegmented.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const mode = ref<'encode' | 'decode'>('encode')
const input = ref('')
const output = ref('')
const errorMsg = ref('')

const modeOptions = [
  { label: '文字 ➔ Base64', value: 'encode' },
  { label: 'Base64 ➔ 文字', value: 'decode' },
]

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
  <ToolShell title="Base64 转换器" subtitle="文字 ↔ Base64" split>
    <template #actions>
      <JcSegmented
        :model-value="mode"
        :options="modeOptions"
        size="small"
        @update:model-value="(v) => switchMode(v as 'encode' | 'decode')"
      />
      <JcButton type="primary" size="small" @click="processText">转换</JcButton>
      <JcButton size="small" :disabled="!output" @click="copyResult">复制结果</JcButton>
      <JcButton size="small" danger ghost @click="clearAll">清空</JcButton>
    </template>

    <template #left-label>{{ mode === 'encode' ? '原始内容 (文本)' : 'Base64 编码串' }}</template>
    <template #left>
      <JcTextarea
        v-model="input"
        mono
        :spellcheck="false"
        class="jc-fill"
        :placeholder="mode === 'encode' ? '在此输入普通文本...' : '在此粘贴 Base64 串...'"
        @input="processText"
      />
    </template>

    <template #right-label>{{ mode === 'encode' ? 'Base64 编码串' : '恢复文本 (原始内容)' }}</template>
    <template #right>
      <div class="base64-right">
        <JcTextarea
          v-model="output"
          mono
          readonly
          :spellcheck="false"
          class="jc-fill"
          :placeholder="mode === 'encode' ? '等待编码...' : '等待解码...'"
        />
        <div v-if="errorMsg" class="base64-error">{{ errorMsg }}</div>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped>
.base64-right {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
  min-height: 0;
}
.base64-error {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>
