<script setup lang="ts">
import { ref, reactive, watch, onMounted, onUnmounted, nextTick } from 'vue'
import JSZip from 'jszip'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'

const customFormatOptions = [
  { label: 'PNG 格式 (.png)', value: 'png' },
  { label: 'JPEG 格式 (.jpg)', value: 'jpg' },
  { label: 'Windows 图标 (.ico)', value: 'ico' }
]

// 支持的上传图片格式
const allowedTypes = ['image/png', 'image/jpeg', 'image/jpg', 'image/bmp', 'image/svg+xml']

// 裁剪与画布状态
const fileInput = ref<HTMLInputElement | null>(null)
const previewCanvas = ref<HTMLCanvasElement | null>(null)
const uploadedFile = ref<File | null>(null)
const uploadedImage = ref<HTMLImageElement | null>(null)
const isDragging = ref(false)

// 转换控制选项
const options = reactive({
  scale: 1.0,
  offsetX: 0,
  offsetY: 0,
  bgColor: '#ffffff',
  isTransparent: true,
  maskType: 'square' as 'square' | 'ios' | 'android' | 'pwa',
  // 新增：图像处理与去模糊参数
  sharpen: 0,
  hueRotate: 0,
  saturate: 100,
  brightness: 100,
  contrast: 100,
})

// 原图元信息
const imgMeta = reactive({
  name: '',
  width: 0,
  height: 0,
  size: 0,
  mime: '',
  src: '',
})

// 各平台模板勾选状态
const platforms = reactive({
  web: true,
  tauri: true,
  ios: true,
  android: true,
  custom: false,
})

// 自定义导出设置
const customSettings = reactive({
  width: 256,
  height: 256,
  format: 'png' as 'png' | 'jpg' | 'ico',
  icoSizes: [16, 32, 48, 128, 256],
})

// 打包导出进度与状态
const isGenerating = ref(false)
const genProgress = ref(0)
const genStatusText = ref('')

// Canvas 画布物理参数
const PREVIEW_SIZE = 300 // 预览画布正方形尺寸
const CROP_SIZE = 220    // 裁剪区域正方形尺寸

// 拖拽控制逻辑
let isMouseDown = false
let startX = 0
let startY = 0

function triggerFileInput() {
  fileInput.value?.click()
}

// 载入图片并初始化裁剪状态
function handleFile(file: File) {
  if (!allowedTypes.includes(file.type) && !file.name.toLowerCase().endsWith('.svg')) {
    alert('请选择有效的 PNG, JPG, BMP 或 SVG 文件！')
    return
  }

  uploadedFile.value = file
  imgMeta.name = file.name
  imgMeta.size = file.size
  imgMeta.mime = file.type || 'image/svg+xml'

  const reader = new FileReader()
  reader.onload = (e) => {
    const src = e.target?.result as string
    imgMeta.src = src

    const img = new Image()
    img.onload = async () => {
      imgMeta.width = img.width
      imgMeta.height = img.height
      uploadedImage.value = img

      // 等待 Vue 将 v-else 内的 canvas 挂载渲染到 DOM 上
      await nextTick()
      // 初始居中并适应裁剪区
      resetCropState()
    }
    img.src = src
  }
  reader.readAsDataURL(file)
}

function resetCropState() {
  if (!uploadedImage.value) return
  const img = uploadedImage.value
  
  // 计算初始缩放比，使其刚好能覆盖裁剪区
  const scaleX = CROP_SIZE / img.width
  const scaleY = CROP_SIZE / img.height
  options.scale = Math.max(scaleX, scaleY)
  options.offsetX = 0
  options.offsetY = 0
  
  drawPreview()
}

function resetAdjustments() {
  options.sharpen = 0
  options.hueRotate = 0
  options.saturate = 100
  options.brightness = 100
  options.contrast = 100
}

function onFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files[0]) {
    handleFile(target.files[0])
  }
}

// 拖拽上传
function onDragOver(e: DragEvent) {
  e.preventDefault()
  isDragging.value = true
}

function onDragLeave() {
  isDragging.value = false
}

function onDrop(e: DragEvent) {
  e.preventDefault()
  isDragging.value = false
  if (e.dataTransfer?.files && e.dataTransfer.files[0]) {
    handleFile(e.dataTransfer.files[0])
  }
}

// 鼠标拖拽平移图片
function onCanvasMouseDown(e: MouseEvent) {
  if (!uploadedImage.value) return
  isMouseDown = true
  startX = e.clientX - options.offsetX
  startY = e.clientY - options.offsetY
  
  window.addEventListener('mousemove', onCanvasMouseMove)
  window.addEventListener('mouseup', onCanvasMouseUp)
}

function onCanvasMouseMove(e: MouseEvent) {
  if (!isMouseDown) return
  options.offsetX = e.clientX - startX
  options.offsetY = e.clientY - startY
  drawPreview()
}

function onCanvasMouseUp() {
  isMouseDown = false
  window.removeEventListener('mousemove', onCanvasMouseMove)
  window.removeEventListener('mouseup', onCanvasMouseUp)
}

// 鼠标滚轮缩放图片
function onCanvasWheel(e: WheelEvent) {
  if (!uploadedImage.value) return
  e.preventDefault()
  
  const zoomFactor = 1.05
  let newScale = options.scale
  if (e.deltaY < 0) {
    newScale = Math.min(newScale * zoomFactor, 10.0) // 放大限制 10x
  } else {
    newScale = Math.max(newScale / zoomFactor, 0.05) // 缩小限制 0.05x
  }
  
  options.scale = parseFloat(newScale.toFixed(4))
  drawPreview()
}

// ================= 图像处理与卷积去模糊算子 =================

