<script setup lang="ts">
import { computed } from 'vue'

defineOptions({ name: 'JcTextarea' })

const props = withDefaults(
  defineProps<{
    modelValue?: string
    placeholder?: string
    rows?: number
    disabled?: boolean
    readonly?: boolean
    /** 等宽字体（代码/JSON 场景） */
    mono?: boolean
    /** 是否允许手动拖拽缩放 */
    resize?: boolean
    spellcheck?: boolean
  }>(),
  {
    modelValue: '',
    placeholder: '',
    rows: 6,
    disabled: false,
    readonly: false,
    mono: false,
    resize: true,
    spellcheck: true,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  change: [value: string]
  focus: [e: FocusEvent]
  blur: [e: FocusEvent]
}>()

const classes = computed(() => [
  'jc-textarea',
  {
    'is-mono': props.mono,
    'is-fixed': !props.resize,
  },
])

function onInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLTextAreaElement).value)
}
function onChange(e: Event) {
  emit('change', (e.target as HTMLTextAreaElement).value)
}
</script>

<template>
  <textarea
    :value="modelValue"
    :placeholder="placeholder"
    :rows="rows"
    :disabled="disabled"
    :readonly="readonly"
    :spellcheck="spellcheck"
    :class="classes"
    @input="onInput"
    @change="onChange"
    @focus="emit('focus', $event)"
    @blur="emit('blur', $event)"
  />
</template>

<style scoped>
.jc-textarea {
  display: block;
  width: 100%;
  font-family: inherit;
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  color: var(--jc-text-primary, #ccc);
  padding: 6px 8px;
  outline: none;
  line-height: 1.6;
  transition: border-color 120ms ease, box-shadow 120ms ease;
}
.jc-textarea::placeholder {
  color: var(--jc-text-secondary, #858585);
}
.jc-textarea:focus {
  border-color: var(--jc-color-accent, #8a58ff);
  box-shadow: 0 0 0 2px var(--jc-color-accent-light-9, rgba(138, 88, 255, 0.15));
}
.jc-textarea:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.jc-textarea.is-mono {
  font-family: 'Cascadia Code', 'Consolas', 'SF Mono', Menlo, monospace;
  font-size: 12px;
}
.jc-textarea.is-fixed {
  resize: none;
}
</style>
