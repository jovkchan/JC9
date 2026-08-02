<script setup lang="ts">
import { ref, watch } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'

// 基准设置
const rootFontSize = ref(16)
const parentFontSize = ref(16)
const viewportWidth = ref(1920)
const viewportHeight = ref(1080)

// 转换主数值 (以 px 为核心进行同步)
const pxVal = ref<number | ''>(16)
const remVal = ref<number | ''>(1)
const emVal = ref<number | ''>(1)
const vwVal = ref<number | ''>(0.8333)
const vhVal = ref<number | ''>(1.4815)

// 滑块值用于可视化
const sliderVal = ref(100)

// 标记正在输入更新的源，防止循环更新
let activeSource = ''

function updateFromPx(val: number | '') {
  if (val === '') {
    pxVal.value = ''
    remVal.value = ''
    emVal.value = ''
    vwVal.value = ''
    vhVal.value = ''
    return
  }
  pxVal.value = Number(val)
  remVal.value = Number((pxVal.value / rootFontSize.value).toFixed(4))
  emVal.value = Number((pxVal.value / parentFontSize.value).toFixed(4))
  vwVal.value = Number(((pxVal.value / viewportWidth.value) * 100).toFixed(4))
  vhVal.value = Number(((pxVal.value / viewportHeight.value) * 100).toFixed(4))
}

function updateFromRem(val: number | '') {
  if (val === '') {
    updateFromPx('')
    return
  }
  remVal.value = Number(val)
  const px = remVal.value * rootFontSize.value
  pxVal.value = Number(px.toFixed(2))
  emVal.value = Number((px / parentFontSize.value).toFixed(4))
  vwVal.value = Number(((px / viewportWidth.value) * 100).toFixed(4))
  vhVal.value = Number(((px / viewportHeight.value) * 100).toFixed(4))
}

function updateFromEm(val: number | '') {
  if (val === '') {
    updateFromPx('')
    return
  }
  emVal.value = Number(val)
  const px = emVal.value * parentFontSize.value
  pxVal.value = Number(px.toFixed(2))
  remVal.value = Number((px / rootFontSize.value).toFixed(4))
  vwVal.value = Number(((px / viewportWidth.value) * 100).toFixed(4))
  vhVal.value = Number(((px / viewportHeight.value) * 100).toFixed(4))
}

function updateFromVw(val: number | '') {
  if (val === '') {
    updateFromPx('')
    return
  }
  vwVal.value = Number(val)
  const px = (vwVal.value / 100) * viewportWidth.value
  pxVal.value = Number(px.toFixed(2))
  remVal.value = Number((px / rootFontSize.value).toFixed(4))
  emVal.value = Number((px / parentFontSize.value).toFixed(4))
  vhVal.value = Number(((px / viewportHeight.value) * 100).toFixed(4))
}

function updateFromVh(val: number | '') {
  if (val === '') {
    updateFromPx('')
    return
  }
  vhVal.value = Number(val)
  const px = (vhVal.value / 100) * viewportHeight.value
  pxVal.value = Number(px.toFixed(2))
  remVal.value = Number((px / rootFontSize.value).toFixed(4))
  emVal.value = Number((px / parentFontSize.value).toFixed(4))
  vwVal.value = Number(((px / viewportWidth.value) * 100).toFixed(4))
}

// 监听各个输入值的变化
watch(pxVal, (newVal) => {
  if (activeSource === 'px') return
  activeSource = 'px'
  updateFromPx(newVal)
  activeSource = ''
})

watch(remVal, (newVal) => {
  if (activeSource === 'rem') return
  activeSource = 'rem'
  updateFromRem(newVal)
  activeSource = ''
})

watch(emVal, (newVal) => {
  if (activeSource === 'em') return
  activeSource = 'em'
  updateFromEm(newVal)
  activeSource = ''
})

watch(vwVal, (newVal) => {
  if (activeSource === 'vw') return
  activeSource = 'vw'
  updateFromVw(newVal)
  activeSource = ''
})

watch(vhVal, (newVal) => {
  if (activeSource === 'vh') return
  activeSource = 'vh'
  updateFromVh(newVal)
  activeSource = ''
})

// 基准值变化时，重新以当前的 px 值为准计算其他值
watch([rootFontSize, parentFontSize, viewportWidth, viewportHeight], () => {
  if (pxVal.value !== '') {
    updateFromPx(pxVal.value)
  }
})

