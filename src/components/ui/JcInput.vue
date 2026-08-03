<script setup lang="ts">
import { computed, ref } from 'vue'
import type { JcBorderBeamColor } from './JcBorderBeam.vue'
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
  }>(),
  {
    modelValue: '',
    type: 'text',
    placeholder: '',
    disabled: false,
    readonly: false,
    size: 'middle',
    clearable: false,
    maxlength: undefined,
    beam: false,
    beamColor: undefined,
    beamAngle: 'to left',
    beamAccelerate: false,
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
const { beamStyle } = useBeam({
  enabled: () => props.beam,
  color: () => props.beamColor,
  angle: () => props.beamAngle,
  accelerate: () => props.beamAccelerate,
  root: () => rootRef.value,
  sizeRatio: () => 0.4,
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
    <input
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
    <button
      v-if="clearable && !!modelValue"
      type="button"
      class="jc-input__clear"
      title="清空"
      @click="clear"
    >
      ✕
    </button>
    <!-- 聚焦流光边框（beam 开启时，共享 .jc-beam 样式） -->
    <span v-if="beam" class="jc-beam" :style="beamStyle" aria-hidden="true">
      <span class="jc-beam__effect" />
    </span>
  </span>
</template>

<style scoped>
.jc-input {
  position: relative;
  display: inline-flex;
  align-items: center;
  width: 100%;
}
.jc-input__inner {
  width: 100%;
  font-family: inherit;
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  color: var(--jc-text-primary, #ccc);
  outline: none;
  transition: border-color 120ms ease, box-shadow 120ms ease;
}
.jc-input__inner::placeholder {
  color: var(--jc-text-secondary, #858585);
}
.jc-input__inner:focus {
  /* 细边框：仅 1px accent 色，无外发光（亮/暗一致，暗色下不再发亮） */
  border-color: var(--jc-color-accent, #8a58ff);
  box-shadow: none;
}
.jc-input__inner:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.jc-input--small .jc-input__inner { height: var(--jc-control-height-sm, 24px); padding: 0 8px; font-size: var(--jc-font-size-sm, 12px); }
.jc-input--middle .jc-input__inner { height: var(--jc-control-height, 28px); padding: 0 10px; font-size: var(--jc-font-size-control, 12px); }
.jc-input--large .jc-input__inner { height: var(--jc-control-height-lg, 36px); padding: 0 12px; font-size: var(--jc-font-size-lg, 14px); }

.jc-input.is-clearable .jc-input__inner { padding-right: 26px; }

.jc-input__clear {
  position: absolute;
  right: 6px;
  width: 16px;
  height: 16px;
  display: flex;
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
}
.jc-input__clear:hover {
  color: var(--jc-text-primary, #ccc);
}

/* 流光激活时原边框调浅为浅紫（避免深紫主色与流光重叠看不清） */
.jc-input.has-beam:focus-within .jc-input__inner {
  border-color: rgba(138, 88, 255, 0.45) !important;
}

</style>
