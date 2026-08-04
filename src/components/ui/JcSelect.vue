<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from 'vue'
import type { JcBorderBeamColor } from './JcBorderBeam.vue'
import JcBeam from './JcBeam.vue'
import { useBeam } from '../../composables/useBeam'

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
    modelValue: undefined,
    options: () => [],
    placeholder: '',
    disabled: false,
    size: 'middle',
    title: '',
    beam: false,
    beamColor: undefined,
    beamAngle: 'to left',
    beamAccelerate: false,
    beamDuration: undefined,
    glow: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  change: [value: string | number, e: Event]
}>()

const classes = computed(() => ['jc-select', `jc-select--${props.size}`, { 'has-beam': props.beam }])

const rootRef = ref<HTMLElement>()
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

// ── 自定义下拉（div 模拟 select，流光可包裹选框 + 弹出列表整体） ──
const triggerRef = ref<HTMLElement>()
const dropdownRef = ref<HTMLElement>()
const open = ref(false)
const expandDir = ref<'up' | 'down'>('down')
const dropdownStyle = ref<Record<string, string>>({})
const overlayStyle = ref<Record<string, string>>({})
let docHandler: ((e: MouseEvent) => void) | undefined

const selected = computed(() => props.options.find((o) => o.value === props.modelValue))

function isSelected(opt: JcSelectOption) {
  return opt.value === props.modelValue
}

function updatePos() {
  const tr = triggerRef.value
  const dd = dropdownRef.value
  if (!tr || !open.value) return
  const r = tr.getBoundingClientRect()
  const dropH = Math.min(240, props.options.length * 28 + 10)
  const spaceBelow = window.innerHeight - r.bottom
  const up = spaceBelow < dropH && r.top > spaceBelow
  expandDir.value = up ? 'up' : 'down'

  const ddPosTop = up ? window.innerHeight - r.top : r.bottom
  const ddH = dd && dd.offsetHeight > 0 ? dd.offsetHeight : dropH

  const pos: Record<string, string> = {
    position: 'fixed',
    left: `${Math.round(r.left)}px`,
    width: `${Math.round(r.width)}px`,
    zIndex: '9999',
  }
  if (up) {
    pos.bottom = `${Math.round(window.innerHeight - r.top)}px`
  } else {
    pos.top = `${Math.round(r.bottom)}px`
  }
  dropdownStyle.value = pos

  // 整体大流光：一个环合并「选框 + 列表」外框（下拉打开时）
  const top = up ? window.innerHeight - ddPosTop - ddH : r.top
  const bottom = up ? r.bottom : ddPosTop + ddH
  overlayStyle.value = {
    position: 'fixed',
    left: `${Math.round(r.left)}px`,
    top: `${Math.round(top)}px`,
    width: `${Math.round(r.width)}px`,
    height: `${Math.round(bottom - top)}px`,
    zIndex: '10000',
  }
}

function openDrop() {
  if (props.disabled) return
  open.value = true
  nextTick(updatePos)
  docHandler = (e: MouseEvent) => {
    const root = rootRef.value
    const dd = dropdownRef.value
    if (root && dd && !root.contains(e.target as Node) && !dd.contains(e.target as Node)) {
      closeDrop()
    }
  }
  document.addEventListener('mousedown', docHandler)
}

function closeDrop() {
  open.value = false
  if (docHandler) {
    document.removeEventListener('mousedown', docHandler)
    docHandler = undefined
  }
}

function toggle() {
  open.value ? closeDrop() : openDrop()
}

function select(opt: JcSelectOption) {
  if (opt.disabled) return
  emit('update:modelValue', opt.value)
  emit('change', opt.value, new MouseEvent('change'))
  closeDrop()
}

function onKeydown(e: KeyboardEvent) {
  if (props.disabled) return
  if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault()
    open.value ? closeDrop() : openDrop()
  } else if (e.key === 'Escape') {
    closeDrop()
  }
}

onBeforeUnmount(() => {
  if (docHandler) document.removeEventListener('mousedown', docHandler)
})
</script>

<template>
  <span ref="rootRef" :class="classes" :title="title">
    <div
      ref="triggerRef"
      class="jc-select__trigger"
      :class="{ 'is-open': open, 'is-disabled': disabled, 'is-placeholder': !selected, 'is-expand-up': expandDir === 'up', 'is-expand-down': expandDir === 'down' }"
      role="combobox"
      :aria-expanded="open"
      :tabindex="disabled ? -1 : 0"
      @click="toggle"
      @keydown="onKeydown"
      @blur="closeDrop"
    >
      <span class="jc-select__value">{{ selected ? selected.label : placeholder }}</span>
      <span class="jc-select__arrow" aria-hidden="true">▾</span>
      <!-- 选框流光 + 内部光晕（下拉关闭时；打开时改用整体大流光包裹选框+列表） -->
      <JcBeam v-if="beam && !open" :glow="glow" :style="beamStyle" />
    </div>

    <!-- 下拉列表（Teleport 到 body，流光可包裹弹出整体外框） -->
    <Teleport to="body">
      <!-- 整体大流光：一个环包裹选框 + 下拉列表整体外框（JcBeam 封装流光+光晕） -->
      <span
        v-if="beam && open"
        class="jc-select__beam-overlay"
        :style="overlayStyle"
        aria-hidden="true"
      >
        <JcBeam :glow="glow" :style="beamStyle" />
      </span>
      <div
        v-show="open"
        ref="dropdownRef"
        class="jc-select__dropdown"
        :class="[`jc-select__dropdown--${props.size}`, { 'has-beam': props.beam, 'is-expand-up': expandDir === 'up', 'is-expand-down': expandDir === 'down' }]"
        :style="dropdownStyle"
        @mousedown.prevent
      >
        <div class="jc-select__list">
          <div
            v-for="opt in options"
            :key="String(opt.value)"
            class="jc-select__option"
            :class="{ 'is-selected': isSelected(opt), 'is-disabled': opt.disabled }"
            @mousedown.prevent
            @click="select(opt)"
          >
            {{ opt.label }}
          </div>
          <div v-if="options.length === 0" class="jc-select__empty">暂无选项</div>
        </div>
      </div>
    </Teleport>
  </span>
