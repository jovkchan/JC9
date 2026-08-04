<script setup lang="ts">
import { ref, watch } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const mode = ref<'escape' | 'unescape'>('escape')
const input = ref('')
const output = ref('')
const errorMsg = ref('')

const modeOptions = [
  { label: '转义 (Escape)', value: 'escape' },
  { label: '反转义 (Unescape)', value: 'unescape' },
]

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
  <ToolShell title="HTML 实体转义 / 反转义" subtitle="Escape / Unescape" split>
    <template #actions>
      <JcSelect beam glow v-model="mode" :options="modeOptions" size="small" />
      <JcButton type="primary" size="small" :disabled="!output" @click="copyResult">复制结果</JcButton>
      <JcButton size="small" danger ghost @click="clearAll">清空</JcButton>
    </template>
    <template #left-label>输入文本 / 实体代码</template>
    <template #left>
      <JcTextarea v-model="input" mono beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="在此粘贴需要转义的 HTML，或者粘贴需要解析反转义的 &amp;lt;div&amp;gt; 实体字符..." />
    </template>
    <template #right-label>结果展示</template>
    <template #right>
      <div class="ht-right">
        <JcTextarea v-model="output" mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="等待转换..." />
        <div v-if="errorMsg" class="ht-error">{{ errorMsg }}</div>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped>
.ht-right { display: flex; flex-direction: column; gap: 8px; flex: 1; min-height: 0; }
.ht-error {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>


