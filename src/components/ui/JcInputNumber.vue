<script setup lang="ts">
import { computed, ref } from 'vue'
import type { JcBorderBeamColor } from './JcBorderBeam.vue'
import JcBeam from './JcBeam.vue'
import { useBeam } from '../../composables/useBeam'

defineOptions({ name: 'JcInputNumber' })

export type JcInputNumberSize = 'large' | 'middle' | 'small' // 对齐 antd InputNumber.size

const props = withDefaults(
  defineProps<{
    modelValue?: number | null
    min?: number
    max?: number
    step?: number
    disabled?: boolean
    size?: JcInputNumberSize
    placeholder?: string
    /** 前置内容（文本或 #prefix 插槽），对齐 antd InputNumber.prefix */
    prefix?: string
    /** 后置内容（文本或 #suffix 插槽），对齐 antd InputNumber.suffix，如单位 'px'/'天' */
    suffix?: string
    /** 聚焦时显示流光边框（BorderBeam 效果） */
    beam?: boolean
    /** 流光颜色（beam 开启时生效），单色或渐变停靠点数组 */
    beamColor?: JcBorderBeamColor
    /** 内部光晕（与流光同步） */
    glow?: boolean
  }>(),
  {
    modelValue: null,
    min: undefined,
    max: undefined,
    step: 1,
    disabled: false,
    size: 'middle',
    placeholder: '',
    prefix: '',
    suffix: '',
    beam: false,
    beamColor: undefined,
    glow: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: number | null]
  change: [value: number | null]
  focus: [e: FocusEvent]
  blur: [e: FocusEvent]
  /** 在输入框内按下回车 */
  enter: []
}>()

const innerRef = ref<HTMLInputElement | null>(null)
const rootRef = ref<HTMLElement>()

const classes = computed(() => [
  'jc-input-number',
  `jc-input-number--${props.size}`,
  { 'has-beam': props.beam },
])

const { beamStyle } = useBeam({
  enabled: () => props.beam,
  color: () => props.beamColor,
  angle: () => 'to left',
  accelerate: () => false,
  duration: () => undefined,
  root: () => rootRef.value,
  sizeRatio: () => 0.4,
  glow: () => props.glow,
  glowBlur: () => undefined,
  glowOpacity: () => undefined,
})

/** 边界钳制 */
function clamp(n: number): number {
  let v = n
  if (props.min !== undefined && v < props.min) v = props.min
  if (props.max !== undefined && v > props.max) v = props.max
  return v
}

/** 提交一个合法数值 */
function commit(v: number | null) {
  emit('update:modelValue', v)
  emit('change', v)
}

function handleInput(e: Event) {
  const raw = (e.target as HTMLInputElement).value
  if (raw === '') {
    // 空输入不立刻提交（保留原值），失焦时恢复
    return
  }
  const n = Number(raw)
  if (Number.isNaN(n)) return
  commit(clamp(n))
}

function handleBlur(e: FocusEvent) {
  // 空输入恢复为最小值或上次值，避免 null 悬空
  const el = e.target as HTMLInputElement
  if (el.value === '' && props.modelValue !== null && props.modelValue !== undefined) {
    el.value = String(props.modelValue)
  }
  emit('blur', e)
}

function stepBy(delta: number) {
  if (props.disabled) return
  const base = props.modelValue ?? props.min ?? 0
  commit(clamp(base + delta))
}

const upDisabled = computed(
  () => props.disabled || (props.max !== undefined && (props.modelValue ?? 0) >= props.max),
)
const downDisabled = computed(
  () => props.disabled || (props.min !== undefined && (props.modelValue ?? 0) <= props.min),
)

defineExpose({
  focus: () => innerRef.value?.focus(),
  blur: () => innerRef.value?.blur(),
})
</script>

<template>
  <span ref="rootRef" :class="classes">
    <span v-if="prefix || $slots.prefix" class="jc-input-number__affix jc-input-number__affix--prefix">
      <slot name="prefix">{{ prefix }}</slot>
    </span>
    <input
      ref="innerRef"
      type="number"
      class="jc-input-number__inner"
      :value="modelValue ?? ''"
      :min="min"
      :max="max"
      :step="step"
      :disabled="disabled"
      :placeholder="placeholder"
      @input="handleInput"
      @focus="emit('focus', $event)"
      @blur="handleBlur"
      @keyup.enter="emit('enter')"
    />
    <span v-if="suffix || $slots.suffix" class="jc-input-number__affix jc-input-number__affix--suffix">
      <slot name="suffix">{{ suffix }}</slot>
    </span>
    <!-- 自绘增减按钮（替代原生 spinner，适配明暗） -->
    <span class="jc-input-number__handler" aria-hidden="true">
      <button
        type="button"
        class="jc-input-number__step"
        title="增加"
        tabindex="-1"
        :disabled="upDisabled"
        @click="stepBy(step)"
      >
        <svg viewBox="0 0 10 10" width="8" height="8" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M2 6.5 5 3.5 8 6.5" /></svg>
      </button>
      <button
        type="button"
        class="jc-input-number__step"
        title="减少"
        tabindex="-1"
        :disabled="downDisabled"
        @click="stepBy(-step)"
      >
        <svg viewBox="0 0 10 10" width="8" height="8" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3.5 5 6.5 8 3.5" /></svg>
      </button>
    </span>
    <!-- 聚焦流光边框 + 内部光晕 -->
    <JcBeam v-if="beam" :glow="glow" :style="beamStyle" />
  </span>
