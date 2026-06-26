<script setup lang="ts">
import { ref, watch, computed } from 'vue'

const inputSvg = ref(`<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100" width="100" height="100">
  <!-- This is a sample comment to be removed -->
  <defs>
    <!-- Empty defs -->
  </defs>
  <g id="Background">
    <rect width="100" height="100" fill="#2563eb" rx="15" />
  </g>
  <g id="LogoGraphics">
    <!-- Circle with high float precision -->
    <circle cx="50.00123" cy="50.00456" r="25.12345" fill="#ffffff" />
    <path d="M 40.000 45.000 L 60.000 45.000 L 50.000 65.000 Z" fill="#1e3a8a" />
  </g>
</svg>`)

const outputSvg = ref('')
const errorMsg = ref('')

// 优化选项配置
const options = ref({
  removeComments: true,
  removeMetadata: true,
  removeNamespaces: true,
  removeEmptyElems: true,
  precisionLimit: true,
  precision: 2
})

// 缩放控制 (10% - 400%)
const previewScale = ref(1.5)

// 优化后/优化前切换预览
const previewMode = ref<'after' | 'before'>('after')

// 计算文件大小
function byteLength(str: string): number {
  return new Blob([str]).size
}

const inputSize = computed(() => byteLength(inputSvg.value))
const outputSize = computed(() => byteLength(outputSvg.value))

const compressionRatio = computed(() => {
  if (inputSize.value === 0) return 0
  const ratio = ((inputSize.value - outputSize.value) / inputSize.value) * 100
  return Number(ratio.toFixed(1))
})

// 优化核心逻辑
function runOptimize() {
  errorMsg.value = ''
  if (!inputSvg.value.trim()) {
    outputSvg.value = ''
    return
  }

  try {
    const parser = new DOMParser()
    const doc = parser.parseFromString(inputSvg.value, 'image/svg+xml')
    const parserError = doc.querySelector('parsererror')
    if (parserError) {
      throw new Error(parserError.textContent || 'SVG 语法格式不正确')
    }

    const svgElem = doc.documentElement
    if (svgElem.tagName.toLowerCase() !== 'svg') {
      throw new Error('根节点必须是 <svg> 元素')
    }

    // 递归清理节点和属性
    function cleanNode(node: Node) {
      if (node.nodeType === Node.COMMENT_NODE && options.value.removeComments) {
        node.parentNode?.removeChild(node)
        return
      }

      if (node.nodeType === Node.ELEMENT_NODE) {
        const elem = node as Element
        const localName = elem.localName.toLowerCase()

        // 移除 metadata 或空容器
        if (options.value.removeMetadata && (localName === 'metadata' || (localName === 'defs' && elem.children.length === 0))) {
          elem.parentNode?.removeChild(elem)
          return
        }

        // 处理属性
        const attrs = Array.from(elem.attributes)
        for (const attr of attrs) {
          const name = attr.name.toLowerCase()

          // 移除不需要的命名空间和编辑软件附加属性
          if (options.value.removeNamespaces && (
            name.startsWith('xmlns:') || 
            name.startsWith('sodipodi:') || 
            name.startsWith('inkscape:') || 
            name === 'xml:space'
          )) {
            elem.removeAttribute(attr.name)
            continue
          }

          // 数字精度限制
          if (options.value.precisionLimit) {
            const limit = options.value.precision
            
            // 如果属性值完全是数字
            const numRegex = /^[+-]?\d*(?:\.\d+)?$/
            if (numRegex.test(attr.value)) {
              const val = parseFloat(attr.value)
              if (!isNaN(val)) {
                elem.setAttribute(attr.name, String(Number(val.toFixed(limit))))
              }
            } else {
              // 对 path d 属性或 points 等批量数字属性进行精度限制替换
              const cleanedVal = attr.value.replace(/(-?\d+\.\d+)/g, (match) => {
                const val = parseFloat(match)
                return String(Number(val.toFixed(limit)))
              })
              elem.setAttribute(attr.name, cleanedVal)
            }
          }
        }

        // 递归处理子节点
        const children = Array.from(elem.childNodes)
        for (const child of children) {
          cleanNode(child)
        }

        // 空元素处理
        if (options.value.removeEmptyElems) {
          const isContainer = ['g', 'defs'].includes(localName)
          if (isContainer && elem.children.length === 0) {
            elem.parentNode?.removeChild(elem)
            return
          }
          if (localName === 'path' && !elem.getAttribute('d')) {
            elem.parentNode?.removeChild(elem)
            return
          }
        }
      }
    }

    // 清理顶层节点下的内容
    const rootChildren = Array.from(svgElem.childNodes)
    for (const child of rootChildren) {
      cleanNode(child)
    }

    // 对 svg 根节点本身进行属性清理
    cleanNode(svgElem)

    const serializer = new XMLSerializer()
    let result = serializer.serializeToString(doc)

    // 清理 XML 头和 DOCTYPE
    if (options.value.removeMetadata) {
      result = result.replace(/<\?xml.*?\?>/gi, '')
      result = result.replace(/<!DOCTYPE.*?>/gi, '')
    }

    // 压缩多余空格与空白 (小心保留文本中的合法空白)
    result = result.replace(/\s+/g, ' ').replace(/>\s+</g, '><').trim()

    outputSvg.value = result
  } catch (err: any) {
    errorMsg.value = err.message || '优化失败'
    outputSvg.value = ''
  }
}

