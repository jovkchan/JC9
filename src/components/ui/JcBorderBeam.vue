<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { buildBeamGradient } from '../../composables/useBeam'

defineOptions({ name: 'JcBorderBeam' })

// API 对齐 Ant Design BorderBeam（6.4.0+），并扩展 wrap/trigger 以兼容 input/select 等替换元素
// - 替换元素（input/select/textarea 等）不能容纳注入的流光层，需用 wrap 模式（组件包一层相对定位包裹层）
// - trigger 控制流光何时显示：always 常显 / hover 悬浮 / focus 聚焦（:focus-within 语义，focusin/focusout）
export type JcBorderBeamColor = string | { color: string; percent: number }[]
export type JcBorderBeamTrigger = 'always' | 'hover' | 'focus'

const props = withDefaults(
  defineProps<{
    /** 流光颜色：单色字符串 或 渐变停靠点数组（percent 使用 0~100 输入区间） */
    color?: JcBorderBeamColor
    /** 流光完成一圈动画的时间（秒） */
    duration?: number
    /** 流光线宽，数字按 px 处理 */
    lineWidth?: number | string
    /** 流光层相对容器边缘的外扩距离，遇到裁剪容器可设为 0 */
    outset?: number | string
    /** 流光可见段尺寸，数字按 px 处理 */
    size?: number | string
    /** 流光长度按宿主宽度比例（0~1，如 0.4 = 宽度的 40%），提供时覆盖 size 并随宿主宽度实时变化 */
    sizeRatio?: number
    /** 流光渐变方向，如 'to left' 或 '-225deg'（与 color 配合） */
    beamAngle?: string
    /** 拐角变速：true=拐角轻微加速 / false=匀速 */
    beamAccelerate?: boolean
    /** 流光何时显示：always 常显 / hover 悬浮 / focus 聚焦（input/select 场景用 focus） */
    trigger?: JcBorderBeamTrigger
    /** 用包裹层承载流光（兼容 input/select 等替换元素；包裹层为 inline-block 贴合内容） */
    wrap?: boolean
  }>(),
  {
    color: undefined,
    duration: 6,
    lineWidth: 1,
    outset: undefined,
    size: 100,
    sizeRatio: undefined,
    beamAngle: 'to left',
    beamAccelerate: false,
    trigger: 'always',
    wrap: false,
  },
)

const anchorRef = ref<HTMLSpanElement | null>(null)
const wrapRef = ref<HTMLDivElement | null>(null)
const hostEl = ref<HTMLElement | null>(null)
const borderState = ref({ width: 0, radius: '0px', borderWidth: [0, 0, 0, 0] as number[] })
let ro: ResizeObserver | null = null

// trigger 触发状态
const active = ref(props.trigger === 'always')
const clearTrigger = ref<(() => void) | null>(null)

function bindTrigger(el: HTMLElement) {
  if (props.trigger === 'always') {
    active.value = true
    return
  }
  active.value = false
  const on = () => { active.value = true }
  const off = () => { active.value = false }
  if (props.trigger === 'hover') {
    el.addEventListener('mouseenter', on)
    el.addEventListener('mouseleave', off)
    clearTrigger.value = () => {
      el.removeEventListener('mouseenter', on)
      el.removeEventListener('mouseleave', off)
    }
  } else {
    // focus：focusin/focusout 冒泡，包裹层或宿主获得焦点即显示
    el.addEventListener('focusin', on)
    el.addEventListener('focusout', off)
    clearTrigger.value = () => {
      el.removeEventListener('focusin', on)
      el.removeEventListener('focusout', off)
    }
  }
}

const unit = (v: number | string) => {
  // 数字补 px；纯数字字符串（如 "80"）也补 px，否则 CSS 变量无单位会失效
  if (typeof v === 'number') return `${v}px`
  const s = String(v).trim()
  return /^\d+(\.\d+)?$/.test(s) ? `${s}px` : v
}

const gradient = computed(() => buildBeamGradient(props.color, props.beamAngle))

const beamStyle = computed(() => {
  const { outset, duration, lineWidth, size, sizeRatio } = props
  // 默认贴合宿主边框外沿（inset = -borderWidth）；outset 显式指定时覆盖
  const insets = borderState.value.borderWidth.map((w) => `-${w}px`).join(' ')
  // 流光长度：提供 sizeRatio 时按宿主宽度比例计算（随宽度实时变化），否则用固定 size
  const beamSize = sizeRatio != null && borderState.value.width > 0
    ? `${Math.round(borderState.value.width * sizeRatio)}px`
    : unit(size)
  return {
    '--jc-bb-inset': outset !== undefined
      ? (typeof outset === 'number' ? `-${outset}px` : `calc(-1 * ${outset})`)
      : insets,
    '--jc-bb-radius': borderState.value.radius,
    '--jc-bb-line-width': unit(lineWidth),
    '--jc-bb-size': beamSize,
    '--jc-bb-duration': `${duration}s`,
    '--jc-bb-gradient': gradient.value,
    '--jc-bb-anim': props.beamAccelerate ? 'jc-beam-move-acc' : 'jc-beam-move',
  } as Record<string, string>
})