// 基于 3x3 邻域拉普拉斯卷积核的去模糊锐化函数
function applySharpen(canvas: HTMLCanvasElement, amount: number) {
  if (amount <= 0) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const w = canvas.width
  const h = canvas.height
  const imgData = ctx.getImageData(0, 0, w, h)
  const src = imgData.data

  // 缓冲区暂存目标像素
  const output = ctx.createImageData(w, h)
  const dst = output.data

  // 将 amount 映射到拉普拉斯算子的强度比
  const mix = (amount / 100) * 0.5 // 限制最大处理比以防止边缘过度噪化
  const a = -mix
  const b = 1 + 4 * mix

  // 复制原图像素为背景底，防止边缘 1 像素宽度的黑边
  dst.set(src)

  // 3x3 空间卷积处理
  for (let y = 1; y < h - 1; y++) {
    const rowOffset = y * w * 4
    const prevRowOffset = (y - 1) * w * 4
    const nextRowOffset = (y + 1) * w * 4

    for (let x = 1; x < w - 1; x++) {
      const idx = rowOffset + x * 4

      for (let c = 0; c < 3; c++) { // 遍历处理 R, G, B
        const center = src[idx + c]
        const left = src[idx - 4 + c]
        const right = src[idx + 4 + c]
        const up = src[prevRowOffset + x * 4 + c]
        const down = src[nextRowOffset + x * 4 + c]

        // 卷积计算公式
        let val = center * b + (left + right + up + down) * a

        // 截断到合法颜色范围
        if (val < 0) val = 0
        else if (val > 255) val = 255

        dst[idx + c] = val
      }
      // 保持 Alpha 不变
      dst[idx + 3] = src[idx + 3]
    }
  }

  ctx.putImageData(output, 0, 0)
}

// 绘制主工作区预览 Canvas
function drawPreview() {
  const canvas = previewCanvas.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  // 清空画布并绘制格子背景（标示透明区域）
  ctx.clearRect(0, 0, PREVIEW_SIZE, PREVIEW_SIZE)
  drawGridBackground(ctx, PREVIEW_SIZE, PREVIEW_SIZE)

  // 绘制背景色 (如果不透明)
  if (!options.isTransparent) {
    ctx.fillStyle = options.bgColor
    ctx.fillRect(0, 0, PREVIEW_SIZE, PREVIEW_SIZE)
  }

  // 绘制用户原图，并在此应用色彩优化滤镜
  if (uploadedImage.value) {
    const img = uploadedImage.value
    ctx.save()
    // 限制绘制在主画布内
    ctx.beginPath()
    ctx.rect(0, 0, PREVIEW_SIZE, PREVIEW_SIZE)
    ctx.clip()

    // 1. 设置色彩滤镜
    ctx.filter = `brightness(${options.brightness}%) contrast(${options.contrast}%) saturate(${options.saturate}%) hue-rotate(${options.hueRotate}deg)`

    // 2. 图像中心定位与变换
    const renderW = img.width * options.scale
    const renderH = img.height * options.scale
    const renderX = PREVIEW_SIZE / 2 + options.offsetX - renderW / 2
    const renderY = PREVIEW_SIZE / 2 + options.offsetY - renderH / 2

    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(img, renderX, renderY, renderW, renderH)
    
    // 3. 复原滤镜以防影响后续的裁剪线与遮罩
    ctx.filter = 'none'
    ctx.restore()

    // 4. 应用基于 3x3 空间卷积的去模糊锐化
    if (options.sharpen > 0) {
      applySharpen(canvas, options.sharpen)
    }
  }

  // 绘制半透明黑色裁剪遮罩与裁剪框
  drawCropMask(ctx)
}

// 绘制透明格纹底色
function drawGridBackground(ctx: CanvasRenderingContext2D, w: number, h: number) {
  const gridW = 10
  const cols = Math.ceil(w / gridW)
  const rows = Math.ceil(h / gridW)
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      ctx.fillStyle = (r + c) % 2 === 0 ? '#1e1e24' : '#25252c'
      ctx.fillRect(c * gridW, r * gridW, gridW, gridW)
    }
  }
}

// 绘制裁剪区半透明黑色遮罩和安全区引导线
function drawCropMask(ctx: CanvasRenderingContext2D) {
  ctx.save()
  const offset = (PREVIEW_SIZE - CROP_SIZE) / 2

  // 1. 建立非裁剪区遮罩
  ctx.fillStyle = 'rgba(0, 0, 0, 0.65)'
  ctx.beginPath()
  ctx.rect(0, 0, PREVIEW_SIZE, PREVIEW_SIZE)

  // 根据安全区形态反向剪切
  if (options.maskType === 'square') {
    ctx.rect(offset, offset, CROP_SIZE, CROP_SIZE)
  } else if (options.maskType === 'ios') {
    ctx.roundRect ? ctx.roundRect(offset, offset, CROP_SIZE, CROP_SIZE, 42) : drawRoundRectPolyfill(ctx, offset, offset, CROP_SIZE, CROP_SIZE, 42)
  } else if (options.maskType === 'android') {
    ctx.arc(PREVIEW_SIZE / 2, PREVIEW_SIZE / 2, CROP_SIZE / 2, 0, Math.PI * 2)
  } else if (options.maskType === 'pwa') {
    ctx.rect(offset, offset, CROP_SIZE, CROP_SIZE)
  }
  
  ctx.clip('evenodd')
  ctx.fillRect(0, 0, PREVIEW_SIZE, PREVIEW_SIZE)
  ctx.restore()

  // 2. 绘制裁剪边框线与安全辅助线
  ctx.save()
  ctx.lineWidth = 1.5
  ctx.strokeStyle = 'var(--jc-color-accent, #8a58ff)'
  ctx.shadowColor = 'rgba(0, 0, 0, 0.5)'
  ctx.shadowBlur = 4

  if (options.maskType === 'square') {
    ctx.strokeRect(offset, offset, CROP_SIZE, CROP_SIZE)
  } else if (options.maskType === 'ios') {
    ctx.beginPath()
    ctx.roundRect ? ctx.roundRect(offset, offset, CROP_SIZE, CROP_SIZE, 42) : drawRoundRectPolyfill(ctx, offset, offset, CROP_SIZE, CROP_SIZE, 42)
    ctx.stroke()
  } else if (options.maskType === 'android') {
    ctx.beginPath()
    ctx.arc(PREVIEW_SIZE / 2, PREVIEW_SIZE / 2, CROP_SIZE / 2, 0, Math.PI * 2)
    ctx.stroke()
  } else if (options.maskType === 'pwa') {
    ctx.strokeRect(offset, offset, CROP_SIZE, CROP_SIZE)
    // 绘制 66% 安全线
    ctx.strokeStyle = 'rgba(255, 165, 0, 0.7)' // 橙色安全参考虚线
    ctx.setLineDash([4, 4])
    const pwaOffset = (PREVIEW_SIZE - CROP_SIZE * 0.66) / 2
    ctx.strokeRect(pwaOffset, pwaOffset, CROP_SIZE * 0.66, CROP_SIZE * 0.66)
  }
  ctx.restore()
}

