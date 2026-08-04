import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { buildBeamGradient, buildGlowGradient } from '@/composables/useBeam'

export interface BeamGradientStop {
  color: string
  percent: number
}

export interface EffectConfig {
  /** 流光总开关 */
  enabled: boolean
  /** 内部光晕开关 */
  glow: boolean
  /** 各控件类型是否启用流光（含其光晕） */
  input: boolean
  textarea: boolean
  select: boolean
  /** 光束统一线性速度（px/s）：各组件圈时长 = 路径周长 ÷ 速度 → 大小组件速度一致 */
  speed: number
  /** 拐角变速 */
  accelerate: boolean
  /** 光晕大小（模糊半径 px） */
  glowBlur: number
  /** 光晕透明度 0~1 */
  glowOpacity: number
  /** 全局渐变颜色（流光/光晕共用停靠点） */
  colors: BeamGradientStop[]
  /** 渐变方向，如 'to left' 或 '-225deg' */
  angle: string
}

export const defaultEffectConfig: EffectConfig = {
  enabled: true,
  glow: true,
  input: true,
  textarea: true,
  select: true,
  speed: 90,
  accelerate: false,
  glowBlur: 6,
  glowOpacity: 0.65,
  colors: [
    { color: '#5b7cff', percent: 0 },
    { color: '#8a58ff', percent: 40 },
    { color: '#a878ff', percent: 68 },
  ],
  angle: 'to left',
}

/**
 * 全局动画效果配置 Store（流光/光晕）——后续新增更多效果的动态配置基础：
 * - 通过给 <html> 写 CSS 变量（--jc-*）全局生效；组件实例 props（beam-color/beam-duration 等）优先于全局值
 * - 通过 html class 控制启用/禁用（jc-beam-off / jc-glow-off / jc-beam-input-off / textarea / select）
 * - 持久化到 DATA 目录 effect-config.json（Tauri 命令），并镜像到 localStorage 兜底
 * - 实时：config 变化 → 立即 apply（CSS 变量）+ 防抖持久化
 */
export const useEffectConfig = defineStore('effectConfig', () => {
  const config = ref<EffectConfig>({ ...defaultEffectConfig })

  /** 把当前配置应用到 <html>（CSS 变量 + 启用/禁用 class） */
  function apply() {
    const el = document.documentElement
    const c = config.value
    // 统一线速度：组件按自身周长计算圈时长；这里设一个参考周长(700px)的兜底值
    el.style.setProperty('--jc-beam-speed', `${c.speed}px/s`)
    el.style.setProperty('--jc-beam-duration', `${Math.max(1, Math.round(700 / c.speed))}s`)
    el.style.setProperty('--jc-beam-anim', c.accelerate ? 'jc-beam-move-acc' : 'jc-beam-move')
    el.style.setProperty('--jc-glow-blur', `${c.glowBlur}px`)
    el.style.setProperty('--jc-glow-opacity', String(c.glowOpacity))
    const beamGrad = buildBeamGradient(c.colors, c.angle)
    const glowGrad = buildGlowGradient(c.colors, c.angle)
    if (beamGrad) el.style.setProperty('--jc-beam-gradient', beamGrad)
    else el.style.removeProperty('--jc-beam-gradient')
    if (glowGrad) el.style.setProperty('--jc-glow-gradient', glowGrad)
    else el.style.removeProperty('--jc-glow-gradient')
    // 启用/禁用开关
    el.classList.toggle('jc-beam-off', !c.enabled)
    el.classList.toggle('jc-glow-off', !c.glow)
    el.classList.toggle('jc-beam-input-off', !c.input)
    el.classList.toggle('jc-beam-textarea-off', !c.textarea)
    el.classList.toggle('jc-beam-select-off', !c.select)
    // 通知已挂载的流光组件重新计算圈时长（速度/尺寸变化）
    if (typeof window !== 'undefined') window.dispatchEvent(new CustomEvent('jc-effect-config'))
  }

  let saveTimer: number | undefined
  /** 持久化：localStorage 兜底 + DATA 目录 JSON（防抖） */
  function persist() {
    const json = JSON.stringify(config.value)
    try { localStorage.setItem('jc9-effect-config', json) } catch { /* ignore */ }
    window.clearTimeout(saveTimer)
    saveTimer = window.setTimeout(() => {
      invoke('save_effect_config', { config: json }).catch(() => {})
    }, 300)
  }

  /** 载入配置（localStorage 优先即时，其次 DATA JSON）并应用 */
  async function load() {
    let data: Partial<EffectConfig> | null = null
    try {
      const saved = localStorage.getItem('jc9-effect-config')
      if (saved) data = JSON.parse(saved)
    } catch { /* ignore */ }
    if (!data) {
      try {
        const json = await invoke<string>('get_effect_config')
        const parsed = JSON.parse(json)
        if (parsed && typeof parsed === 'object') data = parsed
      } catch { /* ignore */ }
    }
    if (data) {
      // 迁移：旧版 duration（固定一圈时长）→ speed（统一线速度，参考周长 700px）
      const legacy = data as Partial<EffectConfig> & { duration?: number }
      if (typeof data.speed !== 'number' && typeof legacy.duration === 'number') {
        data.speed = Math.max(20, Math.round(700 / legacy.duration))
      }
      config.value = {
        ...defaultEffectConfig,
        ...data,
        colors: Array.isArray(data.colors) && data.colors.length
          ? data.colors
          : defaultEffectConfig.colors.map((s) => ({ ...s })),
      }
    }
    apply()
  }

  // 配置变化 → 实时应用 + 持久化
  watch(config, () => { apply(); persist() }, { deep: true })

  // 颜色操作辅助
  function addColor() {
    config.value.colors.push({ color: '#8a58ff', percent: 50 })
  }
  function removeColor(index: number) {
    if (config.value.colors.length > 1) config.value.colors.splice(index, 1)
  }
  function reset() {
    config.value = {
      ...defaultEffectConfig,
      colors: defaultEffectConfig.colors.map((s) => ({ ...s })),
    }
  }

  // 初始：先按默认应用（避免闪烁），再异步载入保存值
  apply()
  load()

  return { config, apply, load, addColor, removeColor, reset, persist }
})