// 滑块改变同步到 px 核心
watch(sliderVal, (newVal) => {
  activeSource = 'px'
  updateFromPx(newVal)
  activeSource = ''
})

// 快捷转换推荐
const presetPx = [12, 14, 16, 18, 20, 24, 28, 32, 40, 48, 64]

function selectPreset(px: number) {
  sliderVal.value = px
  activeSource = 'px'
  updateFromPx(px)
  activeSource = ''
}
</script>

<template>
  <ToolShell title="CSS 单位换算器" subtitle="PX、REM、EM、VW、VH 实时联动转换">
    <div class="tool-body">
      <!-- 顶部基准配置卡片 -->
      <div class="config-section card">
        <div class="card-title">换算基准配置 (CSS Environment)</div>
        <div class="grid-inputs">
          <div class="input-field">
            <label>基准字号 (Root Rem Base)</label>
            <div class="input-with-unit">
              <input type="number" v-model.number="rootFontSize" min="1" />
              <span class="unit">px</span>
            </div>
            <div class="field-desc">html { font-size: Xpx }</div>
          </div>
          <div class="input-field">
            <label>父元素字号 (Parent Em Base)</label>
            <div class="input-with-unit">
              <input type="number" v-model.number="parentFontSize" min="1" />
              <span class="unit">px</span>
            </div>
            <div class="field-desc">用于计算当前元素的 em 值</div>
          </div>
          <div class="input-field">
            <label>视口宽度 (Viewport Width)</label>
            <div class="input-with-unit">
              <input type="number" v-model.number="viewportWidth" min="1" />
              <span class="unit">px</span>
            </div>
            <div class="field-desc">用于计算 100vw = Xpx</div>
          </div>
          <div class="input-field">
            <label>视口高度 (Viewport Height)</label>
            <div class="input-with-unit">
              <input type="number" v-model.number="viewportHeight" min="1" />
              <span class="unit">px</span>
            </div>
            <div class="field-desc">用于计算 100vh = Xpx</div>
          </div>
        </div>
      </div>

      <!-- 主联动换算区域 -->
      <div class="converter-section grid-2">
        <!-- 实时输入栏 -->
        <div class="converter-inputs card">
          <div class="card-title">数值换算 (输入任一数值，其它实时计算)</div>
          
          <div class="row-inputs">
            <div class="val-input-group">
              <span class="val-label px-label">PX</span>
              <input type="number" v-model.number="pxVal" placeholder="0" class="input-highlight" />
              <span class="val-unit-tag">像素单位</span>
            </div>

            <div class="val-input-group">
              <span class="val-label rem-label">REM</span>
              <input type="number" v-model.number="remVal" placeholder="0" />
              <span class="val-unit-tag">相对于根元素</span>
            </div>

            <div class="val-input-group">
              <span class="val-label em-label">EM</span>
              <input type="number" v-model.number="emVal" placeholder="0" />
              <span class="val-unit-tag">相对于父元素</span>
            </div>

            <div class="val-input-group">
              <span class="val-label vw-label">VW</span>
              <input type="number" v-model.number="vwVal" placeholder="0" />
              <span class="val-unit-tag">视口宽度 %</span>
            </div>

            <div class="val-input-group">
              <span class="val-label vh-label">VH</span>
              <input type="number" v-model.number="vhVal" placeholder="0" />
              <span class="val-unit-tag">视口高度 %</span>
            </div>
          </div>
        </div>

        <!-- 可视化演示与滑块 -->
        <div class="visualizer-panel card">
          <div class="card-title">物理大小可视化感受 (Visualizer)</div>
          
          <div class="slider-control">
            <div class="slider-header">
              <span>当前 PX 大小: <strong>{{ pxVal || 0 }}px</strong></span>
              <span>拖动调节</span>
            </div>
            <input type="range" v-model.number="sliderVal" min="1" max="500" class="premium-slider" />
          </div>

          <div class="demo-box-container">
            <div class="demo-element-wrapper">
              <div 
                class="demo-box" 
                :style="{
                  width: (pxVal || 0) + 'px',
                  height: Math.min((pxVal || 0), 150) + 'px'
                }"
              >
                <span>{{ pxVal || 0 }}px</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 快捷预设与对照表 -->
      <div class="preset-section card">
        <div class="card-title">常用像素快捷对照表 (基准: {{ rootFontSize }}px)</div>
        <div class="presets-grid">
          <button 
            v-for="px in presetPx" 
            :key="px" 
            class="preset-card"
            :class="{ active: pxVal === px }"
            @click="selectPreset(px)"
          >
            <div class="p-px">{{ px }}px</div>
            <div class="p-rem">{{ (px / rootFontSize).toFixed(4) }} rem</div>
            <div class="p-vw">{{ ((px / viewportWidth) * 100).toFixed(2) }}vw</div>
          </button>
        </div>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped lang="scss">