</template>

<style scoped>
.jc-input-number {
  position: relative;
  display: inline-flex;
  align-items: center;
  width: 100%;
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  overflow: hidden;
  transition: border-color 150ms var(--jc-motion-ease, ease), box-shadow 150ms var(--jc-motion-ease, ease);
}
.jc-input-number:not(:focus-within):hover {
  /* hover 轻微提亮，提供交互反馈 */
  border-color: var(--jc-color-accent-hover, #a070ff);
}
.jc-input-number:focus-within {
  /* 细边框：仅 1px accent 色，无外发光（与 JcInput 一致） */
  border-color: var(--jc-color-accent, #8a58ff);
}
.jc-input-number__inner {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: none;
  color: var(--jc-text-primary, #ccc);
  font-family: inherit;
  /* 隐藏原生增减按钮（白框来源） */
  -moz-appearance: textfield;
  appearance: textfield;
}
.jc-input-number__inner::-webkit-outer-spin-button,
.jc-input-number__inner::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.jc-input-number__inner::placeholder {
  color: var(--jc-text-secondary, #858585);
}
.jc-input-number__inner:focus {
  box-shadow: none;
}
.jc-input-number__inner:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 前后缀 */
.jc-input-number__affix {
  display: inline-flex;
  align-items: center;
  flex: none;
  color: var(--jc-text-secondary, #858585);
  white-space: nowrap;
  user-select: none;
  font-size: inherit;
}
.jc-input-number__affix--prefix {
  padding-left: 10px;
  padding-right: 2px;
}
.jc-input-number__affix--suffix {
  padding-left: 2px;
  padding-right: 2px;
}

/* 尺寸（对齐 JcInput：inner 定高 + 根边框，总高与 JcInput 完全一致） */
.jc-input-number--small .jc-input-number__inner { height: var(--jc-control-height-sm, 24px); padding: 0 6px; font-size: var(--jc-font-size-sm, 12px); }
.jc-input-number--middle .jc-input-number__inner { height: var(--jc-control-height, 28px); padding: 0 10px; font-size: var(--jc-font-size-control, 12px); }
.jc-input-number--large .jc-input-number__inner { height: var(--jc-control-height-lg, 36px); padding: 0 12px; font-size: var(--jc-font-size-lg, 14px); }

/* 右侧增减按钮（竖排两格，分隔线随明暗） */
.jc-input-number__handler {
  flex: 0 0 auto;
  display: flex;
  flex-direction: column;
  align-self: stretch;
  width: 18px;
  border-left: 1px solid var(--jc-border-default, #3e3e42);
}
.jc-input-number__step {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: none;
  padding: 0;
  color: var(--jc-text-secondary, #858585);
  cursor: pointer;
  transition: background-color 120ms var(--jc-motion-ease, ease), color 120ms var(--jc-motion-ease, ease);
}
.jc-input-number__step svg {
  transition: transform 140ms var(--jc-motion-ease, ease);
}
.jc-input-number__step + .jc-input-number__step {
  border-top: 1px solid var(--jc-border-default, #3e3e42);
}
.jc-input-number__step:hover:not(:disabled) {
  background: var(--jc-bg-btn-hover, #4c4c4c);
  color: var(--jc-text-primary, #ccc);
}
.jc-input-number__step:hover:not(:disabled) svg {
  transform: scale(1.18);
}
.jc-input-number__step:active:not(:disabled) {
  background: var(--jc-bg-selected, #37373d);
}
.jc-input-number__step:active:not(:disabled) svg {
  transform: scale(0.85);
}
.jc-input-number__step:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

/* 流光激活时原边框调浅为浅紫（避免深紫主色与流光重叠看不清） */
.jc-input-number.has-beam:focus-within {
  border-color: rgba(138, 88, 255, 0.45) !important;
}
</style>
