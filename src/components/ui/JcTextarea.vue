<script setup lang="ts">
import { computed, ref } from 'vue'
import type { JcBorderBeamColor } from './JcBorderBeam.vue'
import JcBeam from './JcBeam.vue'
import { useBeam } from '../../composables/useBeam'

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
    /** 聚焦时显示流光边框（BorderBeam 效果）。开启后内部用 wrapper 承载流光层，根元素变为 div.jc-textarea-host */
    beam?: boolean
    /** 流光颜色（beam 开启时生效），单色或渐变停靠点数组 */
    beamColor?: JcBorderBeamColor
    /** 流光长度 = 宿主宽度 × 比例（beam 开启时生效，如 0.4） */
    beamSizeRatio?: number
    /** 流光渐变方向（beam 开启时生效），如 'to left' 或 '-225deg' */
    beamAngle?: string
    /** 拐角变速：true=拐角轻微加速 / false=匀速 */
    beamAccelerate?: boolean
    /** 内部光晕：内环光束与流光同步（同速/同位/同色），模糊柔化为内部发光 */
    glow?: boolean
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
    beam: false,
    beamColor: undefined,
    beamSizeRatio: undefined,
    beamAngle: 'to left',
    beamAccelerate: false,
    glow: false,
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

const hostRef = ref<HTMLElement>()
const { beamStyle } = useBeam({
  enabled: () => props.beam,
  color: () => props.beamColor,
  angle: () => props.beamAngle,
  accelerate: () => props.beamAccelerate,
  root: () => hostRef.value,
  sizeRatio: () => props.beamSizeRatio ?? 0.4,
  glow: () => props.glow,
  glowBlur: () => undefined,
  glowOpacity: () => undefined,
})

function onInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLTextAreaElement).value)
}
function onChange(e: Event) {
  emit('change', (e.target as HTMLTextAreaElement).value)
}
</script>

<template>
  <!-- 无 beam：保持原生 textarea 根（零影响，兼容 ref / class / scoped 穿透 / 事件） -->
  <textarea
    v-if="!beam"
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
  <!-- 有 beam：wrapper 承载流光层（:focus-within 触发） -->
  <div v-else ref="hostRef" class="jc-textarea-host">
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
    <!-- 聚焦流光边框 + 内部光晕（JcBeam 封装） -->
    <JcBeam :glow="glow" :style="beamStyle" />
  </div>
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
  /* 细边框：仅 1px accent 色，无外发光（亮/暗一致，暗色下不再发亮） */
  border-color: var(--jc-color-accent, #8a58ff);
  box-shadow: none;
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

/* ── 聚焦流光边框（beam 开启，对齐 JcBorderBeam / JcInput：mask 挖环 + CSS Motion Path） ── */
.jc-textarea-host {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  width: 100%;
}
.jc-textarea-host > .jc-textarea {
  flex: 1;
  min-height: 0;
  height: 100%;
  resize: none;
}

/* 流光激活时原边框调浅为浅紫（避免深紫主色与流光重叠看不清） */
.jc-textarea-host:focus-within > .jc-textarea {
  border-color: rgba(138, 88, 255, 0.45) !important;
}
</style>
