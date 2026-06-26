<script setup lang="ts">
import { ref, computed, watch } from 'vue'

// 核心单一数据源：RGB + Alpha
const r = ref(138)
const g = ref(88)
const b = ref(255)
const a = ref(1.0) // Alpha 范围 0.0 - 1.0

// HEX 输入展示
const hexInput = ref('#8a58ff')

// HSL 独立响应状态（避免黑白等极端色下的色相 H 抖动）
const hslH = ref(258)
const hslS = ref(100)
const hslL = ref(67)

// CMYK 独立响应状态（0-100%）
const cmykC = ref(46)
const cmykM = ref(65)
const cmykY = ref(0)
const cmykK = ref(0)

// HTML5 原生拾色器绑定值 (取色盘只支持6位标准 HEX)
const colorPickerVal = computed({
  get() {
    return rgbToHex6(r.value, g.value, b.value)
  },
  set(val: string) {
    const parsed = hexToRgba(val)
    if (parsed) {
      r.value = parsed.r
      g.value = parsed.g
      b.value = parsed.b
      // 保持当前的透明度不变
    }
  }
})

// === 核心工具转换函数 ===

// 6位 HEX (Picker使用)
function rgbToHex6(red: number, green: number, blue: number): string {
  const toHex = (c: number) => c.toString(16).padStart(2, '0')
  return `#${toHex(red)}${toHex(green)}${toHex(blue)}`
}

// 支持 6位 / 8位 HEX
function rgbToHex(red: number, green: number, blue: number, alpha: number): string {
  const toHex = (c: number) => c.toString(16).padStart(2, '0')
  const base = `#${toHex(red)}${toHex(green)}${toHex(blue)}`
  if (alpha === 1) return base
  const alphaHex = toHex(Math.round(alpha * 255))
  return base + alphaHex
}

function hexToRgba(hex: string): { r: number; g: number; b: number; a: number } | null {
  const cleanHex = hex.replace(/^#/, '').trim()
  
  // 3位 或 4位 HEX
  if (cleanHex.length === 3 || cleanHex.length === 4) {
    const rPart = parseInt(cleanHex[0] + cleanHex[0], 16)
    const gPart = parseInt(cleanHex[1] + cleanHex[1], 16)
    const bPart = parseInt(cleanHex[2] + cleanHex[2], 16)
    let aPart = 1.0
    if (cleanHex.length === 4) {
      aPart = parseFloat((parseInt(cleanHex[3] + cleanHex[3], 16) / 255).toFixed(2))
    }
    return { r: rPart, g: gPart, b: bPart, a: aPart }
  }

  // 6位 或 8位 HEX
  if (cleanHex.length === 6 || cleanHex.length === 8) {
    const rPart = parseInt(cleanHex.substring(0, 2), 16)
    const gPart = parseInt(cleanHex.substring(2, 4), 16)
    const bPart = parseInt(cleanHex.substring(4, 6), 16)
    let aPart = 1.0
    if (cleanHex.length === 8) {
      aPart = parseFloat((parseInt(cleanHex.substring(6, 8), 16) / 255).toFixed(2))
    }
    return { r: rPart, g: gPart, b: bPart, a: aPart }
  }

  return null
}

function rgbToHsl(red: number, green: number, blue: number) {
  const redNorm = red / 255
  const greenNorm = green / 255
  const blueNorm = blue / 255

  const max = Math.max(redNorm, greenNorm, blueNorm)
  const min = Math.min(redNorm, greenNorm, blueNorm)
  let h = 0
  let s = 0
  const l = (max + min) / 2

  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    switch (max) {
      case redNorm:
        h = (greenNorm - blueNorm) / d + (greenNorm < blueNorm ? 6 : 0)
        break
      case greenNorm:
        h = (blueNorm - redNorm) / d + 2
        break
      case blueNorm:
        h = (redNorm - greenNorm) / d + 4
        break
    }
    h /= 6
  }

  return {
    h: Math.round(h * 360),
    s: Math.round(s * 100),
    l: Math.round(l * 100)
  }
}

function hslToRgb(h: number, s: number, l: number) {
  const hNorm = h / 360
  const sNorm = s / 100
  const lNorm = l / 100

  let red = lNorm
  let green = lNorm
  let blue = lNorm

  if (sNorm !== 0) {
    const hue2rgb = (pVal: number, qVal: number, t: number) => {
      let tTemp = t
      if (tTemp < 0) tTemp += 1
      if (tTemp > 1) tTemp -= 1
      if (tTemp < 1 / 6) return pVal + (qVal - pVal) * 6 * tTemp
      if (tTemp < 1 / 2) return qVal
      if (tTemp < 2 / 3) return pVal + (qVal - pVal) * (2 / 3 - tTemp) * 6
      return pVal
    }

    const q = lNorm < 0.5 ? lNorm * (1 + sNorm) : lNorm + sNorm - lNorm * sNorm
    const p = 2 * lNorm - q

    red = hue2rgb(p, q, hNorm + 1 / 3)
    green = hue2rgb(p, q, hNorm)
    blue = hue2rgb(p, q, hNorm - 1 / 3)
  }

  return {
    r: Math.round(red * 255),
    g: Math.round(green * 255),
    b: Math.round(blue * 255)
  }
}