// 针对旧浏览器的圆角矩形 polyfill
function drawRoundRectPolyfill(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  if (w < 2 * r) r = w / 2
  if (h < 2 * r) r = h / 2
  ctx.moveTo(x + r, y)
  ctx.arcTo(x + w, y, x + w, y + h, r)
  ctx.arcTo(x + w, y + h, x, y + h, r)
  ctx.arcTo(x, y + h, x, y, r)
  ctx.arcTo(x, y, x + w, y, r)
  ctx.closePath()
}

// 监听控制项改变时自动重绘画布
watch([options, uploadedImage], () => {
  drawPreview()
})

// ================= 核心图像生成逻辑 =================

// 逐步缩小平滑 Canvas 降采样函数，以防止像素失真
function getResizedCanvas(srcCanvas: HTMLCanvasElement, targetW: number, targetH: number): HTMLCanvasElement {
  let curCanvas = srcCanvas
  let curW = srcCanvas.width
  let curH = srcCanvas.height

  // 每次缩小一半，平滑过滤
  while (curW > targetW * 2 && curH > targetH * 2) {
    const nextCanvas = document.createElement('canvas')
    nextCanvas.width = Math.floor(curW / 2)
    nextCanvas.height = Math.floor(curH / 2)
    const ctx = nextCanvas.getContext('2d')!
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(curCanvas, 0, 0, curW, curH, 0, 0, nextCanvas.width, nextCanvas.height)
    curCanvas = nextCanvas
    curW = nextCanvas.width
    curH = nextCanvas.height
  }

  // 最终绘制到目标尺寸的 Canvas 上
  const finalCanvas = document.createElement('canvas')
  finalCanvas.width = targetW
  finalCanvas.height = targetH
  const finalCtx = finalCanvas.getContext('2d')!
  finalCtx.imageSmoothingEnabled = true
  finalCtx.imageSmoothingQuality = 'high'
  finalCtx.drawImage(curCanvas, 0, 0, curW, curH, 0, 0, targetW, targetH)
  return finalCanvas
}

// 获取完成裁剪、滤镜调色并去模糊优化后的高清晰度基础 Canvas (物理分辨率同原图比例或以 CROP_SIZE 的映射比例缩放)
function getBaseCroppedCanvas(isForcedOpaque = false, forceBgColor = '#ffffff'): HTMLCanvasElement {
  const img = uploadedImage.value
  if (!img) throw new Error('未载入图片')

  // 建立 1024x1024 的 Canvas 用于高清晰度导出
  const exportCanvas = document.createElement('canvas')
  exportCanvas.width = 1024
  exportCanvas.height = 1024
  const ctx = exportCanvas.getContext('2d')!

  const renderOpaque = isForcedOpaque || !options.isTransparent
  if (renderOpaque) {
    ctx.fillStyle = isForcedOpaque ? forceBgColor : options.bgColor
    ctx.fillRect(0, 0, 1024, 1024)
  }

  // 1. 设置色彩优化滤镜
  ctx.save()
  ctx.filter = `brightness(${options.brightness}%) contrast(${options.contrast}%) saturate(${options.saturate}%) hue-rotate(${options.hueRotate}deg)`

  // 2. 投射变换：利用预览画布的相对比例将原图完美缩放/平移映射到 1024x1024 的 Canvas 上
  const ratio = 1024 / CROP_SIZE
  const renderW = img.width * options.scale * ratio
  const renderH = img.height * options.scale * ratio
  const renderX = 1024 / 2 + options.offsetX * ratio - renderW / 2
  const renderY = 1024 / 2 + options.offsetY * ratio - renderH / 2

  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(img, renderX, renderY, renderW, renderH)
  ctx.filter = 'none'
  ctx.restore()

  // 3. 应用去模糊锐化滤波
  if (options.sharpen > 0) {
    applySharpen(exportCanvas, options.sharpen)
  }

  return exportCanvas
}

// 导出指定尺寸的 PNG / JPEG 的 Uint8Array 二进制
async function getExportImageBuffer(targetW: number, targetH: number, mimeType = 'image/png', isForcedOpaque = false, forceBgColor = '#ffffff'): Promise<Uint8Array> {
  const baseCanvas = getBaseCroppedCanvas(isForcedOpaque, forceBgColor)
  const resizedCanvas = getResizedCanvas(baseCanvas, targetW, targetH)
  
  return new Promise((resolve, reject) => {
    resizedCanvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error('Canvas 导出二进制失败'))
        return
      }
      const reader = new FileReader()
      reader.onload = () => {
        resolve(new Uint8Array(reader.result as ArrayBuffer))
      }
      reader.readAsArrayBuffer(blob)
    }, mimeType, mimeType === 'image/jpeg' ? 0.92 : undefined)
  })
}

// ================= ICO / ICNS 纯 JS 二进制组装算法 =================

// 组装 ICO 文件
function buildIcoBuffer(images: { width: number; height: number; buffer: Uint8Array }[]): Uint8Array {
  const numImages = images.length
  const dirSize = 16 * numImages
  const headerSize = 6
  let totalSize = headerSize + dirSize
  for (const img of images) {
    totalSize += img.buffer.length
  }

  const out = new Uint8Array(totalSize)
  const view = new DataView(out.buffer)

  // 1. Header (6 字节)
  view.setUint16(0, 0, true) // Reserved
  view.setUint16(2, 1, true) // Type: 1 = ICO
  view.setUint16(4, numImages, true) // 图像数量

  // 2. Directory Entries (每个 16 字节)
  let offset = headerSize + dirSize
  for (let i = 0; i < numImages; i++) {
    const img = images[i]
    const dirOffset = headerSize + i * 16

    const w = img.width >= 256 ? 0 : img.width
    const h = img.height >= 256 ? 0 : img.height

    view.setUint8(dirOffset, w)
    view.setUint8(dirOffset + 1, h)
    view.setUint8(dirOffset + 2, 0) // No color palette
    view.setUint8(dirOffset + 3, 0) // Reserved
    view.setUint16(dirOffset + 4, 1, true) // Color planes
    view.setUint16(dirOffset + 6, 32, true) // 32 bpp
    view.setUint32(dirOffset + 8, img.buffer.length, true) // 数据大小
    view.setUint32(dirOffset + 12, offset, true) // 数据在文件中的绝对偏移

    // 复制 PNG 数据到对应偏移量
    out.set(img.buffer, offset)
    offset += img.buffer.length
  }

  return out
}