watch([inputSvg, options], () => {
  runOptimize()
}, { immediate: true, deep: true })

function copyOutput() {
  if (!outputSvg.value) return
  navigator.clipboard.writeText(outputSvg.value)
}

function downloadSvg() {
  if (!outputSvg.value) return
  const blob = new Blob([outputSvg.value], { type: 'image/svg+xml' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'optimized.svg'
  a.click()
  URL.revokeObjectURL(url)
}

function clearAll() {
  inputSvg.value = ''
  outputSvg.value = ''
}

const fileInput = ref<HTMLInputElement | null>(null)

function handleFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    const file = target.files[0]
    const reader = new FileReader()
    reader.onload = (evt: any) => {
      inputSvg.value = evt.target.result || ''
    }
    reader.onerror = () => {
      errorMsg.value = '无法读取本地 SVG 文件'
    }
    reader.readAsText(file)
  }
}


// 格式化输出字节单位
function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  return (bytes / 1024).toFixed(2) + ' KB'
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">SVG 预览与优化工具</div>
      <div class="tool-desc-header">粘贴 SVG 代码，实时清理冗余元素和属性并预览</div>
    </div>

    <div class="tool-body">
      <!-- 选项配置与输入区 -->
      <div class="left-pane card">
        <div class="card-title">优化选项设置 (Optimize Settings)</div>
        <div class="options-grid">
          <label class="opt-label">
            <input type="checkbox" v-model="options.removeComments" />
            <span>移除注释 (&lt;!-- --&gt;)</span>
          </label>
          <label class="opt-label">
            <input type="checkbox" v-model="options.removeMetadata" />
            <span>移除 XML 声明 &amp; DOCTYPE</span>
          </label>
          <label class="opt-label">
            <input type="checkbox" v-model="options.removeNamespaces" />
            <span>移除命名空间 &amp; 编辑器专有属性</span>
          </label>
          <label class="opt-label">
            <input type="checkbox" v-model="options.removeEmptyElems" />
            <span>移除空元素/空容器</span>
          </label>
          <label class="opt-label">
            <input type="checkbox" v-model="options.precisionLimit" />
            <span>数字精度限制</span>
          </label>
          <div v-if="options.precisionLimit" class="precision-slider-wrap">
            <span class="p-text">保留小数精度: {{ options.precision }} 位</span>
            <input type="range" v-model.number="options.precision" min="0" max="6" />
          </div>
        </div>

        <div class="editor-wrap">
          <div class="editor-header">
            <span>输入原始 SVG 源码:</span>
            <div class="editor-acts-left">
              <button class="btn-clear" style="margin-right: 8px;" @click="fileInput?.click()">打开本地文件</button>
              <button class="btn-clear" @click="clearAll">清空</button>
            </div>
          </div>
          <input 
            type="file" 
            ref="fileInput" 
            accept=".svg" 
            style="display: none;" 
            @change="handleFileSelect" 
          />
          <textarea v-model="inputSvg" placeholder="在这里粘贴 &lt;svg&gt;...&lt;/svg&gt; 代码..." spellcheck="false" class="code-editor"></textarea>
        </div>
      </div>

      <!-- 预览与导出结果 -->
      <div class="right-pane">
        <!-- 统计面板 -->
        <div class="stats-panel card" v-if="outputSvg">
          <div class="stat-item">
            <span class="label">原始体积:</span>
            <span class="value">{{ formatSize(inputSize) }}</span>
          </div>
          <div class="stat-arrow">➔</div>
          <div class="stat-item">
            <span class="label">优化后体积:</span>
            <span class="value val-success">{{ formatSize(outputSize) }}</span>
          </div>
          <div class="stat-badge" :class="{ success: compressionRatio > 0 }">
            {{ compressionRatio > 0 ? `节省了 ${compressionRatio}%` : '未发生压缩' }}
          </div>
        </div>

        <!-- 实时预览面板 -->
        <div class="preview-panel card">
          <div class="preview-header">
            <div class="preview-tabs">
              <button :class="{ active: previewMode === 'after' }" @click="previewMode = 'after'">
                优化后预览
              </button>
              <button :class="{ active: previewMode === 'before' }" @click="previewMode = 'before'">
                优化前预览
              </button>
            </div>
            <div class="scale-control">
              <span class="scale-text">缩放: {{ Math.round(previewScale * 100) }}%</span>
              <button @click="previewScale = Math.max(0.5, previewScale - 0.25)">-</button>
              <button @click="previewScale = Math.min(4, previewScale + 0.25)">+</button>
            </div>
          </div>

          <div class="checkerboard-bg">
            <div class="preview-render-area" :style="{ transform: `scale(${previewScale})` }">
              <div v-if="previewMode === 'after' && outputSvg" v-html="outputSvg"></div>
              <div v-else-if="previewMode === 'before' && inputSvg" v-html="inputSvg"></div>
              <div v-else class="preview-empty">等待输入有效的 SVG</div>
            </div>
          </div>
        </div>

        <!-- 优化后源码输出 -->
        <div class="output-panel card" v-if="outputSvg">
          <div class="output-header">
            <span>优化后的 SVG 源码:</span>
            <div class="output-acts">
              <button class="btn-act pri" @click="copyOutput">复制源码</button>
              <button class="btn-act" @click="downloadSvg">下载 .svg</button>
            </div>
          </div>
          <textarea readonly class="code-editor code-output" :value="outputSvg" spellcheck="false"></textarea>
        </div>

        <div v-if="errorMsg" class="error-panel card">
          <span class="err-title">解析出错:</span>
          <span class="err-desc">{{ errorMsg }}</span>
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
  padding: 16px;
  background: var(--jc-bg-app);
  overflow-y: auto;
  gap: 16px;
}
.tool-header {
  flex-shrink: 0;
  border-left: 3px solid var(--jc-color-accent);
  padding-left: 10px;
}
.tool-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}
.tool-desc-header {
  font-size: 12px;
  color: var(--jc-text-secondary);
  margin-top: 2px;
}
.tool-body {
  display: grid;
  grid-template-columns: 1fr 1.2fr;
  gap: 16px;
  align-items: start;
  max-width: 1250px;
  @media (max-width: 950px) {
    grid-template-columns: 1fr;
  }
}
.card {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
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

// 左栏选项
.options-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px 16px;
  background: var(--jc-bg-input);
  padding: 12px;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong);
}
.opt-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  span {
    font-size: 11px;
    color: var(--jc-text-primary);
  }
  input {
    cursor: pointer;
  }
}
.precision-slider-wrap {
  grid-column: span 2;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border-top: 1px dashed var(--jc-border-default);
  padding-top: 8px;
  margin-top: 4px;
  .p-text {
    font-size: 10px;
    color: var(--jc-text-secondary);
  }
  input[type="range"] {
    flex: 1;
    max-width: 150px;
  }
}

