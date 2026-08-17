<script setup lang="ts">
import { ref } from 'vue'
import JcInput from './JcInput.vue'
import type { JcBorderBeamColor } from './JcBorderBeam.vue'

defineOptions({ name: 'JcColorPicker' })

export type JcColorPickerSize = 'large' | 'middle' | 'small' // 对齐 antd Input.size

const props = withDefaults(
  defineProps<{
    modelValue?: string
    size?: JcColorPickerSize
    disabled?: boolean
    placeholder?: string
    /** 输入框聚焦时显示流光边框（透传给内部 JcInput） */
    beam?: boolean
    /** 流光颜色（beam 开启时生效），单色或渐变停靠点数组 */
    beamColor?: JcBorderBeamColor
    /** 内部光晕（透传给内部 JcInput） */
    glow?: boolean
  }>(),
  {
    modelValue: '',
    size: 'middle',
    disabled: false,
    placeholder: '留空',
    beam: false,
    beamColor: undefined,
    glow: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  change: [value: string, e: Event]
  focus: [e: FocusEvent]
  blur: [e: FocusEvent]
}>()

const nativeRef = ref<HTMLInputElement | null>(null)

function openNativePicker() {
  if (props.disabled) return
  nativeRef.value?.click()
}

/** 原生取色器选取颜色（注意：值恒为合法 hex，无需校验） */
function onNativeInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLInputElement).value)
}
</script>

<template>
  <div class="jc-color-picker">
    <!-- 取色块：点击触发原生取色器，棋盘格底表示透明 -->
    <span
      class="jc-color-picker__swatch"
      :class="`jc-color-picker__swatch--${size}`"
      :title="disabled ? '' : '点击打开取色器'"
      @click="openNativePicker"
    >
      <span
        class="jc-color-picker__color"
        :style="{ backgroundColor: modelValue || 'transparent' }"
      />
      <input
        ref="nativeRef"
        type="color"
        class="jc-color-picker__native"
        :value="modelValue && /^#[0-9a-fA-F]{6}$/.test(modelValue) ? modelValue : '#000000'"
        :disabled="disabled"
        @input="onNativeInput"
      />
    </span>
    <!-- 输入部分复用 JcInput（自带 beam 流光 / 明暗 token） -->
    <JcInput
      :model-value="modelValue"
      :size="size"
      :disabled="disabled"
      :placeholder="placeholder"
      :beam="beam"
      :beam-color="beamColor"
      :glow="glow"
      @update:model-value="emit('update:modelValue', $event)"
      @change="(v, e) => emit('change', v, e)"
      @focus="emit('focus', $event)"
      @blur="emit('blur', $event)"
    />
  </div>
</template>

<style scoped>
.jc-color-picker {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: 100%;
}

/* 取色块 */
.jc-color-picker__swatch {
  position: relative;
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  overflow: hidden;
  cursor: pointer;
  transition: border-color 120ms ease;
  /* 透明棋盘格底（亮/暗模式下随 --jc-* token 变化） */
  background-image:
    linear-gradient(45deg, var(--jc-color-checker, rgba(128,128,128,.35)) 25%, transparent 25%),
    linear-gradient(-45deg, var(--jc-color-checker, rgba(128,128,128,.35)) 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, var(--jc-color-checker, rgba(128,128,128,.35)) 75%),
    linear-gradient(-45deg, transparent 75%, var(--jc-color-checker, rgba(128,128,128,.35)) 75%);
  background-size: 8px 8px;
  background-position: 0 0, 0 4px, 4px -4px, -4px 0;
}
.jc-color-picker__swatch--small { width: 24px; height: 24px; }
.jc-color-picker__swatch--middle { width: 28px; height: 28px; }
.jc-color-picker__swatch--large { width: 36px; height: 36px; }
.jc-color-picker__swatch:hover { border-color: var(--jc-color-accent-hover, #a070ff); }
.jc-color-picker__swatch:active { border-color: var(--jc-color-accent-active, #6f35e8); }
.jc-color-picker__swatch:has(.jc-color-picker__native:focus-visible) {
  border-color: var(--jc-color-accent, #8a58ff);
}
.jc-color-picker__swatch:has(.jc-color-picker__native:disabled) {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 当前颜色覆盖层（铺在棋盘格之上） */
.jc-color-picker__color {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

/* 原生取色器：透明铺满色块，点击即打开系统取色弹窗 */
.jc-color-picker__native {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  padding: 0;
  border: none;
  opacity: 0;
  cursor: pointer;
}
</style>