// 组装 macOS ICNS 文件
function buildIcnsBuffer(images: { ostype: string; buffer: Uint8Array }[]): Uint8Array {
  let totalSize = 8
  for (const img of images) {
    totalSize += 8 + img.buffer.length
  }

  const out = new Uint8Array(totalSize)
  const view = new DataView(out.buffer)

  // 1. Header (8 字节，大端 Big Endian)
  out.set([105, 99, 110, 115], 0) // 'icns' 魔数
  view.setUint32(4, totalSize, false) // 写入总大小

  // 2. 数据块拼接
  let offset = 8
  const encoder = new TextEncoder()
  for (const img of images) {
    // 写入 4 字节 OSType (大端)
    const typeBytes = encoder.encode(img.ostype)
    out.set(typeBytes, offset)

    // 写入 4 字节块大小（含 8 字节块头）
    const blockSize = 8 + img.buffer.length
    view.setUint32(offset + 4, blockSize, false)

    // 写入 PNG 数据
    out.set(img.buffer, offset + 8)
    offset += blockSize
  }

  return out
}

// ================= ZIP 打包和多平台预设输出 =================

async function generateAllIcons() {
  if (!uploadedImage.value) {
    alert('请先上传图片！')
    return
  }

  isGenerating.value = true
  genProgress.value = 0
  genStatusText.value = '准备图像图层数据...'

  try {
    const zip = new JSZip()

    // 1. Web & PWA
    if (platforms.web) {
      genStatusText.value = '正在生成 Web/PWA 图标包...'
      const webFolder = zip.folder('web')!
      
      // Favicon.ico (包含 16x16, 32x32, 48x48)
      const f16 = await getExportImageBuffer(16, 16)
      const f32 = await getExportImageBuffer(32, 32)
      const f48 = await getExportImageBuffer(48, 48)
      const icoBuf = buildIcoBuffer([
        { width: 16, height: 16, buffer: f16 },
        { width: 32, height: 32, buffer: f32 },
        { width: 48, height: 48, buffer: f48 },
      ])
      webFolder.file('favicon.ico', icoBuf)

      // PWA & Mobile Icons
      const pwa192 = await getExportImageBuffer(192, 192)
      const pwa512 = await getExportImageBuffer(512, 512)
      const appleTouch = await getExportImageBuffer(180, 180)
      
      webFolder.file('android-chrome-192x192.png', pwa192)
      webFolder.file('android-chrome-512x512.png', pwa512)
      webFolder.file('apple-touch-icon.png', appleTouch)
      
      // site.webmanifest 配置文件
      const manifest = {
        name: 'My Application',
        short_name: 'App',
        icons: [
          { src: '/android-chrome-192x192.png', sizes: '192x192', type: 'image/png' },
          { src: '/android-chrome-512x512.png', sizes: '512x512', type: 'image/png' }
        ],
        theme_color: options.isTransparent ? '#ffffff' : options.bgColor,
        background_color: options.isTransparent ? '#ffffff' : options.bgColor,
        display: 'standalone'
      }
      webFolder.file('site.webmanifest', JSON.stringify(manifest, null, 2))
      genProgress.value = 25
    }

    // 2. Tauri / Electron 桌面端
    if (platforms.tauri) {
      genStatusText.value = '正在打包桌面端多合一图标 (ICO & ICNS)...'
      const tauriFolder = zip.folder('tauri')!

      // icon.ico (16, 32, 48, 64, 128, 256)
      const sizes = [16, 32, 48, 64, 128, 256]
      const icoImages = []
      for (const size of sizes) {
        const buf = await getExportImageBuffer(size, size)
        icoImages.push({ width: size, height: size, buffer: buf })
      }
      tauriFolder.file('icon.ico', buildIcoBuffer(icoImages))

      // icon.icns (16, 32, 64, 128, 256, 512, 1024)
      const icnsSizes = [
        { size: 16, ostype: 'icp4' },
        { size: 32, ostype: 'icp5' },
        { size: 64, ostype: 'icp6' },
        { size: 128, ostype: 'ic07' },
        { size: 256, ostype: 'ic08' },
        { size: 512, ostype: 'ic09' },
        { size: 1024, ostype: 'ic10' }
      ]
      const icnsImages = []
      for (const item of icnsSizes) {
        const buf = await getExportImageBuffer(item.size, item.size)
        icnsImages.push({ ostype: item.ostype, buffer: buf })
      }
      tauriFolder.file('icon.icns', buildIcnsBuffer(icnsImages))

      // 额外的标准尺寸 PNG
      const p32 = await getExportImageBuffer(32, 32)
      const p128 = await getExportImageBuffer(128, 128)
      const p256 = await getExportImageBuffer(256, 256)
      const p512 = await getExportImageBuffer(512, 512)
      tauriFolder.file('32x32.png', p32)
      tauriFolder.file('128x128.png', p128)
      tauriFolder.file('128x128@2x.png', p256)
      tauriFolder.file('icon.png', p512)

      genProgress.value = 50
    }

    // 3. iOS AppIcon.appiconset (强行去除透明通道，填充指定背景色)
    if (platforms.ios) {
      genStatusText.value = '正在绘制 iOS 苹果规格图标包...'
      const iosFolder = zip.folder('ios/AppIcon.appiconset')!

      // iOS 所需要的完整列表与文件命名映射关系
      const iosSpecs = [
        { size: 40, name: 'icon-20@2x.png' },
        { size: 60, name: 'icon-20@3x.png' },
        { size: 58, name: 'icon-29@2x.png' },
        { size: 87, name: 'icon-29@3x.png' },
        { size: 80, name: 'icon-40@2x.png' },
        { size: 120, name: 'icon-40@3x.png' },
        { size: 120, name: 'icon-60@2x.png' },
        { size: 180, name: 'icon-60@3x.png' },
        { size: 76, name: 'icon-76.png' },
        { size: 152, name: 'icon-76@2x.png' },
        { size: 167, name: 'icon-83.5@2x.png' },
        { size: 1024, name: 'icon-1024.png' } // App Store 营销图
      ]

      // 强制不透明，白色背景或者用户指定的不透明底色
      const forceBg = options.isTransparent ? '#ffffff' : options.bgColor

      for (const spec of iosSpecs) {
        const buf = await getExportImageBuffer(spec.size, spec.size, 'image/png', true, forceBg)
        iosFolder.file(spec.name, buf)
      }

      // 自动配装 Xcode 标准的 Contents.json 结构
      const contentsJson = {
        images: [
          { idiom: 'iphone', size: '20x20', scale: '2x', filename: 'icon-20@2x.png' },
          { idiom: 'iphone', size: '20x20', scale: '3x', filename: 'icon-20@3x.png' },
          { idiom: 'iphone', size: '29x29', scale: '2x', filename: 'icon-29@2x.png' },
          { idiom: 'iphone', size: '29x29', scale: '3x', filename: 'icon-29@3x.png' },
          { idiom: 'iphone', size: '40x40', scale: '2x', filename: 'icon-40@2x.png' },
          { idiom: 'iphone', size: '40x40', scale: '3x', filename: 'icon-40@3x.png' },
          { idiom: 'iphone', size: '60x60', scale: '2x', filename: 'icon-60@2x.png' },
          { idiom: 'iphone', size: '60x60', scale: '3x', filename: 'icon-60@3x.png' },
          { idiom: 'ipad', size: '20x20', scale: '1x', filename: 'icon-20.png' },
          { idiom: 'ipad', size: '20x20', scale: '2x', filename: 'icon-20@2x.png' },
          { idiom: 'ipad', size: '29x29', scale: '1x', filename: 'icon-29.png' },
          { idiom: 'ipad', size: '29x29', scale: '2x', filename: 'icon-29@2x.png' },
          { idiom: 'ipad', size: '40x40', scale: '1x', filename: 'icon-40.png' },
          { idiom: 'ipad', size: '40x40', scale: '2x', filename: 'icon-40@2x.png' },
          { idiom: 'ipad', size: '76x76', scale: '1x', filename: 'icon-76.png' },
          { idiom: 'ipad', size: '76x76', scale: '2x', filename: 'icon-76@2x.png' },
          { idiom: 'ipad', size: '83.5x83.5', scale: '2x', filename: 'icon-83.5@2x.png' },
          { idiom: 'ios-marketing', size: '1024x1024', scale: '1x', filename: 'icon-1024.png' }
        ],
        info: { author: 'xcode', version: 1 }
      }
      iosFolder.file('Contents.json', JSON.stringify(contentsJson, null, 2))

      genProgress.value = 75
    }

    // 4. Android 结构化 Mipmap PNG
    if (platforms.android) {
      genStatusText.value = '正在打包 Android 密度图标组...'
      const androidFolder = zip.folder('android/res')!

      const androidSpecs = [
        { density: 'mipmap-mdpi', size: 48 },
        { density: 'mipmap-hdpi', size: 72 },
        { density: 'mipmap-xhdpi', size: 96 },
        { density: 'mipmap-xxhdpi', size: 144 },
        { density: 'mipmap-xxxhdpi', size: 192 }
      ]

      for (const spec of androidSpecs) {
        // 导出普通的集成式 ic_launcher 图标
        const buf = await getExportImageBuffer(spec.size, spec.size)
        androidFolder.file(`${spec.density}/ic_launcher.png`, buf)

        // 导出 Adaptive 适应性图标所需要的前景图（保留透明通道）与背景图（用户设置的不透明色）
        const fgBuf = await getExportImageBuffer(spec.size, spec.size)
        const forceBg = options.isTransparent ? '#ffffff' : options.bgColor
        const bgBuf = await getExportImageBuffer(spec.size, spec.size, 'image/png', true, forceBg)

        androidFolder.file(`${spec.density}/ic_launcher_foreground.png`, fgBuf)
        androidFolder.file(`${spec.density}/ic_launcher_background.png`, bgBuf)
      }

      // anydpi-v26/ic_launcher.xml adaptive 矢量关联文件
      const adaptiveXml = `<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>`
      androidFolder.file('mipmap-anydpi-v26/ic_launcher.xml', adaptiveXml)

      genProgress.value = 90
    }

    // 5. 自定义导出
    if (platforms.custom) {
      genStatusText.value = '处理自定义尺寸与格式导出...'
      const customFolder = zip.folder('custom')!
      
      const w = customSettings.width
      const h = customSettings.height
      const fmt = customSettings.format

      if (fmt === 'png') {
        const buf = await getExportImageBuffer(w, h, 'image/png')
        customFolder.file(`custom_icon_${w}x${h}.png`, buf)
      } else if (fmt === 'jpg') {
        const buf = await getExportImageBuffer(w, h, 'image/jpeg')
        customFolder.file(`custom_icon_${w}x${h}.jpg`, buf)
      } else if (fmt === 'ico') {
        const customIcoImages = []
        for (const size of customSettings.icoSizes) {
          const buf = await getExportImageBuffer(size, size, 'image/png')
          customIcoImages.push({ width: size, height: size, buffer: buf })
        }
        customFolder.file(`custom_icon_${w}x${h}.ico`, buildIcoBuffer(customIcoImages))
      }
    }

    genStatusText.value = '正在打包并输出 ZIP 压缩包...'
    const content = await zip.generateAsync({ type: 'blob' })
    
    // 弹出文件保存位置选择器
    const filePath = await save({
      filters: [{ name: 'ZIP 压缩包', extensions: ['zip'] }],
      defaultPath: `app_icons_package_${Date.now()}.zip`,
      title: '选择图标包保存位置'
    })

    if (!filePath) {
      genStatusText.value = '已取消保存'
      isGenerating.value = false
      return
    }

    genStatusText.value = '正在写入本地磁盘...'
    const arrayBuffer = await content.arrayBuffer()
    const uint8Array = new Uint8Array(arrayBuffer)

    // 通过 Tauri Rust 后端直接写文件，避开 capabilities 的复杂限制
    await invoke('write_file_binary', { path: filePath, data: Array.from(uint8Array) })

    genProgress.value = 100
    genStatusText.value = '保存成功！已在文件夹中为您定位文件'

    // 自动在系统资源管理器中定位刚才保存的文件
    await invoke('show_in_folder', { path: filePath })
  } catch (error: any) {
    console.error(error)
    alert(`图标生成失败: ${error.message || error}`)
  } finally {
    isGenerating.value = false
  }
}

