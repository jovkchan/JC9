import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { JcBorderBeamColor } from '../components/ui/JcBorderBeam.vue'

/** 流光渐变尾部占位（percent 0~100 映射到此，为尾部透明过渡预留空间） */
export const BEAM_COLOR_STOP_PERCENT = 70

/** 数字补 px；纯数字字符串（如 "4"）也补 px，否则 CSS 变量无单位会失效 */
const unit = (v: number | string) => {
  if (typeof v === 'number') return `${v}px`
  const s = String(v).trim()
  return /^\d+(\.\d+)?$/.test(s) ? `${s}px` : v
}

/** 构建流光渐变（对齐 antd util.ts）：percent 0~100 → 0~70，尾部补透明渐变 */
export function buildBeamGradient(
  color: JcBorderBeamColor | undefined,
  angle: string,
): string | undefined {
  if (!color) return undefined
  const items = Array.isArray(color) ? color : [{ color, percent: 0 }]
  if (items.length === 0) return undefined

  const mapped = items.map((it) => ({
    color: it.color,
    percent: Number(
      ((Math.min(Math.max(it.percent, 0), 100) / 100) * BEAM_COLOR_STOP_PERCENT).toFixed(2),
    ),
  }))
  const last = mapped[mapped.length - 1]
  const stops =
    last.percent >= BEAM_COLOR_STOP_PERCENT
      ? mapped
      : [...mapped, { color: last.color, percent: BEAM_COLOR_STOP_PERCENT }]

  return `linear-gradient(${angle}, ${stops.map((s) => `${s.color} ${s.percent}%`).join(', ')}, transparent)`
}

/** 构建光晕渐变：与流光同色系，但头部/尾部淡入淡出。
 *  光晕元素比流光长（1.6×）且 offset-anchor 靠后 → 头部超出流光，形成柔和过渡（不再是硬蓝头） */
export function buildGlowGradient(
  color: JcBorderBeamColor | undefined,
  angle: string,
): string | undefined {
  if (!color) return undefined
  const items = Array.isArray(color) ? color : [{ color, percent: 0 }]
  if (items.length === 0) return undefined
  // 主色带铺在 4% ~ 96%（头尾各留约 4px 淡出 → 若隐若现的泛光）
  const mapped = items.map((it) => ({
    color: it.color,
    percent: 4 + (Math.min(Math.max(it.percent, 0), 100) / 100) * 92,
  }))
  const first = mapped[0]
  const last = mapped[mapped.length - 1]
  const stops: { color: string; percent: number }[] = [{ color: first.color, percent: 4 }]
  for (const s of mapped) {
    if (s.percent > 4 && s.percent < 96) stops.push(s)
  }
  stops.push({ color: last.color, percent: 96 })
  return `linear-gradient(${angle}, transparent 0%, ${stops
    .map((s) => `${s.color} ${s.percent}%`)
    .join(', ')}, transparent 100%)`
}

export interface UseBeamOptions {
  /** 是否开启流光 */
  enabled: () => boolean
  /** 流光颜色（单色或渐变停靠点数组） */
  color: () => JcBorderBeamColor | undefined
  /** 渐变方向，如 'to left' 或 '-225deg' */
  angle: () => string
  /** 拐角变速：true=拐角轻微加速 / false=匀速 */
  accelerate: () => boolean
  /** 宿主元素（用于测量宽度计算光束长度） */
  root: () => HTMLElement | undefined
  /** 光束长度 = 宿主宽度 × 比例（如 0.4）；细长条高度固定 < 高度，避免上下双轨 */
  sizeRatio: () => number
  /** 是否开启内部光晕（与流光同路径/同速/同色） */
  glow: () => boolean
  /** 光晕模糊半径（数字按 px；缺省用 --jc-glow-blur token） */
  glowBlur: () => number | string | undefined
  /** 光晕不透明度 0~1（缺省用 --jc-glow-opacity token） */
  glowOpacity: () => number | undefined
}

/**
 * 表单控件共用流光逻辑（JcInput / JcTextarea / JcSelect）：
 * 渐变构建 + 光束长度随宽度实时变化（ResizeObserver）+ beamStyle 变量。
 * 输出与全局 `.jc-beam` 共享样式约定的 CSS 变量对齐：
 * `--jc-beam-size` / `--jc-beam-gradient` / `--jc-beam-anim`
 */
export function useBeam(opts: UseBeamOptions) {
  const beamGradient = computed(() => buildBeamGradient(opts.color(), opts.angle()))
  const glowGradient = computed(() => buildGlowGradient(opts.color(), opts.angle()))

  const beamSizeRef = ref('100px')
  let ro: ResizeObserver | undefined

  function syncBeamSize() {
    const el = opts.root()
    if (!opts.enabled() || !el) return
    beamSizeRef.value = `${Math.round(el.clientWidth * opts.sizeRatio())}px`
  }

  onMounted(() => {
    if (!opts.enabled()) return
    syncBeamSize()
    const el = opts.root()
    if (el) {
      ro = new ResizeObserver(syncBeamSize)
      ro.observe(el)
    }
  })
  onBeforeUnmount(() => ro?.disconnect())

  const beamStyle = computed(() => {
    const glow = opts.glow()
    const blur = opts.glowBlur()
    // 光晕前后各外扩 4px（若隐若现的泛光）：宽度 = 流光 + 8px，offset-anchor 使 4px 均匀落在流光头尾两侧
    const glowPad = 4
    const size = parseFloat(beamSizeRef.value) || 100
    const glowSize = size + glowPad * 2
    return {
      '--jc-beam-size': beamSizeRef.value,
      '--jc-beam-gradient': beamGradient.value,
      '--jc-beam-anim': opts.accelerate() ? 'jc-beam-move-acc' : 'jc-beam-move',
      ...(glow
        ? {
            '--jc-glow-blur': blur !== undefined ? unit(blur) : 'var(--jc-glow-blur, 6px)',
            '--jc-glow-opacity': String(opts.glowOpacity() ?? 0.65),
            '--jc-glow-size': `${glowSize}px`,
            '--jc-glow-anchor': `${((0.9 * size + glowPad) / glowSize) * 100}%`,
            ...(glowGradient.value ? { '--jc-glow-gradient': glowGradient.value } : {}),
          }
        : {}),
    } as Record<string, string>
  })

  return { beamGradient, glowGradient, beamStyle }
}
