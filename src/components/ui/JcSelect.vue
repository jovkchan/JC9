<script setup lang="ts">
import { computed } from 'vue'

defineOptions({ name: 'JcSelect' })

export interface JcSelectOption {
  label: string
  value: string | number
  disabled?: boolean
}

export type JcSelectSize = 'large' | 'middle' | 'small'  // 对齐 antd Select.size

const props = withDefaults(
  defineProps<{
    modelValue?: string | number
    options?: JcSelectOption[]
    placeholder?: string
    disabled?: boolean
    size?: JcSelectSize
    title?: string
  }>(),
  {
    modelValue: undefined,
    options: () => [],
    placeholder: '',
    disabled: false,
    size: 'middle',
    title: '',
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  change: [value: string | number, e: Event]
}>()

const classes = computed(() => ['jc-select', `jc-select--${props.size}`])

function onChange(e: Event) {
  const el = e.target as HTMLSelectElement
  emit('update:modelValue', el.value)
  emit('change', el.value, e)
}
</script>

<template>
  <span :class="classes" :title="title">
    <select
      :value="modelValue"
      :disabled="disabled"
      class="jc-select__inner"
      @change="onChange"
    >
      <option v-if="placeholder" value="" disabled>{{ placeholder }}</option>
      <option
        v-for="opt in options"
        :key="String(opt.value)"
        :value="opt.value"
        :disabled="opt.disabled"
      >
        {{ opt.label }}
      </option>
    </select>
    <span class="jc-select__arrow" aria-hidden="true">▾</span>
  </span>
</template>

<style scoped>
.jc-select {
  position: relative;
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
}
.jc-select__inner {
  appearance: none;
  -webkit-appearance: none;
  width: 100%;
  font-family: inherit;
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  color: var(--jc-text-primary, #ccc);
  outline: none;
  cursor: pointer;
  transition: border-color 120ms ease, box-shadow 120ms ease;
}
.jc-select__inner:focus {
  border-color: var(--jc-color-accent, #8a58ff);
  box-shadow: 0 0 0 2px var(--jc-color-accent-light-9, rgba(138, 88, 255, 0.15));
}
.jc-select__inner:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.jc-select__inner option {
  background: var(--jc-bg-elevated, #2d2d30);
  color: var(--jc-text-primary, #ccc);
}

.jc-select--small .jc-select__inner { height: var(--jc-control-height-sm, 24px); padding: 0 24px 0 8px; font-size: var(--jc-font-size-sm, 12px); }
.jc-select--middle .jc-select__inner { height: var(--jc-control-height, 28px); padding: 0 26px 0 10px; font-size: var(--jc-font-size-control, 12px); }
.jc-select--large .jc-select__inner { height: var(--jc-control-height-lg, 36px); padding: 0 28px 0 12px; font-size: var(--jc-font-size-lg, 14px); }

.jc-select__arrow {
  position: absolute;
  right: 8px;
  pointer-events: none;
  font-size: 10px;
  color: var(--jc-text-secondary, #858585);
}
</style>
