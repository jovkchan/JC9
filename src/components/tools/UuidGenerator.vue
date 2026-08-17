<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInputNumber from '@/components/ui/JcInputNumber.vue'
import JcCheckbox from '@/components/ui/JcCheckbox.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const count = ref(5)
const hyphens = ref(true)
const uppercase = ref(false)
const output = ref('')

function generateUuid() {
  const list: string[] = []
  for (let i = 0; i < count.value; i++) {
    // 纯前端产生 UUID v4 (对齐 crypto.randomUUID() 支持)
    let uuid = ''
    try {
      uuid = crypto.randomUUID()
    } catch {
      // 降级后备算法
      uuid = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
        const r = (Math.random() * 16) | 0
        const v = c === 'x' ? r : (r & 0x3) | 0x8
        return v.toString(16)
      })
    }

    if (!hyphens.value) {
      uuid = uuid.replace(/-/g, '')
    }
    if (uppercase.value) {
      uuid = uuid.toUpperCase()
    } else {
      uuid = uuid.toLowerCase()
    }
    list.push(uuid)
  }
  output.value = list.join('\n')
}

function copyAll() {
  if (!output.value) return
  navigator.clipboard.writeText(output.value)
}

function clearAll() {
  output.value = ''
}
</script>

<template>
  <ToolShell title="UUID 生成器" subtitle="UUID v4" split>
    <template #actions>
      <JcButton type="primary" size="small" @click="generateUuid">批量生成</JcButton>
      <JcButton size="small" :disabled="!output" @click="copyAll">复制全部</JcButton>
      <JcButton size="small" danger ghost :disabled="!output" @click="clearAll">清空</JcButton>
    </template>
    <template #left-label>生成选项</template>
    <template #left>
      <div class="uuid-form">
        <div class="uuid-form__row">
          <label class="uuid-form__label">生成数量</label>
          <JcInputNumber :model-value="count" :min="1" :max="1000" size="small" beam glow class="uuid-form__num" @update:model-value="count = Math.min(1000, Math.max(1, $event ?? 1))" />
        </div>
        <JcCheckbox v-model:checked="hyphens">保留连字符 (-)</JcCheckbox>
        <JcCheckbox v-model:checked="uppercase">大写格式</JcCheckbox>
      </div>
    </template>
    <template #right-label>生成的 UUID 列表</template>
    <template #right>
      <JcTextarea v-model="output" mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="等待生成..." />
    </template>
  </ToolShell>
</template>

<style scoped>
.uuid-form { display: flex; flex-direction: column; gap: 12px; }
.uuid-form__row { display: flex; align-items: center; gap: 8px; }
.uuid-form__label { font-size: 11px; color: var(--jc-text-secondary); text-transform: uppercase; }
.uuid-form__num { width: 90px; }
</style>
