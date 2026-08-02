<script setup lang="ts">
import JcCheckbox from './JcCheckbox.vue'

defineOptions({ name: 'JcCheckboxGroup' })

// API 对齐 Ant Design Checkbox.Group：options / value(数组) / vertical
export interface JcCheckboxOption {
  label: string
  value: string | number
  disabled?: boolean
}

const props = withDefaults(
  defineProps<{
    modelValue?: Array<string | number>
    options?: JcCheckboxOption[]
    disabled?: boolean
    vertical?: boolean
  }>(),
  {
    modelValue: () => [],
    options: () => [],
    disabled: false,
    vertical: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: Array<string | number>]
  change: [value: Array<string | number>]
}>()

function toggle(v: string | number) {
  const cur = props.modelValue
  const next = cur.includes(v) ? cur.filter((x) => x !== v) : [...cur, v]
  emit('update:modelValue', next)
  emit('change', next)
}
</script>

<template>
  <div :class="['jc-checkbox-group', { 'is-vertical': vertical }]" role="group">
    <JcCheckbox
      v-for="opt in options"
      :key="String(opt.value)"
      :checked="modelValue.includes(opt.value)"
      :disabled="disabled || opt.disabled"
      @change="toggle(opt.value)"
    >
      {{ opt.label }}
    </JcCheckbox>
  </div>
</template>

<style scoped>
.jc-checkbox-group {
  display: inline-flex;
  align-items: center;
  gap: var(--jc-space, 16px);
  flex-wrap: wrap;
}
.jc-checkbox-group.is-vertical {
  flex-direction: column;
  align-items: flex-start;
  gap: var(--jc-space-sm, 12px);
}
</style>
