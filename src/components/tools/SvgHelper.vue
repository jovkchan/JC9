<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcInputNumber from '@/components/ui/JcInputNumber.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import { convertSvgToAndroidVector } from './composables/useSvgToAndroidVector'
import { convertSvgToSfSymbols, type SfSymbolsResult } from './composables/useSvgToSfSymbols'
import { convertToSvg } from './composables/useVectorXmlToSvg'

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

// 工具模式：SVG 优化 / SVG → Android Vector / SVG → SF Symbols / 反向转 SVG
const toolMode = ref<'optimize' | 'android' | 'sf' | 'reverse'>('optimize')

// SF Symbols 转换选项与结果
const sfOptions = ref({ symbolName: 'MySymbol' })
const sfResult = ref<SfSymbolsResult | null>(null)
const sfError = ref('')

// 反向转换（Android XML / SF SVG → SVG）
const reverseOutput = ref('')
const reverseWarnings = ref<string[]>([])
const reverseError = ref('')
const reverseKind = ref<'vector' | 'svg' | 'unknown'>('unknown')

// PNG 导出选项
const pngOptions = ref({
  width: 512 as number | null,
  height: 512 as number | null,
  bg: 'transparent',
  bgColor: '#ffffff',
})

const pngBgOptions = [
  { label: '透明', value: 'transparent' },
  { label: '白色', value: '#ffffff' },
  { label: '黑色', value: '#000000' },
  { label: '自定义', value: 'custom' },
]

// Android Vector 转换选项
const vecOptions = ref({
  widthDp: 24 as number | null,
  heightDp: 24 as number | null,
  precision: 2,
})
const vecOutput = ref('')
const vecWarnings = ref<string[]>([])
const vecError = ref('')

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

// —— SVG → Android Vector ——
function runVectorConvert() {
  vecError.value = ''
  vecWarnings.value = []
  const res = convertSvgToAndroidVector(inputSvg.value, {
    widthDp: vecOptions.value.widthDp ?? 24,
    heightDp: vecOptions.value.heightDp ?? 24,
    precision: vecOptions.value.precision,
  })
  vecOutput.value = res.xml
  vecWarnings.value = res.warnings
  if (res.error) vecError.value = res.error
}

function copyVecOutput() {
  if (!vecOutput.value) return
  navigator.clipboard.writeText(vecOutput.value)
}

