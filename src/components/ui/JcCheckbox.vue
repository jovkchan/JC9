<script setup lang="ts">
import { computed } from 'vue'

defineOptions({ name: 'JcCheckbox' })

// API 对齐 Ant Design Checkbox：checked / indeterminate / disabled / name / value
// 参考: https://ant.design/components/checkbox-cn
const props = withDefaults(
  defineProps<{
    checked?: boolean
    /** 半选状态（父节点部分选中） */
    indeterminate?: boolean
    disabled?: boolean
    name?: string
    value?: string | number
  }>(),
  {
    checked: false,
    indeterminate: false,
    disabled: false,
    name: '',
    value: undefined,
  },
)

const emit = defineEmits<{
  'update:checked': [value: boolean]
  change: [value: boolean, e: Event]
}>()

const classes = computed(() => [
  'jc-checkbox',
  {
    'is-checked': props.checked,
    'is-indeterminate': props.indeterminate && !props.checked,
    'is-disabled': props.disabled,
  },
])

function onChange(e: Event) {
  if (props.disabled) return
  const v = !props.checked
  emit('update:checked', v)
  emit('change', v, e)
}
</script>

<template>
  <label :class="classes" role="checkbox" :aria-checked="indeterminate ? 'mixed' : checked">
    <input
      type="checkbox"
      class="jc-checkbox__input"
      :name="name || undefined"
      :value="value"
      :checked="checked"
      :disabled="disabled"
      @change="onChange"
    />
    <span class="jc-checkbox__box" aria-hidden="true">
      <span v-if="indeterminate && !checked" class="jc-checkbox__indeterminate" />
      <svg v-else-if="checked" class="jc-checkbox__check" viewBox="0 0 12 10">
        <path d="M1 5.5 4.5 9 11 1" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </span>
    <span class="jc-checkbox__label"><slot /></span>
  </label>
</template>

<style scoped>
.jc-checkbox {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  user-select: none;
  font-size: var(--jc-font-size, 13px);
  color: var(--jc-text-primary, #ccc);
}
.jc-checkbox__input {
  position: absolute;
  width: 0;
  height: 0;
  opacity: 0;
}
.jc-checkbox__box {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: var(--jc-radius-sm, 4px);
  background: var(--jc-bg-input, #3c3c3c);
  color: var(--jc-color-white, #fff);
  flex-shrink: 0;
  transition: border-color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease;
}
.jc-checkbox:hover .jc-checkbox__box { border-color: var(--jc-color-accent, #8a58ff); }
.jc-checkbox.is-checked .jc-checkbox__box,
.jc-checkbox.is-indeterminate .jc-checkbox__box {
  background: var(--jc-color-accent, #8a58ff);
  border-color: var(--jc-color-accent, #8a58ff);
  box-shadow: 0 0 0 2px var(--jc-color-accent-light-9, rgba(138, 88, 255, 0.15));
}
.jc-checkbox__check { width: 10px; height: 10px; }
.jc-checkbox__indeterminate {
  width: 8px;
  height: 2px;
  border-radius: 1px;
  background: var(--jc-color-white, #fff);
}
.jc-checkbox.is-disabled { opacity: 0.5; cursor: not-allowed; }
</style>
