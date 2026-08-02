<script setup lang="ts">
import JcRadio from './JcRadio.vue'

defineOptions({ name: 'JcRadioGroup' })

// API 对齐 Ant Design Radio.Group：options / value / disabled / vertical / name
export interface JcRadioOption {
  label: string
  value: string | number
  disabled?: boolean
}

withDefaults(
  defineProps<{
    modelValue?: string | number
    options?: JcRadioOption[]
    disabled?: boolean
    name?: string
    vertical?: boolean
  }>(),
  {
    modelValue: undefined,
    options: () => [],
    disabled: false,
    name: '',
    vertical: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  change: [value: string | number]
}>()

function select(v: string | number) {
  emit('update:modelValue', v)
  emit('change', v)
}
</script>

<template>
  <div :class="['jc-radio-group', { 'is-vertical': vertical }]" role="radiogroup">
    <JcRadio
      v-for="opt in options"
      :key="String(opt.value)"
      :checked="modelValue === opt.value"
      :disabled="disabled || opt.disabled"
      :name="name"
      @change="select(opt.value)"
    >
      {{ opt.label }}
    </JcRadio>
  </div>
</template>

<style scoped>
.jc-radio-group {
  display: inline-flex;
  align-items: center;
  gap: var(--jc-space, 16px);
  flex-wrap: wrap;
}
.jc-radio-group.is-vertical {
  flex-direction: column;
  align-items: flex-start;
  gap: var(--jc-space-sm, 12px);
}
</style>