// RGB 与 CMYK 互转数学公式
function rgbToCmyk(red: number, green: number, blue: number) {
  const rNorm = red / 255
  const gNorm = green / 255
  const bNorm = blue / 255

  const k = 1 - Math.max(rNorm, gNorm, bNorm)
  if (k === 1) {
    return { c: 0, m: 0, y: 0, k: 100 }
  }

  const c = (1 - rNorm - k) / (1 - k)
  const m = (1 - gNorm - k) / (1 - k)
  const y = (1 - bNorm - k) / (1 - k)

  return {
    c: Math.round(c * 100),
    m: Math.round(m * 100),
    y: Math.round(y * 100),
    k: Math.round(k * 100)
  }
}

function cmykToRgb(c: number, m: number, y: number, k: number) {
  const cNorm = c / 100
  const mNorm = m / 100
  const yNorm = y / 100
  const kNorm = k / 100

  const red = 255 * (1 - cNorm) * (1 - kNorm)
  const green = 255 * (1 - mNorm) * (1 - kNorm)
  const blue = 255 * (1 - yNorm) * (1 - kNorm)

  return {
    r: Math.round(red),
    g: Math.round(green),
    b: Math.round(blue)
  }
}

// === 联动核心 Watcher ===
watch([r, g, b, a], ([newR, newG, newB, newA]) => {
  // 1. 同步 HEX
  const computedHex = rgbToHex(newR, newG, newB, newA)
  if (hexInput.value.toLowerCase() !== computedHex.toLowerCase()) {
    hexInput.value = computedHex
  }

  // 2. 同步 HSL
  const computedHsl = rgbToHsl(newR, newG, newB)
  if (computedHsl.s > 0) {
    hslH.value = computedHsl.h
  }
  hslS.value = computedHsl.s
  hslL.value = computedHsl.l

  // 3. 同步 CMYK
  const computedCmyk = rgbToCmyk(newR, newG, newB)
  cmykC.value = computedCmyk.c
  cmykM.value = computedCmyk.m
  cmykY.value = computedCmyk.y
  cmykK.value = computedCmyk.k
}, { immediate: true })

// 监听主动 HEX 文本框输入
function handleHexInput() {
  let val = hexInput.value.trim()
  if (!val.startsWith('#')) {
    val = '#' + val
  }
  const parsed = hexToRgba(val)
  if (parsed) {
    r.value = parsed.r
    g.value = parsed.g
    b.value = parsed.b
    a.value = parsed.a
  }
}

// 监听主动 HSL 进度条调节
function handleHslSlider() {
  const parsed = hslToRgb(hslH.value, hslS.value, hslL.value)
  r.value = parsed.r
  g.value = parsed.g
  b.value = parsed.b
}

// 监听主动 CMYK 进度条调节
function handleCmykSlider() {
  const parsed = cmykToRgb(cmykC.value, cmykM.value, cmykY.value, cmykK.value)
  r.value = parsed.r
  g.value = parsed.g
  b.value = parsed.b
}

// 一键复制
function copyText(text: string) {
  navigator.clipboard.writeText(text)
}

