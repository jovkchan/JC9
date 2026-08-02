<script setup lang="ts">
import { computed } from 'vue'

defineOptions({ name: 'JcRadio' })

// API 对齐 Ant Design Radio：checked / value / disabled / name
// 参考: https://ant.design/components/radio-cn
const props = withDefaults(
  defineProps<{
    checked?: boolean
    value?: string | number
    disabled?: boolean
    name?: string
  }>(),
  {
    checked: false,
    value: undefined,
    disabled: false,
    name: '',
  },
)

const emit = defineEmits<{
  'update:checked': [value: boolean]
  change: [value: boolean, e: Event]
}>()

const classes = computed(() => [
  'jc-radio',
  {
    'is-checked': props.checked,
    'is-disabled': props.disabled,
  },
])

function onChange(e: Event) {
  if (props.disabled) return
  emit('update:checked', true)
  emit('change', true, e)
}
</script>

<template>
  <label :class="classes" role="radio" :aria-checked="checked">
    <input
      type="radio"
      class="jc-radio__input"
      :name="name || undefined"
      :value="value"
      :checked="checked"
      :disabled="disabled"
      @change="onChange"
    />
    <span class="jc-radio__dot" aria-hidden="true" />
    <span class="jc-radio__label"><slot /></span>
  </label>
</template>

<style scoped>
.jc-radio {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  user-select: none;
  font-size: var(--jc-font-size, 13px);
  color: var(--jc-text-primary, #ccc);
}
.jc-radio__input {
  position: absolute;
  width: 0;
  height: 0;
  opacity: 0;
}
.jc-radio__dot {
  position: relative;
  width: 16px;
  height: 16px;
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 50%;
  background: var(--jc-bg-input, #3c3c3c);
  flex-shrink: 0;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}
.jc-radio__dot::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--jc-color-accent, #8a58ff);
  transform: translate(-50%, -50%) scale(0);
  transition: transform 0.2s var(--jc-motion-ease, cubic-bezier(0.645, 0.045, 0.355, 1));
}
.jc-radio:hover .jc-radio__dot { border-color: var(--jc-color-accent, #8a58ff); }
.jc-radio.is-checked .jc-radio__dot {
  border-color: var(--jc-color-accent, #8a58ff);
  box-shadow: 0 0 0 2px var(--jc-color-accent-light-9, rgba(138, 88, 255, 0.15));
}
.jc-radio.is-checked .jc-radio__dot::after { transform: translate(-50%, -50%) scale(1); }
.jc-radio.is-disabled { opacity: 0.5; cursor: not-allowed; }
</style>