// 拖拽缩放平移及窗口改变响应
function handleResize() {
  drawPreview()
}

onMounted(() => {
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  if (imgMeta.src) {
    URL.revokeObjectURL(imgMeta.src)
  }
})
</script>

<template>
  <ToolShell title="图标生成器" split>
    <template #left-label>上传与画布裁剪编辑</template>
    <template #left>
      <!-- 左栏：上传区与画布裁剪编辑 -->
      <div class="edit-pane">
        <!-- 未上传状态 -->
        <div 
          v-if="!uploadedImage"
          class="flat-drop-zone flex-fill"
          :class="{ dragging: isDragging }"
          @dragover="onDragOver"
          @dragleave="onDragLeave"
          @drop="onDrop"
          @click="triggerFileInput"
        >
          <input type="file" ref="fileInput" @change="onFileSelect" accept="image/*,.svg" class="file-input-hidden" />
          <div class="flat-drop-content-vertical">
            <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="upload-icon"><rect width="18" height="18" x="3" y="3" rx="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>
            <span class="upload-msg-big">将您的 PNG, JPG, BMP 或 SVG 文件拖拽到此处，或 <strong>点击选择文件</strong></span>
            <span class="upload-tip-big">建议选择高分辨率的图片，小分辨率图可使用去模糊强化</span>
          </div>
        </div>

        <!-- 已上传编辑状态 -->
        <div v-else class="crop-workspace">
          <div class="workspace-header">
            <span class="meta-name" :title="imgMeta.name">{{ imgMeta.name }}</span>
            <span class="meta-specs">{{ imgMeta.width }} x {{ imgMeta.height }} Px</span>
            <JcButton size="small" danger @click="uploadedImage = null; uploadedFile = null; imgMeta.src = ''; resetAdjustments()">清除</JcButton>
          </div>

          <!-- 鼠标与滚轮操控画布 -->
          <div class="canvas-container">
            <canvas 
              ref="previewCanvas" 
              :width="PREVIEW_SIZE" 
              :height="PREVIEW_SIZE"
              class="workspace-canvas"
              @mousedown="onCanvasMouseDown"
              @wheel="onCanvasWheel"
            ></canvas>
            <div class="canvas-hud-tip">使用鼠标拖拽移动，滚动滚轮进行缩放</div>
          </div>

          <!-- 裁剪滑块参数调节 -->
          <div class="control-sliders">
            <div class="slider-row">
              <label>缩放 (Scale)</label>
              <input type="range" v-model.number="options.scale" min="0.1" max="4.0" step="0.01" />
              <span class="slider-value">{{ Math.round(options.scale * 100) }}%</span>
            </div>
            <div class="slider-row">
              <label>位移 X (Offset)</label>
              <input type="range" v-model.number="options.offsetX" min="-500" max="500" step="1" />
              <span class="slider-value">{{ options.offsetX }}px</span>
            </div>
            <div class="slider-row">
              <label>位移 Y (Offset)</label>
              <input type="range" v-model.number="options.offsetY" min="-500" max="500" step="1" />
              <span class="slider-value">{{ options.offsetY }}px</span>
            </div>
          </div>
        </div>
      </div>
    </template>

    <template #right-label>安全区与平台属性配置</template>
    <template #right>
      <!-- 右栏：安全区与平台属性配置 -->
      <div class="config-pane">
        <!-- 裁剪区域蒙版与底色 -->
        <div class="config-group">
          <div class="group-label">安全区裁剪模版预览</div>
          <div class="mask-options">
            <button :class="['mask-btn', { on: options.maskType === 'square' }]" @click="options.maskType = 'square'">方形</button>
            <button :class="['mask-btn', { on: options.maskType === 'ios' }]" @click="options.maskType = 'ios'">iOS (圆角)</button>
            <button :class="['mask-btn', { on: options.maskType === 'android' }]" @click="options.maskType = 'android'">Android (圆形)</button>
            <button :class="['mask-btn', { on: options.maskType === 'pwa' }]" @click="options.maskType = 'pwa'">PWA (可遮罩)</button>
          </div>
        </div>

        <!-- 新增：图像处理与去模糊面板 -->
        <div class="config-group" v-if="uploadedImage">
          <div class="group-label">画质优化与调色 (去模糊)</div>
          <div class="image-adjust-panel">
            <div class="adj-row">
              <label title="通过 3x3 拉普拉斯卷积算法重建像素边缘，提升模糊小图放大后的清晰度">清晰度 (锐化)</label>
              <input type="range" v-model.number="options.sharpen" min="0" max="100" step="1" />
              <span class="adj-value">{{ options.sharpen }}%</span>
            </div>
            <div class="adj-row">
              <label title="旋转色相以一键更换图标的整体主题色">色相换色 (Hue)</label>
              <input type="range" v-model.number="options.hueRotate" min="0" max="360" step="1" />
              <span class="adj-value">{{ options.hueRotate }}°</span>
            </div>
            <div class="adj-row">
              <label>饱和度 (Saturate)</label>
              <input type="range" v-model.number="options.saturate" min="0" max="200" step="5" />
              <span class="adj-value">{{ options.saturate }}%</span>
            </div>
            <div class="adj-row">
              <label>亮度 (Brightness)</label>
              <input type="range" v-model.number="options.brightness" min="50" max="150" step="1" />
              <span class="adj-value">{{ options.brightness }}%</span>
            </div>
            <div class="adj-row">
              <label>对比度 (Contrast)</label>
              <input type="range" v-model.number="options.contrast" min="50" max="150" step="1" />
              <span class="adj-value">{{ options.contrast }}%</span>
            </div>
            <JcButton size="small" block @click="resetAdjustments">重置所有调整</JcButton>
          </div>
        </div>

        <div class="config-group">
          <div class="group-label">背景填充与透明</div>
          <div class="bg-settings">
            <label class="check-container">
              <input type="checkbox" v-model="options.isTransparent" />
              <span class="checkmark"></span>
              保持透明背景 (PNG / ICO 导出有效)
            </label>
            <div v-if="!options.isTransparent" class="color-picker-wrap">
              <label>背景填充颜色</label>
              <div class="color-picker-row">
                <input type="color" v-model="options.bgColor" class="color-picker" />
                <input type="text" v-model="options.bgColor" class="color-hex-text" />
              </div>
            </div>
          </div>
        </div>

        <!-- 平台预设 -->
        <div class="config-group flex-fill-y">
          <div class="group-label">导出预设 (一键打包)</div>
          <div class="presets-list">
            <label class="check-container">
              <input type="checkbox" v-model="platforms.web" />
              <span class="checkmark"></span>
              <div class="preset-desc">
                <strong>Web & PWA 网页图标包</strong>
                <span>包含 favicon.ico、apple-touch-icon.png 及 manifest.json 声明</span>
              </div>
            </label>

            <label class="check-container">
              <input type="checkbox" v-model="platforms.tauri" />
              <span class="checkmark"></span>
              <div class="preset-desc">
                <strong>Tauri & Electron 桌面应用</strong>
                <span>包含 icon.ico (多合一)、icon.icns (macOS) 及标准 PNG 序列</span>
              </div>
            </label>

            <label class="check-container">
              <input type="checkbox" v-model="platforms.ios" />
              <span class="checkmark"></span>
              <div class="preset-desc">
                <strong>iOS AppStore 移动端</strong>
                <span>包含 AppIcon.appiconset 标准 Xcode 图片包（自动填充底色）</span>
              </div>
            </label>

            <label class="check-container">
              <input type="checkbox" v-model="platforms.android" />
              <span class="checkmark"></span>
              <div class="preset-desc">
                <strong>Android 移动端</strong>
                <span>包含 res/mipmap 各密度图片，支持 Adaptive 前背景隔离图</span>
              </div>
            </label>

            <label class="check-container">
              <input type="checkbox" v-model="platforms.custom" />
              <span class="checkmark"></span>
              <div class="preset-desc">
                <strong>自定义输出规格</strong>
                <span>允许按指定的高度、宽度和特定的单文件格式输出</span>
              </div>
            </label>

            <!-- 自定义配置展开域 -->
            <div v-if="platforms.custom" class="custom-specs-panel">
              <div class="custom-row">
                <div class="custom-col">
                  <label>宽度 (Px)</label>
                  <input type="number" v-model.number="customSettings.width" />
                </div>
                <div class="custom-col">
                  <label>高度 (Px)</label>
                  <input type="number" v-model.number="customSettings.height" />
                </div>
              </div>
              <div class="custom-row">
                <div class="custom-col">
                  <label>导出文件格式</label>
                  <JcSelect beam v-model="customSettings.format" :options="customFormatOptions" style="width: 100%" />
                </div>
              </div>
              <div v-if="customSettings.format === 'ico'" class="custom-row">
                <div class="custom-col">
                  <label>ICO 打包内嵌尺寸</label>
                  <div class="ico-size-checks">
                    <label v-for="sz in [16, 32, 48, 64, 128, 256]" :key="sz" class="sz-check">
                      <input type="checkbox" :value="sz" v-model="customSettings.icoSizes" />
                      {{ sz }}x{{ sz }}
                    </label>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 导出打包执行按钮 -->
        <div class="action-footer">
          <div v-if="isGenerating" class="progress-bar-container">
            <div class="progress-info">
              <span>{{ genStatusText }}</span>
              <span>{{ genProgress }}%</span>
            </div>
            <div class="progress-track">
              <div class="progress-fill" :style="{ width: genProgress + '%' }"></div>
            </div>
          </div>
          <JcButton
            v-else
            type="primary"
            block
            size="large"
            :disabled="!uploadedImage || (!platforms.web && !platforms.tauri && !platforms.ios && !platforms.android && !platforms.custom)"
            @click="generateAllIcons"
          >
            一键生成并打包 ZIP
          </JcButton>
        </div>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped lang="scss">
