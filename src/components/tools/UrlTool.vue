<script setup lang="ts">
import { ref, watch } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const mode = ref<'encode' | 'decode'>('encode')
const input = ref('')
const output = ref('')
const errorMsg = ref('')

const modeOptions = [
  { label: '编码 (Encode)', value: 'encode' },
  { label: '解码 (Decode)', value: 'decode' },
]

function processText() {
  errorMsg.value = ''
  if (!input.value) {
    output.value = ''
    return
  }
  try {
    if (mode.value === 'encode') {
      output.value = encodeURIComponent(input.value)
    } else {
      output.value = decodeURIComponent(input.value.replace(/\+/g, '%20')) // 兼容将空格转义成+的特殊URL解码
    }
  } catch (e: any) {
    errorMsg.value = e.message || 'URL 转换失败，请检查数据格式是否合法'
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
  <ToolShell title="URL 编码 / 解码" subtitle="Encode / Decode" split>
    <template #actions>
      <JcSelect beam v-model="mode" :options="modeOptions" size="small" />
      <JcButton type="primary" size="small" :disabled="!output" @click="copyResult">复制结果</JcButton>
      <JcButton size="small" danger ghost @click="clearAll">清空</JcButton>
    </template>
    <template #left-label>原始文本</template>
    <template #left>
      <JcTextarea v-model="input" mono beam :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="粘贴需要 URL 编码或解码的文本..." />
    </template>
    <template #right-label>转换结果</template>
    <template #right>
      <div class="url-right">
        <JcTextarea v-model="output" mono readonly beam :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="等待转换..." />
        <div v-if="errorMsg" class="url-error">{{ errorMsg }}</div>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped>
.url-right { display: flex; flex-direction: column; gap: 8px; flex: 1; min-height: 0; }
.url-error {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>