function measure() {
  const host = hostEl.value
  if (!host) return
  const cs = getComputedStyle(host)
  const num = (v: string) => { const n = parseFloat(v); return Number.isFinite(n) ? n : 0 }
  borderState.value = {
    width: host.offsetWidth,
    radius: cs.borderRadius,
    borderWidth: [
      num(cs.borderTopWidth),
      num(cs.borderRightWidth),
      num(cs.borderBottomWidth),
      num(cs.borderLeftWidth),
    ],
  }
}

onMounted(() => {
  // wrap 模式：包裹层自身是宿主（能容纳流光层）；注入模式：slot 根元素是宿主
  const host = props.wrap ? wrapRef.value : (anchorRef.value?.previousElementSibling as HTMLElement | null)
  if (!host) return
  hostEl.value = host
  bindTrigger(host)
  measure()
  ro = new ResizeObserver(measure)
  ro.observe(host)
})

onBeforeUnmount(() => {
  clearTrigger.value?.()
  clearTrigger.value = null
  ro?.disconnect()
  ro = null
})
</script>

<template>
  <!-- wrap 模式：相对定位包裹层承载流光（兼容 input/select 等替换元素） -->
  <div v-if="wrap" ref="wrapRef" class="jc-border-beam__wrap" :class="{ 'is-active': active }">
    <slot />
    <div class="jc-border-beam" :class="{ 'is-active': active }" :style="beamStyle" aria-hidden="true">
      <span class="jc-border-beam__effect" />
    </div>
  </div>
  <!-- 注入模式：原样渲染子内容，流光层注入宿主内部（不改变布局） -->
  <template v-else>
    <slot />
    <span ref="anchorRef" aria-hidden="true" class="jc-border-beam__anchor" />
    <Teleport v-if="hostEl" :to="hostEl">
      <div class="jc-border-beam" :class="{ 'is-active': active }" :style="beamStyle" aria-hidden="true">
        <span class="jc-border-beam__effect" />
      </div>
    </Teleport>
  </template>
</template>

<style scoped>
.jc-border-beam__anchor {
  display: none;
}

/* wrap 模式包裹层：inline-block 贴合内容尺寸，作为流光定位上下文 */
.jc-border-beam__wrap {
  position: relative;
  display: inline-block;
}

/* 流光层：mask 挖环 + CSS Motion Path 光束（对齐 antd style）
   trigger 控制：默认隐藏（opacity 0 + 动画暂停），is-active 时显示
   注意：scoped + @supports 下必须用平铺选择器，不能 SCSS 嵌套（否则编译成无效选择器） */
.jc-border-beam {
  display: none;
  position: absolute;
  inset: var(--jc-bb-inset, -1px);
  border-radius: var(--jc-bb-radius, 0px);
  z-index: 1;
  overflow: hidden;
  pointer-events: none;
  padding: var(--jc-bb-line-width, 1px);
  opacity: 0;
  transition: opacity 0.3s ease;
}
.jc-border-beam__effect {
  animation-play-state: paused;
}
.jc-border-beam.is-active {
  opacity: 1;
}
.jc-border-beam.is-active .jc-border-beam__effect {
  animation-play-state: running;
}

@supports ((mask-composite: exclude) or (-webkit-mask-composite: xor)) {
  .jc-border-beam {
    -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    mask-composite: exclude;
  }
}

@supports (offset-path: rect(0 auto auto 0 round 1px)) {
  .jc-border-beam {
    display: block;
  }
  .jc-border-beam__effect {
    position: absolute;
    top: 0;
    left: 0;
    width: var(--jc-bb-size, 100px);
    height: var(--jc-beam-height, 12px);
    opacity: 1;
    background-image: var(--jc-bb-gradient, var(--jc-beam-gradient, linear-gradient(to left, #a878ff 0%, #c99fff 70%, transparent)));
    offset-anchor: 90% 50%;
    offset-distance: 0%;
    offset-path: rect(0 auto auto 0 round var(--jc-beam-height, 12px));
    offset-rotate: auto;
    animation-name: var(--jc-bb-anim, jc-beam-move-acc);
    animation-duration: var(--jc-bb-duration, 6s);
    animation-timing-function: linear;
    animation-iteration-count: infinite;
    will-change: offset-distance;
  }
}

/* 减少动态效果时隐藏流光 */
@media (prefers-reduced-motion: reduce) {
  .jc-border-beam__effect {
    display: none;
  }
}
</style>