.tool-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 1200px;
}
.card {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.card-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-primary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--jc-border-strong);
  padding-bottom: 6px;
}
.grid-inputs {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}
.grid-2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  @media (max-width: 900px) {
    grid-template-columns: 1fr;
  }
}
.input-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}
.input-with-unit {
  display: flex;
  align-items: center;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  overflow: hidden;
  &:focus-within {
    border-color: var(--jc-color-accent);
  }
  input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--jc-text-primary);
    padding: 6px 10px;
    font-size: 12px;
    width: 60px;
    outline: none;
    -moz-appearance: textfield;
    &::-webkit-outer-spin-button,
    &::-webkit-inner-spin-button {
      -webkit-appearance: none;
      margin: 0;
    }
  }
  .unit {
    background: var(--jc-bg-hover);
    padding: 6px 10px;
    font-size: 11px;
    color: var(--jc-text-secondary);
    border-left: 1px solid var(--jc-border-strong);
  }
}
.field-desc {
  font-size: 10px;
  color: var(--jc-text-secondary);
  opacity: 0.8;
}

// 主联动转换
.row-inputs {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.val-input-group {
  display: flex;
  align-items: center;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  overflow: hidden;
  &:focus-within {
    border-color: var(--jc-color-accent);
  }
  .val-label {
    width: 60px;
    text-align: center;
    font-weight: bold;
    font-size: 12px;
    padding: 8px 0;
    color: var(--jc-color-white);
    flex-shrink: 0;
  }
  .px-label { background: #3b82f6; }
  .rem-label { background: #10b981; }
  .em-label { background: #f59e0b; }
  .vw-label { background: #8b5cf6; }
  .vh-label { background: #ec4899; }

  input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--jc-text-primary);
    padding: 8px 12px;
    font-size: 13px;
    font-family: 'Cascadia Code', Consolas, monospace;
    font-weight: 500;
    outline: none;
    &.input-highlight {
      color: var(--jc-text-highlight);
    }
  }
  .val-unit-tag {
    font-size: 10px;
    color: var(--jc-text-secondary);
    padding-right: 12px;
    white-space: nowrap;
  }
}

// 可视化演示
.visualizer-panel {
  justify-content: space-between;
}
.slider-control {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.slider-header {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--jc-text-secondary);
  strong {
    color: var(--jc-text-highlight);
  }
}
.premium-slider {
  width: 100%;
  height: 6px;
  border-radius: 3px;
  outline: none;
  background: var(--jc-border-strong);
  -webkit-appearance: none;
  &::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--jc-color-accent);
    cursor: pointer;
    transition: transform 0.1s;
    &:hover {
      transform: scale(1.2);
    }
  }
}
.demo-box-container {
  flex: 1;
  background: var(--jc-bg-app);
  border: 1px dashed var(--jc-border-default);
  border-radius: 4px;
  min-height: 180px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  position: relative;
}
.demo-element-wrapper {
  max-width: 90%;
  max-height: 90%;
  overflow: auto;
  padding: 10px;
}
.demo-box {
  background: linear-gradient(135deg, var(--jc-color-accent), #4f46e5);
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 12px;
  font-weight: bold;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  transition: width 0.15s ease-out, height 0.15s ease-out;
  min-width: 20px;
  min-height: 20px;
  max-width: 100%;
  white-space: nowrap;
}

// 预设对照表
.presets-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(90px, 1fr));
  gap: 10px;
}
.preset-card {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  padding: 8px;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  transition: all 0.2s;
  &:hover {
    border-color: var(--jc-color-accent);
    background: var(--jc-bg-hover);
  }
  &.active {
    border-color: var(--jc-color-accent);
    background: rgba(59, 130, 246, 0.1);
    .p-px {
      color: var(--jc-color-accent);
    }
  }
  .p-px {
    font-size: 12px;
    font-weight: bold;
    color: var(--jc-text-primary);
  }
  .p-rem {
    font-size: 10px;
    color: var(--jc-text-secondary);
  }
  .p-vw {
    font-size: 9px;
    color: var(--jc-text-secondary);
    opacity: 0.8;
  }
}
</style>
