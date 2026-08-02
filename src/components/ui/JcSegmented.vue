<script setup lang="ts">
import { computed } from 'vue'

defineOptions({ name: 'JcSegmented' })

// API 对齐 Ant Design Segmented：options / value / onChange / size / block
// 参考: https://ant.design/components/segmented-cn
export interface JcSegmentedOption {
  label: string
  value: string | number
  icon?: string
  disabled?: boolean
}
export type JcSegmentedSize = 'large' | 'middle' | 'small'

const props = withDefaults(
  defineProps<{
    modelValue?: string | number
    options?: JcSegmentedOption[]
    size?: JcSegmentedSize
    /** 撑满父容器 */
    block?: boolean
    disabled?: boolean
  }>(),
  {
    modelValue: undefined,
    options: () => [],
    size: 'middle',
    block: false,
    disabled: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  change: [value: string | number]
}>()

const classes = computed(() => [
  'jc-segmented',
  `jc-segmented--${props.size}`,
  { 'is-block': props.block },
])

function select(opt: JcSegmentedOption) {
  if (opt.disabled || props.disabled) return
  emit('update:modelValue', opt.value)
  emit('change', opt.value)
}
</script>

<template>
  <div :class="classes" role="radiogroup">
    <button
      v-for="opt in options"
      :key="String(opt.value)"
      type="button"
      role="radio"
      :class="['jc-segmented__item', { on: modelValue === opt.value }]"
      :disabled="disabled || opt.disabled"
      :aria-checked="modelValue === opt.value"
      @click="select(opt)"
    >
      <span v-if="opt.icon" class="jc-segmented__icon">{{ opt.icon }}</span>
      <span>{{ opt.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.jc-segmented {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: var(--jc-radius, 6px);
}
.jc-segmented.is-block { display: flex; width: 100%; }
.jc-segmented.is-block .jc-segmented__item { flex: 1; }

.jc-segmented__item {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  border: none;
  background: transparent;
  color: var(--jc-text-secondary, #858585);
  font-family: inherit;
  border-radius: var(--jc-radius-sm, 4px);
  cursor: pointer;
  white-space: nowrap;
  transition: background 120ms ease, color 120ms ease, box-shadow 120ms ease;
}
.jc-segmented__item:hover:not(:disabled) { color: var(--jc-text-primary, #ccc); }
.jc-segmented__item.on {
  background: var(--jc-bg-elevated, #2d2d30);
  color: var(--jc-text-highlight, #e0e0e0);
  box-shadow: var(--jc-shadow-1, 0 1px 2px -2px rgba(0, 0, 0, 0.16));
}
.jc-segmented__item:disabled { opacity: 0.5; cursor: not-allowed; }

.jc-segmented--small .jc-segmented__item { height: 18px; padding: 0 8px; font-size: var(--jc-font-size-sm, 12px); }
.jc-segmented--middle .jc-segmented__item { height: 22px; padding: 0 12px; font-size: var(--jc-font-size-control, 12px); }
.jc-segmented--large .jc-segmented__item { height: 30px; padding: 0 16px; font-size: var(--jc-font-size-lg, 14px); }
</style>