.file-input-hidden {
  position: absolute;
  width: 0;
  height: 0;
  opacity: 0;
  pointer-events: none;
}

/* ================= 左栏：编辑工作区 ================= */
.edit-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  justify-content: center;
}

.flex-fill {
  flex: 1;
  display: flex;
  height: 100%;
}

.flat-drop-zone {
  border: 2px dashed var(--jc-border-strong);
  background: var(--jc-bg-panel);
  border-radius: 6px;
  cursor: pointer;
  align-items: center;
  justify-content: center;
  transition: all 0.25s;
  &:hover, &.dragging {
    border-color: var(--jc-color-accent);
    background: var(--jc-bg-hover);
  }
}

.flat-drop-content-vertical {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  gap: 12px;
  padding: 24px;
  
  .upload-icon {
    color: var(--jc-color-accent);
    margin-bottom: 6px;
    animation: pulse 2s infinite ease-in-out;
  }
  .upload-msg-big {
    font-size: 13px;
    color: var(--jc-text-primary);
    strong {
      color: var(--jc-color-accent);
    }
  }
  .upload-tip-big {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}

@keyframes pulse {
  0% { transform: scale(1); opacity: 0.9; }
  50% { transform: scale(1.08); opacity: 1; }
  100% { transform: scale(1); opacity: 0.9; }
}

.crop-workspace {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 12px;
}

.workspace-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  border-bottom: 1px solid var(--jc-border-default);
  padding-bottom: 8px;
  
  .meta-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--jc-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 180px;
  }
  .meta-specs {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}