// 动态输出字符串计算属性
const hexString = computed(() => hexInput.value.toUpperCase())
const rgbaString = computed(() => a.value === 1.0 ? `rgb(${r.value}, ${g.value}, ${b.value})` : `rgba(${r.value}, ${g.value}, ${b.value}, ${a.value})`)
const hslaString = computed(() => a.value === 1.0 ? `hsl(${hslH.value}, ${hslS.value}%, ${hslL.value}%)` : `hsla(${hslH.value}, ${hslS.value}%, ${hslL.value}%, ${a.value})`)
const cmykString = computed(() => `cmyk(${cmykC.value}%, ${cmykM.value}%, ${cmykY.value}%, ${cmykK.value}%)`)
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">颜色转换器</div>
    </div>
    <div class="tool-body-split flex-row">
      <!-- 左侧：颜色预览（棋盘底）与文本输出 -->
      <div class="color-preview-pane">
        <!-- 棋盘格效果的透明盒底，套用 RGBA 背景层 -->
        <div class="preview-box-chessboard">
          <div class="color-filled-layer" :style="{ backgroundColor: rgbaString }"></div>
          <input type="color" v-model="colorPickerVal" class="hidden-picker" id="htmlColorPicker" />
          <label for="htmlColorPicker" class="picker-trigger">
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
            点击取色盘
          </label>
        </div>
        
        <div class="color-text-display">
          <div class="text-row">
            <span class="format-label">HEX</span>
            <span class="value-text">{{ hexString }}</span>
            <button class="copy-btn" @click="copyText(hexString)" title="复制 HEX">复制</button>
          </div>
          <div class="text-row">
            <span class="format-label">RGBA</span>
            <span class="value-text">{{ rgbaString }}</span>
            <button class="copy-btn" @click="copyText(rgbaString)" title="复制 RGBA">复制</button>
          </div>
          <div class="text-row">
            <span class="format-label">HSLA</span>
            <span class="value-text">{{ hslaString }}</span>
            <button class="copy-btn" @click="copyText(hslaString)" title="复制 HSLA">复制</button>
          </div>
          <div class="text-row">
            <span class="format-label">CMYK</span>
            <span class="value-text">{{ cmykString }}</span>
            <button class="copy-btn" @click="copyText(cmykString)" title="复制 CMYK">复制</button>
          </div>
        </div>
      </div>

      <!-- 右侧：控制条 -->
      <div class="color-sliders-pane">
        <!-- HEX 文本输入 -->
        <div class="slider-group">
          <div class="slider-header">HEX 值</div>
          <input type="text" v-model="hexInput" @input="handleHexInput" class="hex-text-input" placeholder="#FFFFFF" />
        </div>

        <!-- RGB + Alpha 调节 -->
        <div class="slider-group">
          <div class="slider-header">RGB + Alpha (屏幕调色)</div>
          <div class="slider-row">
            <span class="channel-label red">R</span>
            <input type="range" min="0" max="255" v-model.number="r" class="color-slider" />
            <input type="number" min="0" max="255" v-model.number="r" class="num-input" />
          </div>
          <div class="slider-row">
            <span class="channel-label green">G</span>
            <input type="range" min="0" max="255" v-model.number="g" class="color-slider" />
            <input type="number" min="0" max="255" v-model.number="g" class="num-input" />
          </div>
          <div class="slider-row">
            <span class="channel-label blue">B</span>
            <input type="range" min="0" max="255" v-model.number="b" class="color-slider" />
            <input type="number" min="0" max="255" v-model.number="b" class="num-input" />
          </div>
          <div class="slider-row">
            <span class="channel-label alpha">A</span>
            <input type="range" min="0" max="1" step="0.01" v-model.number="a" class="color-slider" />
            <input type="number" min="0" max="1" step="0.01" v-model.number="a" class="num-input" />
          </div>
        </div>

        <!-- HSL 调节 -->
        <div class="slider-group">
          <div class="slider-header">HSL (色彩/饱和度/亮度)</div>
          <div class="slider-row">
            <span class="channel-label">H</span>
            <input type="range" min="0" max="360" v-model.number="hslH" @input="handleHslSlider" class="color-slider" />
            <input type="number" min="0" max="360" v-model.number="hslH" @input="handleHslSlider" class="num-input" />
            <span class="unit">°</span>
          </div>
          <div class="slider-row">
            <span class="channel-label">S</span>
            <input type="range" min="0" max="100" v-model.number="hslS" @input="handleHslSlider" class="color-slider" />
            <input type="number" min="0" max="100" v-model.number="hslS" @input="handleHslSlider" class="num-input" />
            <span class="unit">%</span>
          </div>
          <div class="slider-row">
            <span class="channel-label">L</span>
            <input type="range" min="0" max="100" v-model.number="hslL" @input="handleHslSlider" class="color-slider" />
            <input type="number" min="0" max="100" v-model.number="hslL" @input="handleHslSlider" class="num-input" />
            <span class="unit">%</span>
          </div>
        </div>

        <!-- CMYK 印刷色彩调节 -->
        <div class="slider-group">
          <div class="slider-header">CMYK (印刷色彩模型)</div>
          <div class="slider-row">
            <span class="channel-label cmyk-c">C</span>
            <input type="range" min="0" max="100" v-model.number="cmykC" @input="handleCmykSlider" class="color-slider" />
            <input type="number" min="0" max="100" v-model.number="cmykC" @input="handleCmykSlider" class="num-input" />
            <span class="unit">%</span>
          </div>
          <div class="slider-row">
            <span class="channel-label cmyk-m">M</span>
            <input type="range" min="0" max="100" v-model.number="cmykM" @input="handleCmykSlider" class="color-slider" />
            <input type="number" min="0" max="100" v-model.number="cmykM" @input="handleCmykSlider" class="num-input" />
            <span class="unit">%</span>
          </div>
          <div class="slider-row">
            <span class="channel-label cmyk-y">Y</span>
            <input type="range" min="0" max="100" v-model.number="cmykY" @input="handleCmykSlider" class="color-slider" />
            <input type="number" min="0" max="100" v-model.number="cmykY" @input="handleCmykSlider" class="num-input" />
            <span class="unit">%</span>
          </div>
          <div class="slider-row">
            <span class="channel-label cmyk-k">K</span>
            <input type="range" min="0" max="100" v-model.number="cmykK" @input="handleCmykSlider" class="color-slider" />
            <input type="number" min="0" max="100" v-model.number="cmykK" @input="handleCmykSlider" class="num-input" />
            <span class="unit">%</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.tool-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  padding: 12px;
  background: var(--jc-bg-app);
  overflow: hidden;
}
.tool-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  flex-shrink: 0;
}
.tool-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 20px;
  min-height: 0;
  &.flex-row {
    flex-direction: row;
  }
}
.color-preview-pane {
  display: flex;
  flex-direction: column;
  flex: 0 0 280px;
  gap: 12px;
}