</template>

<style scoped>
.jc-select {
  position: relative;
  display: inline-flex;
  flex-shrink: 0;
}
.jc-select__trigger {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  font-family: inherit;
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  color: var(--jc-text-primary, #ccc);
  outline: none;
  cursor: pointer;
  user-select: none;
  transition: border-color 120ms ease, box-shadow 120ms ease;
}
.jc-select__trigger:focus,
.jc-select__trigger.is-open {
  /* 细边框：仅 1px accent 色，无外发光（亮/暗一致，暗色下不再发亮） */
  border-color: var(--jc-color-accent, #8a58ff);
  box-shadow: none;
}
.jc-select__trigger.is-disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.jc-select__value {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.jc-select__value.is-placeholder {
  color: var(--jc-text-secondary, #858585);
}

.jc-select--small .jc-select__trigger { height: var(--jc-control-height-sm, 24px); padding: 0 24px 0 8px; font-size: var(--jc-font-size-sm, 12px); }
.jc-select--middle .jc-select__trigger { height: var(--jc-control-height, 28px); padding: 0 26px 0 10px; font-size: var(--jc-font-size-control, 12px); }
.jc-select--large .jc-select__trigger { height: var(--jc-control-height-lg, 36px); padding: 0 28px 0 12px; font-size: var(--jc-font-size-lg, 14px); }

.jc-select__arrow {
  position: absolute;
  right: 8px;
  pointer-events: none;
  font-size: 10px;
  color: var(--jc-text-secondary, #858585);
  transition: transform 120ms ease;
}
.jc-select__trigger.is-open .jc-select__arrow {
  transform: rotate(180deg);
}

/* ── 下拉列表 ── */
.jc-select__dropdown {
  position: relative;
  /* 背景与选框同色，融合时连接处无色差分割线；融合一体后无阴影（与选框一致） */
  background: var(--jc-bg-input, #3c3c3c);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  overflow: hidden;
}
.jc-select__list {
  max-height: 240px;
  overflow-y: auto;
  padding: 4px;
}
.jc-select__option {
  padding: 4px 8px;
  border-radius: 2px;
  font-size: inherit;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--jc-text-primary, #ccc);
}
.jc-select__option:hover {
  background: var(--jc-bg-btn-hover, #4c4c4c);
}
.jc-select__option.is-selected {
  color: var(--jc-color-accent, #8a58ff);
  font-weight: 500;
}
.jc-select__option.is-disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.jc-select__empty {
  padding: 8px;
  text-align: center;
  color: var(--jc-text-secondary, #858585);
  font-size: 12px;
}

/* ── 融合：下拉与选框无缝隙连成一体（点击后像一个整体，连接处无分割线） ── */
.jc-select__trigger.is-open.is-expand-down {
  border-bottom-left-radius: 0;
  border-bottom-right-radius: 0;
  border-bottom: none;
}
.jc-select__dropdown.is-expand-down {
  border-top-left-radius: 0;
  border-top-right-radius: 0;
  border-top: none;
}
.jc-select__trigger.is-open.is-expand-up {
  border-top-left-radius: 0;
  border-top-right-radius: 0;
  border-top: none;
}
.jc-select__dropdown.is-expand-up {
  border-bottom-left-radius: 0;
  border-bottom-right-radius: 0;
  border-bottom: none;
}

/* 流光：整体大流光包裹「选框 + 列表」（Teleport 到 body，组件内触发显示） */
.jc-select__beam-overlay {
  pointer-events: none;
  /* 整体浮动阴影：选框 + 下拉一起浮起（融合一体的投影） */
  box-shadow: var(--jc-shadow-menu, 0 4px 14px rgba(0, 0, 0, 0.4));
  /* 流光/光晕常显规则已移至 global.scss：Teleport 到 body 不在 :focus-within 作用域，
     且 .jc-beam/.jc-beam-glow 由 JcBeam 组件渲染（无本组件 data-v），scoped 匹配不到 */
}
/* 流光激活时原边框调浅为浅紫（避免与流光重叠看不清） */
.jc-select.has-beam:focus-within .jc-select__trigger {
  border-color: rgba(138, 88, 255, 0.45) !important;
}
.jc-select__dropdown.has-beam {
  border-color: rgba(138, 88, 255, 0.45) !important;
}

</style>
