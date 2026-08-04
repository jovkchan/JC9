<script setup lang="ts">
import { ref, watch } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const input = ref('user_name')
const output = ref('')
const convertType = ref<'camel' | 'pascal' | 'snake' | 'kebab' | 'constant'>('camel')

const convertTypeOptions = [
  { label: 'userName (小驼峰)', value: 'camel' },
  { label: 'UserName (大驼峰)', value: 'pascal' },
  { label: 'user_name (蛇形)', value: 'snake' },
  { label: 'user-name (烤串)', value: 'kebab' },
  { label: 'USER_NAME (常量)', value: 'constant' },
]

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
  <ToolShell title="命名风格转换器" subtitle="Case Converter" split>
    <template #actions>
      <JcSelect beam glow v-model="convertType" :options="convertTypeOptions" size="small" />
      <JcButton type="primary" size="small" :disabled="!output" @click="copyResult">复制结果</JcButton>
      <JcButton size="small" danger ghost @click="clearAll">清空</JcButton>
    </template>
    <template #left-label>输入原始文本 (支持多行批量)</template>
    <template #left>
      <JcTextarea v-model="input" mono beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="输入要转换风格的变量名，如 user_name 或 userName..." />
    </template>
    <template #right-label>转换后结果</template>
    <template #right>
      <JcTextarea v-model="output" mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="等待转换..." />
    </template>
  </ToolShell>
</template>