/* 棋盘格防透视盒效果 */
.preview-box-chessboard {
  position: relative;
  flex: 1;
  min-height: 150px;
  border-radius: 6px;
  border: 1px solid var(--jc-border-default);
  box-shadow: inset 0 2px 8px rgba(0,0,0,0.2);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  
  /* 制作调色专用透明棋盘格背景 */
  background-color: #37373d;
  background-image: linear-gradient(45deg, #252526 25%, transparent 25%), 
                    linear-gradient(-45deg, #252526 25%, transparent 25%), 
                    linear-gradient(45deg, transparent 75%, #252526 75%), 
                    linear-gradient(-45deg, transparent 75%, #252526 75%);
  background-size: 14px 14px;
  background-position: 0 0, 0 7px, 7px -7px, -7px 0px;
}

.color-filled-layer {
  position: absolute;
  inset: 0;
  z-index: 1;
}

.hidden-picker {
  position: absolute;
  width: 0;
  height: 0;
  opacity: 0;
  pointer-events: none;
}
.picker-trigger {
  position: relative;
  z-index: 2;
  cursor: pointer;
  background: rgba(0, 0, 0, 0.65);
  color: #fff;
  padding: 6px 12px;
  border-radius: 20px;
  font-size: 11px;
  display: flex;
  align-items: center;
  gap: 6px;
  user-select: none;
  backdrop-filter: blur(4px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: all 0.2s;
  &:hover {
    background: rgba(0, 0, 0, 0.85);
    border-color: var(--jc-color-accent);
  }
  svg {
    stroke: var(--jc-color-accent);
  }
}
.color-text-display {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.text-row {
  display: flex;
  align-items: center;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 6px 8px;
  border-radius: 4px;
}
.format-label {
  font-size: 10px;
  font-weight: 700;
  color: var(--jc-text-secondary);
  width: 45px;
}
.value-text {
  flex: 1;
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  color: var(--jc-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding-right: 4px;
}
.copy-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 2px 8px;
  font-size: 10px;
  cursor: pointer;
  border-radius: 2px;
  flex-shrink: 0;
  &:hover {
    background: var(--jc-color-accent);
    color: #fff;
  }
}

.color-sliders-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 16px;
  border-radius: 6px;
  overflow-y: auto;
}
.slider-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.slider-header {
  font-size: 11px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--jc-border-default);
  padding-bottom: 4px;
}
.hex-text-input {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 13px;
  padding: 6px 10px;
  outline: none;
  border-radius: 3px;
  width: 150px;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.slider-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.channel-label {
  font-size: 11px;
  font-weight: 700;
  width: 18px;
  text-align: center;
  color: var(--jc-text-secondary);
  &.red { color: #f44747; }
  &.green { color: #4ec9b0; }
  &.blue { color: #007acc; }
  &.alpha { color: var(--jc-color-warning); }
  
  /* CMYK标签颜色，提供高可见度颜色 */
  &.cmyk-c { color: #00ffff; text-shadow: 0 0 1px #000; }
  &.cmyk-m { color: #ff00ff; text-shadow: 0 0 1px #000; }
  &.cmyk-y { color: #ffff00; text-shadow: 0 0 1px #000; }
  &.cmyk-k { color: #888888; }
}
.color-slider {
  flex: 1;
  accent-color: var(--jc-color-accent);
  cursor: pointer;
}
.num-input {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  width: 55px;
  padding: 2px 4px;
  font-size: 11px;
  text-align: right;
  outline: none;
  border-radius: 2px;
  &::-webkit-inner-spin-button,
  &::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.unit {
  font-size: 11px;
  color: var(--jc-text-secondary);
  width: 10px;
}
</style>
