<script setup lang="ts">
defineOptions({ name: 'JcBeam' })

/**
 * 流光共享层（JcInput / JcTextarea / JcSelect 共用）：
 * 封装「边框流光环(.jc-beam) + 内部光晕(.jc-beam-glow)」两层结构，避免各控件重复手写光晕标记。
 * - 把 beamStyle 的 CSS 变量（--jc-beam-* / --jc-glow-*）同时注入两层 → 流光与光晕同尺寸/同渐变/同速度严格同步
 * - 组件渲染为 fragment（两个 span 直接成为宿主子元素），position:absolute inset:0 以宿主为定位上下文
 * - glow=false 时只渲染流光环（零额外开销）
 */
defineProps<{
  /** 是否开启内部光晕（与流光同路径/同速/同色，头部淡入过渡） */
  glow?: boolean
  /** beamStyle：useBeam / JcBorderBeam 输出的 CSS 变量，应用到流光环与光晕两层 */
  style?: Record<string, string>
}>()
</script>

<template>
  <span class="jc-beam" :style="style" aria-hidden="true">
    <span class="jc-beam__effect" />
  </span>
  <span v-if="glow" class="jc-beam-glow" :style="style" aria-hidden="true">
    <span class="jc-beam-glow__effect" />
  </span>
</template>