.canvas-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: #121216;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong);
  position: relative;
  overflow: hidden;
}

.workspace-canvas {
  cursor: grab;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  border-radius: 2px;
  &:active {
    cursor: grabbing;
  }
}

.canvas-hud-tip {
  position: absolute;
  bottom: 8px;
  background: rgba(0, 0, 0, 0.7);
  color: #c9c9d4;
  font-size: 10px;
  padding: 3px 8px;
  border-radius: 12px;
  pointer-events: none;
}

.control-sliders {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex-shrink: 0;
  background: var(--jc-bg-app);
  padding: 8px;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong);
}

.slider-row {
  display: flex;
  align-items: center;
  gap: 10px;
  label {
    width: 76px;
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
  input[type="range"] {
    flex: 1;
    height: 4px;
    background: var(--jc-border-strong);
    border-radius: 2px;
    outline: none;
    -webkit-appearance: none;
    &::-webkit-slider-thumb {
      -webkit-appearance: none;
      width: 12px;
      height: 12px;
      border-radius: 50%;
      background: var(--jc-color-accent);
      cursor: pointer;
      transition: transform 0.1s;
      &:hover {
        transform: scale(1.2);
      }
    }
  }
  .slider-value {
    width: 48px;
    text-align: right;
    font-size: 10px;
    font-family: monospace;
    color: var(--jc-text-primary);
  }
}

/* ================= 右栏：配置选项 ================= */
.config-pane {
  display: flex;
  flex-direction: column;
  flex: 0 0 320px;
  height: 100%;
  overflow-y: auto;
}

.config-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 12px;
  flex-shrink: 0;
  
  &.flex-fill-y {
    flex: 1;
    min-height: 0;
    margin-bottom: 0;
  }
}