// 编辑器区域
.editor-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
}
.btn-clear {
  background: none;
  border: none;
  color: var(--jc-color-error);
  font-size: 11px;
  cursor: pointer;
  &:hover {
    text-decoration: underline;
  }
}
.code-editor {
  width: 100%;
  height: 380px;
  resize: vertical;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  color: var(--jc-text-primary);
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  padding: 10px;
  line-height: 1.5;
  outline: none;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}

// 右栏布局
.right-pane {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

// 统计面板
.stats-panel {
  flex-direction: row;
  align-items: center;
  gap: 20px;
  padding: 12px 16px;
}
.stat-item {
  display: flex;
  flex-direction: column;
  .label {
    font-size: 10px;
    color: var(--jc-text-secondary);
  }
  .value {
    font-size: 13px;
    font-weight: bold;
    font-family: 'Cascadia Code', Consolas, monospace;
    color: var(--jc-text-primary);
  }
  .val-success {
    color: var(--jc-color-success);
  }
}
.stat-arrow {
  color: var(--jc-text-secondary);
  font-size: 16px;
}
.stat-badge {
  margin-left: auto;
  font-size: 11px;
  font-weight: 600;
  padding: 4px 10px;
  background: var(--jc-bg-hover);
  border-radius: 20px;
  color: var(--jc-text-secondary);
  &.success {
    background: rgba(16, 185, 129, 0.15);
    color: var(--jc-color-success);
  }
}

// 预览区
.preview-panel {
  gap: 0;
  padding: 0;
  overflow: hidden;
}
.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: var(--jc-bg-elevated);
  border-bottom: 1px solid var(--jc-border-default);
}
.preview-tabs {
  display: flex;
  gap: 4px;
  button {
    background: none;
    border: none;
    padding: 4px 10px;
    font-size: 11px;
    color: var(--jc-text-secondary);
    cursor: pointer;
    border-radius: 3px;
    &:hover {
      background: var(--jc-bg-hover);
      color: var(--jc-text-primary);
    }
    &.active {
      background: var(--jc-bg-panel);
      color: var(--jc-color-accent);
      font-weight: 600;
    }
  }
}
.scale-control {
  display: flex;
  align-items: center;
  gap: 6px;
  .scale-text {
    font-size: 10px;
    color: var(--jc-text-secondary);
    margin-right: 4px;
  }
  button {
    width: 20px;
    height: 20px;
    border: 1px solid var(--jc-border-strong);
    background: var(--jc-bg-panel);
    color: var(--jc-text-primary);
    border-radius: 2px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    &:hover {
      background: var(--jc-bg-hover);
    }
  }
}

