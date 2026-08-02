<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const input = ref('')
const output = ref('')
const errorMsg = ref('')
const indentSize = ref(2)

const indentOptions = [
  { label: '2 空格', value: 2 },
  { label: '4 空格', value: 4 },
]

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
  <ToolShell title="JSON 格式化器" subtitle="Format / Minify" split>
    <template #actions>
      <JcSelect v-model="indentSize" :options="indentOptions" size="small" @change="formatJson" />
      <JcButton type="primary" size="small" @click="formatJson">格式化</JcButton>
      <JcButton size="small" @click="minifyJson">压缩 (Minify)</JcButton>
      <JcButton size="small" :disabled="!output" @click="copyResult">复制结果</JcButton>
      <JcButton size="small" danger ghost @click="clearAll">清空</JcButton>
    </template>
    <template #left-label>原始 JSON</template>
    <template #left>
      <JcTextarea v-model="input" mono :spellcheck="false" class="jc-fill" placeholder="在此粘贴 JSON 文本..." @input="formatJson" />
    </template>
    <template #right-label>格式化结果</template>
    <template #right>
      <div class="json-right">
        <JcTextarea v-model="output" mono readonly :spellcheck="false" class="jc-fill" placeholder="等待格式化..." />
        <div v-if="errorMsg" class="json-error">{{ errorMsg }}</div>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped>
.json-right { display: flex; flex-direction: column; gap: 8px; flex: 1; min-height: 0; }
.json-error {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>