function downloadVectorXml() {
  if (!vecOutput.value) return
  const blob = new Blob([vecOutput.value], { type: 'text/xml' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'vector_drawable.xml'
  a.click()
  URL.revokeObjectURL(url)
}

// —— SVG → SF Symbols ——
function runSfConvert() {
  sfError.value = ''
  const res = convertSvgToSfSymbols(inputSvg.value, sfOptions.value.symbolName)
  sfResult.value = res.error ? null : res
  if (res.error) sfError.value = res.error
}

function copySfSvg() {
  if (!sfResult.value) return
  navigator.clipboard.writeText(sfResult.value.svg)
}

async function downloadSymbolset() {
  const res = sfResult.value
  if (!res || !res.svg) return
  const JSZip = (await import('jszip')).default
  const zip = new JSZip()
  const folder = zip.folder(`${res.symbolName}.symbolset`)
  if (folder) {
    folder.file(res.svgFileName, res.svg)
    folder.file('Contents.json', res.contentsJson)
  }
  const blob = await zip.generateAsync({ type: 'blob' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${res.symbolName}.symbolset.zip`
  a.click()
  URL.revokeObjectURL(url)
}

// —— 反向转换（Android XML / SF SVG → SVG） ——
function runReverseConvert() {
  reverseError.value = ''
  reverseWarnings.value = []
  const res = convertToSvg(inputSvg.value)
  reverseOutput.value = res.svg
  reverseWarnings.value = res.warnings
  reverseKind.value = res.kind
  if (res.error) reverseError.value = res.error
}

function copyReverse() {
  if (!reverseOutput.value) return
  navigator.clipboard.writeText(reverseOutput.value)
}

function downloadReverseSvg() {
  if (!reverseOutput.value) return
  const blob = new Blob([reverseOutput.value], { type: 'image/svg+xml' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'converted.svg'
  a.click()
  URL.revokeObjectURL(url)
}

// —— 统一转换分派：按当前模式实时转换 ——
watch([toolMode, inputSvg, options, vecOptions, sfOptions], () => {
  if (toolMode.value === 'optimize') runOptimize()
  else if (toolMode.value === 'android') runVectorConvert()
  else if (toolMode.value === 'sf') runSfConvert()
  else if (toolMode.value === 'reverse') runReverseConvert()
}, { immediate: true, deep: true })

// —— PNG 导出（自定义尺寸） ——
const pngSourceSvg = computed(() => {
  if (toolMode.value === 'optimize') return outputSvg.value || inputSvg.value
  if (toolMode.value === 'reverse') return reverseOutput.value || inputSvg.value
  return inputSvg.value
})

const pngSourceLabel = computed(() => {
  if (toolMode.value === 'optimize') return '优化后 SVG'
  if (toolMode.value === 'reverse') return reverseKind.value === 'vector' ? '转换出的 SVG' : '输入 SVG'
  return '输入 SVG'
})

function doDownloadPng() {
  const src = pngSourceSvg.value
  if (!src) {
    errorMsg.value = '没有可导出的 SVG 源码'
    return
  }
  const w = Math.max(1, Math.min(4096, Math.round(pngOptions.value.width ?? 512)))
  const h = Math.max(1, Math.min(4096, Math.round(pngOptions.value.height ?? 512)))
  const bg = pngOptions.value.bg === 'custom' ? pngOptions.value.bgColor : pngOptions.value.bg

  const svgBlob = new Blob([src], { type: 'image/svg+xml;charset=utf-8' })
  const url = URL.createObjectURL(svgBlob)
  const img = new Image()
  img.onload = () => {
    const canvas = document.createElement('canvas')
    canvas.width = w
    canvas.height = h
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    if (bg && bg !== 'transparent') {
      ctx.fillStyle = bg
      ctx.fillRect(0, 0, w, h)
    }
    ctx.drawImage(img, 0, 0, w, h)
    canvas.toBlob((blob) => {
      URL.revokeObjectURL(url)
      if (!blob) return
      const pngUrl = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = pngUrl
      a.download = `icon_${w}x${h}.png`
      a.click()
      URL.revokeObjectURL(pngUrl)
    }, 'image/png')
  }
  img.onerror = () => {
    URL.revokeObjectURL(url)
    errorMsg.value = 'PNG 导出失败：SVG 无法渲染（可能包含不支持的语法）'
  }
  img.src = url
}

const rightLabel = computed(() => {
  switch (toolMode.value) {
    case 'android': return '预览与导出（Vector）'
    case 'sf': return '预览与导出（SF Symbols）'
    case 'reverse': return '预览与导出（SVG）'
    default: return '预览与导出'
  }
})

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
  <ToolShell title="SVG 预览与优化工具" subtitle="粘贴 SVG 代码，实时清理冗余元素和属性并预览" split>
    <template #left-label>优化选项与输入</template>
    <template #left>
      <div class="mode-switch">
        <JcButton size="small" :type="toolMode === 'optimize' ? 'primary' : 'default'" @click="toolMode = 'optimize'">SVG 优化</JcButton>
        <JcButton size="small" :type="toolMode === 'android' ? 'primary' : 'default'" @click="toolMode = 'android'">Android Vector</JcButton>
        <JcButton size="small" :type="toolMode === 'sf' ? 'primary' : 'default'" @click="toolMode = 'sf'">SF Symbols</JcButton>
        <JcButton size="small" :type="toolMode === 'reverse' ? 'primary' : 'default'" @click="toolMode = 'reverse'">反向转 SVG</JcButton>
      </div>

      <div v-if="toolMode === 'optimize'" class="options-grid">
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

      <div v-else-if="toolMode === 'android'" class="vec-options">
        <div class="vec-size-row">
          <label class="vec-field">
            <span>输出宽 (dp)</span>
            <JcInputNumber v-model="vecOptions.widthDp" :min="1" :max="512" :step="1" size="small" suffix="dp" />
          </label>
          <label class="vec-field">
            <span>输出高 (dp)</span>
            <JcInputNumber v-model="vecOptions.heightDp" :min="1" :max="512" :step="1" size="small" suffix="dp" />
          </label>
        </div>
        <div class="precision-slider-wrap">
          <span class="p-text">pathData 小数精度: {{ vecOptions.precision }} 位</span>
          <input type="range" v-model.number="vecOptions.precision" min="0" max="4" />
        </div>
        <div class="vec-hint">
          将 <code>&lt;svg&gt;</code> 图标转换为 Android <code>&lt;vector&gt;</code> 可绘制 XML（可直接放入 <code>res/drawable</code>）。支持 path / rect / circle / ellipse / line / polyline / polygon 及分组 transform。
        </div>
      </div>

      <div v-else-if="toolMode === 'sf'" class="vec-options">
        <label class="vec-field">
          <span>Symbol 名称（symbolset 目录名）</span>
          <JcInput v-model="sfOptions.symbolName" size="small" placeholder="如 MySymbol / gear" />
        </label>
        <div class="vec-hint">
          转换为 Apple SF Symbols 模板：归一化到 <code>24×24</code> 网格、统一黑色填充（不含描边）。输出 <code>.symbolset</code> 结构（<code>Contents.json</code> + SVG），可打包下载后放入 <code>Assets.xcassets</code>。
        </div>
      </div>

      <div v-else class="vec-options">
        <div class="vec-hint">
          输入 Android <code>&lt;vector&gt;</code> XML 或 SF Symbols 的 <code>&lt;svg&gt;</code>，自动检测并转换为标准 <code>&lt;svg&gt;</code>。支持 path / group 及 fill / stroke 系列属性。
        </div>
      </div>

      <div class="editor-wrap">
        <div class="editor-header">
          <span>输入{{ toolMode === 'reverse' ? ' SVG / Android XML' : '原始 SVG 源码' }}:</span>
          <div class="editor-acts-left">
            <JcButton size="small" @click="fileInput?.click()">打开本地文件</JcButton>
            <JcButton danger size="small" @click="clearAll">清空</JcButton>
          </div>
        </div>
        <input type="file" ref="fileInput" accept=".svg,.xml" style="display: none" @change="handleFileSelect" />
        <JcTextarea v-model="inputSvg" mono beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="在这里粘贴 &lt;svg&gt; / &lt;vector&gt; 代码..." />
      </div>
    </template>

    <template #right-label>{{ rightLabel }}</template>
    <template #right>
      <div class="stats-panel card" v-if="toolMode === 'optimize' && outputSvg">
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

      <div class="preview-panel card">
        <div class="preview-header">
          <div class="preview-tabs" v-if="toolMode === 'optimize'">
            <JcButton size="small" :type="previewMode === 'after' ? 'primary' : 'default'" @click="previewMode = 'after'">优化后预览</JcButton>
            <JcButton size="small" :type="previewMode === 'before' ? 'primary' : 'default'" @click="previewMode = 'before'">优化前预览</JcButton>
          </div>
          <div class="scale-control">
            <span class="scale-text">缩放: {{ Math.round(previewScale * 100) }}%</span>
            <JcButton size="small" @click="previewScale = Math.max(0.5, previewScale - 0.25)">-</JcButton>
            <JcButton size="small" @click="previewScale = Math.min(4, previewScale + 0.25)">+</JcButton>
          </div>
        </div>

        <div class="checkerboard-bg">
          <div class="preview-render-area" :style="{ transform: `scale(${previewScale})` }">
            <div v-if="toolMode === 'optimize' && previewMode === 'after' && outputSvg" v-html="outputSvg"></div>
            <div v-else-if="toolMode === 'optimize' && previewMode === 'before' && inputSvg" v-html="inputSvg"></div>
            <div v-else-if="toolMode === 'android' && inputSvg" v-html="inputSvg"></div>
            <div v-else-if="toolMode === 'sf' && inputSvg" v-html="inputSvg"></div>
            <div v-else-if="toolMode === 'reverse' && reverseOutput" v-html="reverseOutput"></div>
            <div v-else class="preview-empty">等待输入有效的 SVG / XML</div>
          </div>
        </div>
      </div>

      <div class="output-panel card" v-if="toolMode === 'optimize' && outputSvg">
        <div class="output-header">
          <span>优化后的 SVG 源码:</span>
          <div class="output-acts">
            <JcButton type="primary" size="small" @click="copyOutput">复制源码</JcButton>
            <JcButton size="small" @click="downloadSvg">下载 .svg</JcButton>
          </div>
        </div>
        <JcTextarea mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill code-output" :model-value="outputSvg" />
      </div>

      <div class="output-panel card" v-if="toolMode === 'android' && vecOutput">
        <div class="output-header">
          <span>生成的 Android Vector Drawable XML:</span>
          <div class="output-acts">
            <JcButton type="primary" size="small" @click="copyVecOutput">复制 XML</JcButton>
            <JcButton size="small" @click="downloadVectorXml">下载 .xml</JcButton>
          </div>
        </div>
        <JcTextarea mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill code-output" :model-value="vecOutput" />
      </div>

      <div v-if="toolMode === 'sf' && sfResult" class="output-panel card">
        <div class="output-header">
          <span>{{ sfResult.symbolName }}.symbolset 结构:</span>
          <div class="output-acts">
            <JcButton type="primary" size="small" @click="downloadSymbolset">下载 .symbolset.zip</JcButton>
            <JcButton size="small" @click="copySfSvg">复制 SVG</JcButton>
          </div>
        </div>
        <div class="sf-file">
          <div class="sf-file-label">Contents.json</div>
          <JcTextarea mono readonly :spellcheck="false" class="jc-fill sf-code" :model-value="sfResult.contentsJson" />
        </div>
        <div class="sf-file">
          <div class="sf-file-label">{{ sfResult.svgFileName }}（24×24 黑色模板）</div>
          <JcTextarea mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill sf-code" :model-value="sfResult.svg" />
        </div>
      </div>

      <div class="output-panel card" v-if="toolMode === 'reverse' && reverseOutput">
        <div class="output-header">
          <span>转换出的标准 SVG:</span>
          <div class="output-acts">
            <JcButton type="primary" size="small" @click="copyReverse">复制 SVG</JcButton>
            <JcButton size="small" @click="downloadReverseSvg">下载 .svg</JcButton>
          </div>
        </div>
        <JcTextarea mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill code-output" :model-value="reverseOutput" />
      </div>

      <!-- PNG 导出：所有模式通用 -->
      <div class="output-panel card png-panel">
        <div class="output-header">
          <span>PNG 导出（自定义尺寸）</span>
        </div>
        <div class="vec-size-row png-size-row">
          <label class="vec-field">
            <span>宽度 (px)</span>
            <JcInputNumber v-model="pngOptions.width" :min="1" :max="4096" :step="1" size="small" suffix="px" />
          </label>
          <label class="vec-field">
            <span>高度 (px)</span>
            <JcInputNumber v-model="pngOptions.height" :min="1" :max="4096" :step="1" size="small" suffix="px" />
          </label>
        </div>
        <div class="vec-size-row png-size-row">
          <label class="vec-field">
            <span>背景</span>
            <JcSelect v-model="pngOptions.bg" :options="pngBgOptions" size="small" style="width: 100%" />
          </label>
          <label v-if="pngOptions.bg === 'custom'" class="vec-field">
            <span>自定义背景色</span>
            <div class="png-color-wrap"><input type="color" v-model="pngOptions.bgColor" /></div>
          </label>
        </div>
        <div class="png-acts">
          <JcButton type="primary" size="small" :disabled="!pngSourceSvg" @click="doDownloadPng">下载 PNG</JcButton>
          <span class="png-note">来源：{{ pngSourceLabel }}{{ pngSourceSvg ? '' : '（无可导出 SVG）' }}</span>
        </div>
      </div>

      <div v-if="toolMode === 'android' && vecWarnings.length" class="warn-panel card">
        <span class="warn-title">转换提示</span>
        <ul class="warn-list">
          <li v-for="(w, i) in vecWarnings" :key="i">{{ w }}</li>
        </ul>
      </div>

      <div v-if="toolMode === 'reverse' && reverseWarnings.length" class="warn-panel card">
        <span class="warn-title">转换提示</span>
        <ul class="warn-list">
          <li v-for="(w, i) in reverseWarnings" :key="i">{{ w }}</li>
        </ul>
      </div>

      <div v-if="toolMode === 'android' && vecError" class="error-panel card">
        <span class="err-title">转换出错:</span>
        <span class="err-desc">{{ vecError }}</span>
      </div>

      <div v-if="toolMode === 'sf' && sfError" class="error-panel card">
        <span class="err-title">转换出错:</span>
        <span class="err-desc">{{ sfError }}</span>
      </div>

      <div v-if="toolMode === 'reverse' && reverseError" class="error-panel card">
        <span class="err-title">转换出错:</span>
        <span class="err-desc">{{ reverseError }}</span>
      </div>

      <div v-if="errorMsg" class="error-panel card">
        <span class="err-title">解析出错:</span>
        <span class="err-desc">{{ errorMsg }}</span>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped lang="scss">
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
.mode-switch {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}
.options-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px 16px;
  background: var(--jc-bg-input);
  padding: 12px;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong);
}

// Android Vector 选项
.vec-options {
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: var(--jc-bg-input);
  padding: 12px;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong);
}
.vec-size-row {
  display: flex;
  gap: 12px;
}
.vec-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  span {
    font-size: 10px;
    color: var(--jc-text-secondary);
  }
}
.vec-hint {
  font-size: 10px;
  line-height: 1.6;
  color: var(--jc-text-secondary);
  border-top: 1px dashed var(--jc-border-default);
  padding-top: 8px;
  code {
    color: var(--jc-text-primary);
    background: var(--jc-bg-hover);
    padding: 0 3px;
    border-radius: 3px;
    font-size: 10px;
  }
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
.code-output {
  color: var(--jc-color-success);
}
/* beam 模式下 code-output 落到 wrapper 根，颜色需穿透到内部 textarea */
.code-output :deep(textarea) {
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

// 转换提示（警告）
.warn-panel {
  background: rgba(245, 158, 11, 0.08);
  border-color: rgba(245, 158, 11, 0.25);
  color: var(--jc-text-primary);
  font-size: 11px;
  padding: 10px 14px;
  gap: 6px;
  .warn-title {
    font-weight: bold;
    color: var(--jc-color-warning, #f5a623);
  }
  .warn-list {
    margin: 0;
    padding-left: 16px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    color: var(--jc-text-secondary);
  }
}

// SF Symbols 输出
.sf-file {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.sf-file-label {
  font-size: 10px;
  color: var(--jc-text-secondary);
  font-family: 'Cascadia Code', Consolas, monospace;
}
.sf-code {
  max-height: 160px;
}

// PNG 导出面板
.png-panel {
  gap: 10px;
}
.png-size-row {
  gap: 10px;
}
.png-color-wrap {
  display: flex;
  align-items: center;
  input[type="color"] {
    width: 100%;
    height: 24px;
    padding: 0;
    border: 1px solid var(--jc-border-strong);
    border-radius: 4px;
    background: transparent;
    cursor: pointer;
  }
}
.png-acts {
  display: flex;
  align-items: center;
  gap: 10px;
  border-top: 1px dashed var(--jc-border-default);
  padding-top: 8px;
}
.png-note {
  font-size: 10px;
  color: var(--jc-text-secondary);
}
</style>
