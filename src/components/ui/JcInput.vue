<script setup lang="ts">
import { computed, ref } from 'vue'
import type { JcBorderBeamColor } from './JcBorderBeam.vue'
import JcBeam from './JcBeam.vue'
import { useBeam } from '../../composables/useBeam'

defineOptions({ name: 'JcInput' })

export type JcInputSize = 'large' | 'middle' | 'small'  // 对齐 antd Input.size

const props = withDefaults(
  defineProps<{
    modelValue?: string
    type?: string
    placeholder?: string
    disabled?: boolean
    readonly?: boolean
    size?: JcInputSize
    /** 前置内容（文本或 #prefix 插槽），对齐 antd Input.prefix */
    prefix?: string
    /** 后置内容（文本或 #suffix 插槽），对齐 antd Input.suffix */
    suffix?: string
    /** 可一键清空 */
    clearable?: boolean
    maxlength?: number
    /** 聚焦时显示流光边框（BorderBeam 效果） */
    beam?: boolean
    /** 流光颜色（beam 开启时生效），单色或渐变停靠点数组 */
    beamColor?: JcBorderBeamColor
    /** 流光渐变方向（beam 开启时生效），如 'to left' 或 '-225deg' */
    beamAngle?: string
    /** 拐角变速：true=拐角轻微加速 / false=匀速 */
    beamAccelerate?: boolean
    /** 流光完成一圈的时长（秒），缺省 6s */
    beamDuration?: number
    /** 内部光晕：内环光束与流光同步（同速/同位/同色），模糊柔化为内部发光 */
    glow?: boolean
  }>(),
  {
    modelValue: '',
    type: 'text',
    placeholder: '',
    disabled: false,
    readonly: false,
    size: 'middle',
    prefix: '',
    suffix: '',
    clearable: false,
    maxlength: undefined,
    beam: false,
    beamColor: undefined,
    beamAngle: 'to left',
    beamAccelerate: false,
    beamDuration: undefined,
    glow: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  change: [value: string, e: Event]
  focus: [e: FocusEvent]
  blur: [e: FocusEvent]
  clear: []
}>()

const classes = computed(() => [
  'jc-input',
  `jc-input--${props.size}`,
  { 'is-clearable': props.clearable && !!props.modelValue },
  { 'has-beam': props.beam },
])

const rootRef = ref<HTMLElement>()
const innerRef = ref<HTMLInputElement>()
defineExpose({
  focus: () => innerRef.value?.focus(),
  blur: () => innerRef.value?.blur(),
})
const { beamStyle } = useBeam({
  enabled: () => props.beam,
  color: () => props.beamColor,
  angle: () => props.beamAngle,
  accelerate: () => props.beamAccelerate,
  duration: () => props.beamDuration,
  root: () => rootRef.value,
  sizeRatio: () => 0.4,
  glow: () => props.glow,
  glowBlur: () => undefined,
  glowOpacity: () => undefined,
})

function onInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLInputElement).value)
}
function onChange(e: Event) {
  emit('change', (e.target as HTMLInputElement).value, e)
}
function clear() {
  emit('update:modelValue', '')
  emit('clear')
}
</script>

<template>
  <span ref="rootRef" :class="classes">
    <span v-if="prefix || $slots.prefix" class="jc-input__affix jc-input__affix--prefix">
      <slot name="prefix">{{ prefix }}</slot>
    </span>
    <input
      ref="innerRef"
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
      :maxlength="maxlength"
      class="jc-input__inner"
      @input="onInput"
      @change="onChange"
      @focus="emit('focus', $event)"
      @blur="emit('blur', $event)"
    />
    <span class="jc-input__after">
      <button
        v-if="clearable && !!modelValue"
        type="button"
        class="jc-input__clear"
        title="清空"
        @click="clear"
      >
        ✕
      </button>
      <span v-if="suffix || $slots.suffix" class="jc-input__affix jc-input__affix--suffix">
        <slot name="suffix">{{ suffix }}</slot>
      </span>
    </span>
    <!-- 聚焦流光边框 + 内部光晕（JcBeam 封装：流光环与光晕共用同一套 beamStyle 变量） -->
    <JcBeam v-if="beam" :glow="glow" :style="beamStyle" />
  </span>
</template>

<style scoped>
.jc-input {
  position: relative;
  display: inline-flex;
  align-items: center;
  width: 100%;
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  transition: border-color 120ms ease, box-shadow 120ms ease;
}
.jc-input:focus-within {
  /* 细边框：仅 1px accent 色，无外发光（亮/暗一致，暗色下不再发亮） */
  border-color: var(--jc-color-accent, #8a58ff);
}
.jc-input__inner {
  flex: 1;
  min-width: 0;
  font-family: inherit;
  background: transparent;
  border: none;
  outline: none;
  color: var(--jc-text-primary, #ccc);
  transition: border-color 120ms ease, box-shadow 120ms ease;
}
.jc-input__inner::placeholder {
  color: var(--jc-text-secondary, #858585);
}
.jc-input__inner:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 前后缀 */
.jc-input__affix {
  display: inline-flex;
  align-items: center;
  flex: none;
  color: var(--jc-text-secondary, #858585);
  white-space: nowrap;
  user-select: none;
  font-size: inherit;
}
.jc-input__affix--prefix {
  padding-left: 10px;
  padding-right: 2px;
}
.jc-input__affix--suffix {
  padding-left: 2px;
  padding-right: 4px;
}
.jc-input__after {
  display: inline-flex;
  align-items: center;
  flex: none;
  padding-right: 6px;
}

.jc-input--small .jc-input__inner { height: var(--jc-control-height-sm, 24px); padding: 0 8px; font-size: var(--jc-font-size-sm, 12px); }
.jc-input--middle .jc-input__inner { height: var(--jc-control-height, 28px); padding: 0 10px; font-size: var(--jc-font-size-control, 12px); }
.jc-input--large .jc-input__inner { height: var(--jc-control-height-lg, 36px); padding: 0 12px; font-size: var(--jc-font-size-lg, 14px); }

.jc-input__clear {
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: var(--jc-bg-btn-hover, #4c4c4c);
  color: var(--jc-text-secondary, #858585);
  font-size: 10px;
  line-height: 1;
  cursor: pointer;
  padding: 0;
  margin-right: 4px;
}
.jc-input__clear:hover {
  color: var(--jc-text-primary, #ccc);
}

/* 流光激活时原边框调浅为浅紫（避免深紫主色与流光重叠看不清） */
.jc-input.has-beam:focus-within {
  border-color: rgba(138, 88, 255, 0.45) !important;
}

</style>