// 棋盘格背景 (Checkerboard background)
.checkerboard-bg {
  min-height: 220px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--jc-bg-app);
  background-image: 
    linear-gradient(45deg, var(--jc-bg-hover) 25%, transparent 25%), 
    linear-gradient(-45deg, var(--jc-bg-hover) 25%, transparent 25%), 
    linear-gradient(45deg, transparent 75%, var(--jc-bg-hover) 75%), 
    linear-gradient(-45deg, transparent 75%, var(--jc-bg-hover) 75%);
  background-size: 16px 16px;
  background-position: 0 0, 0 8px, 8px -8px, -8px 0px;
  overflow: auto;
  position: relative;
  padding: 20px;
}
.preview-render-area {
  display: inline-block;
  transform-origin: center center;
  transition: transform 0.15s ease-out;
  :deep(svg) {
    display: block;
    max-width: 100%;
    max-height: 300px;
    height: auto;
  }
}
.preview-empty {
  font-size: 11px;
  color: var(--jc-text-secondary);
}

// 输出源码区
.output-panel {
  gap: 8px;
}
.output-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
}
.output-acts {
  display: flex;
  gap: 6px;
}
.btn-act {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 4px 10px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 2px;
  &:hover {
    background: var(--jc-bg-btn-hover);
  }
  &.pri {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    &:hover {
      background: var(--jc-color-accent-hover);
    }
  }
}
.code-output {
  height: 180px;
  color: var(--jc-color-success);
}

// 错误提示
.error-panel {
  background: rgba(239, 68, 68, 0.1);
  border-color: rgba(239, 68, 68, 0.2);
  color: var(--jc-color-error);
  font-size: 11px;
  padding: 10px 14px;
  flex-direction: row;
  gap: 6px;
  .err-title {
    font-weight: bold;
  }
}
</style>