.group-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-left: 2.5px solid var(--jc-color-accent);
  padding-left: 6px;
}

.mask-options {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 6px;
}

.mask-btn {
  background: var(--jc-bg-app);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-secondary);
  padding: 5px 0;
  font-size: 11px;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.2s;
  &:hover {
    background: var(--jc-bg-hover);
    color: var(--jc-text-primary);
  }
  &.on {
    border-color: var(--jc-color-accent);
    background: rgba(138, 88, 255, 0.1);
    color: var(--jc-color-accent);
    font-weight: 600;
  }
}

/* 调色与去模糊面板 */
.image-adjust-panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: var(--jc-bg-app);
  padding: 8px;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong);
}

.adj-row {
  display: flex;
  align-items: center;
  gap: 8px;
  label {
    width: 82px;
    font-size: 10px;
    color: var(--jc-text-secondary);
  }
  input[type="range"] {
    flex: 1;
    height: 3px;
    background: var(--jc-border-strong);
    border-radius: 2px;
    outline: none;
    -webkit-appearance: none;
    &::-webkit-slider-thumb {
      -webkit-appearance: none;
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: var(--jc-color-accent);
      cursor: pointer;
    }
  }
  .adj-value {
    width: 36px;
    text-align: right;
    font-size: 9.5px;
    font-family: monospace;
    color: var(--jc-text-primary);
  }
}

.reset-adj-btn {
  width: 100%;
  margin-top: 4px;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-strong);
  &:hover {
    border-color: var(--jc-color-accent);
    background: var(--jc-bg-hover);
  }
}

.bg-settings {
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--jc-bg-app);
  padding: 8px;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong);
}

.color-picker-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
  border-top: 1px solid var(--jc-border-strong);
  padding-top: 8px;
  margin-top: 4px;
  
  label {
    font-size: 10px;
    color: var(--jc-text-secondary);
  }
}

.color-picker-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.color-picker {
  -webkit-appearance: none;
  border: none;
  width: 24px;
  height: 24px;
  border-radius: 4px;
  cursor: pointer;
  background: none;
  &::-webkit-color-swatch-wrapper {
    padding: 0;
  }
  &::-webkit-color-swatch {
    border: 1px solid var(--jc-border-strong);
    border-radius: 4px;
  }
}

.color-hex-text {
  flex: 1;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 3px 6px;
  font-size: 11px;
  border-radius: 3px;
  outline: none;
  font-family: monospace;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}

.presets-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-right: 2px;
}

/* Custom Checkbox CSS */
.check-container {
  display: flex;
  position: relative;
  padding-left: 24px;
  cursor: pointer;
  font-size: 11px;
  color: var(--jc-text-primary);
  user-select: none;
  align-items: flex-start;
  
  input {
    position: absolute;
    opacity: 0;
    cursor: pointer;
    height: 0;
    width: 0;
  }
  
  .checkmark {
    position: absolute;
    top: 1px;
    left: 0;
    height: 14px;
    width: 14px;
    background-color: var(--jc-bg-input);
    border: 1.5px solid var(--jc-border-strong);
    border-radius: 3px;
    transition: all 0.2s;
  }
  
  &:hover input ~ .checkmark {
    border-color: var(--jc-color-accent);
  }
  
  input:checked ~ .checkmark {
    background-color: var(--jc-color-accent);
    border-color: var(--jc-color-accent);
  }
  
  .checkmark:after {
    content: "";
    position: absolute;
    display: none;
  }
  
  input:checked ~ .checkmark:after {
    display: block;
  }
  
  .checkmark:after {
    left: 4.5px;
    top: 1.5px;
    width: 3.5px;
    height: 7px;
    border: solid white;
    border-width: 0 1.5px 1.5px 0;
    transform: rotate(45deg);
  }
}

.preset-desc {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: -1px;
  
  strong {
    font-size: 11px;
    color: var(--jc-text-primary);
  }
  span {
    font-size: 9.5px;
    color: var(--jc-text-secondary);
    line-height: 1.2;
  }
}

/* 自定义设置面板扩展 */
.custom-specs-panel {
  background: var(--jc-bg-app);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  padding: 8px;
  margin-left: 24px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.custom-row {
  display: flex;
  gap: 8px;
}

.custom-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  
  label {
    font-size: 9.5px;
    color: var(--jc-text-secondary);
  }
  input[type="number"], select {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-strong);
    color: var(--jc-text-primary);
    padding: 4px 6px;
    font-size: 11px;
    border-radius: 3px;
    outline: none;
    width: 100%;
    &:focus {
      border-color: var(--jc-color-accent);
    }
  }
}

.ico-size-checks {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 4px;
  background: var(--jc-bg-panel);
  padding: 6px;
  border-radius: 3px;
  border: 1px solid var(--jc-border-strong);
}

.sz-check {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 9px;
  color: var(--jc-text-secondary);
  cursor: pointer;
  input {
    accent-color: var(--jc-color-accent);
    width: 11px;
    height: 11px;
  }
}

/* 按钮及进度条 */
.action-footer {
  margin-top: 12px;
  flex-shrink: 0;
}

.big-action {
  width: 100%;
  padding: 8px 0;
  font-size: 12px;
  font-weight: 600;
  border-radius: 4px;
}

.progress-bar-container {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  color: var(--jc-text-secondary);
}

.progress-track {
  height: 6px;
  background: var(--jc-border-strong);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--jc-color-accent);
  transition: width 0.15s ease-out;
}
</style>
