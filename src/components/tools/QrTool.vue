<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import QRCode from 'qrcode'
import jsQR from 'jsqr'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcSegmented from '@/components/ui/JcSegmented.vue'

const tabOptions = [
  { label: '生成二维码', value: 'generate' },
  { label: '解析二维码', value: 'parse' }
]
const gradientDirectionOptions = [
  { label: '水平渐变', value: 'horizontal' },
  { label: '垂直渐变', value: 'vertical' },
  { label: '对角渐变', value: 'diagonal' }
]
const dotTypeOptions = [
  { label: '直角方形 (普通)', value: 'square' },
  { label: '优雅圆点 (圆润)', value: 'round' },
  { label: '极简小圆 (多留白)', value: 'dot-small' },
  { label: '重叠大圆 (流体粗体)', value: 'overlap' },
  { label: '璀璨星形 (四角贝塞尔)', value: 'star' },
  { label: '倾斜菱形 (45度倾斜)', value: 'diamond' },
  { label: '平滑液态 (圆角融合)', value: 'liquid' },
  { label: '简约十字 (极简格子)', value: 'cross' }
]
const eyeTypeOptions = [
  { label: '方正经典 (外方内方)', value: 'square' },
  { label: '圆润圆角 (外圆角内圆角)', value: 'round' },
  { label: '前沿圆形 (外圆内圆)', value: 'circle' },
  { label: '时尚盾牌 (外盾内方)', value: 'shield' },
  { label: '左斜叶形 (左上右下圆角)', value: 'leaf-left' },
  { label: '右斜叶形 (右上左下圆角)', value: 'leaf-right' },
  { label: '外圆内方 (科技动感)', value: 'circle-square' },
  { label: '外方内圆 (特色嵌套)', value: 'square-circle' }
]
const labelSizeOptions = [
  { label: '11px (迷你)', value: 11 },
  { label: '13px (标准)', value: 13 },
  { label: '15px (较大)', value: 15 },
  { label: '17px (醒目)', value: 17 }
]
const marginOptions = [
  { label: '0个色块 (无白边)', value: 0 },
  { label: '1个色块 (极窄)', value: 1 },
  { label: '2个色块 (标准)', value: 2 },
  { label: '3个色块 (宽边)', value: 3 },
  { label: '4个色块 (超宽)', value: 4 }
]
const errorCorrectionOptions = [
  { label: '7% (L 级 - 低)', value: 'L' },
  { label: '15% (M 级 - 中)', value: 'M' },
  { label: '25% (Q 级 - 高)', value: 'Q' },
  { label: '30% (H 级 - 极高)', value: 'H' }
]

const activeTab = ref<'generate' | 'parse'>('generate')

// ================= 生成与美化状态 =================
const qrText = ref('https://github.com')
const qrCanvasRef = ref<HTMLCanvasElement | null>(null)
const errorCorrectionLevel = ref<'L' | 'M' | 'Q' | 'H'>('H') // 容错率，有Logo时强制定为 'H'
const margin = ref(2) // 码边距（0-4个逻辑格子）
const qrSize = ref(280) // 二维码本体渲染宽度
const generateError = ref('')

// 颜色设置
const fgColorMode = ref<'single' | 'gradient'>('single')
const fgColor = ref('#8a58ff')
const fgColor2 = ref('#00f0ff')
const fgGradientDirection = ref<'horizontal' | 'vertical' | 'diagonal'>('diagonal')
const bgColor = ref('#ffffff')

// 码点与码眼形状为 8 种
type DotShape = 'square' | 'round' | 'dot-small' | 'star' | 'diamond' | 'liquid' | 'overlap' | 'cross'
type EyeShape = 'square' | 'round' | 'circle' | 'shield' | 'leaf-left' | 'leaf-right' | 'circle-square' | 'square-circle'

const dotType = ref<DotShape>('square')
const eyeType = ref<EyeShape>('square')

// 码眼颜色
const eyeColorMode = ref<'follow' | 'custom'>('follow')
const eyeOuterColor = ref('#8a58ff')
const eyeInnerColor = ref('#00f0ff')

// LOGO 设置
const logoType = ref<'none' | 'upload' | 'preset'>('none')
const logoSrc = ref('')
const logoFileInput = ref<HTMLInputElement | null>(null)
const logoImageCached = ref<HTMLImageElement | null>(null)

// 底部文字设置
const drawText = ref(false)
const labelText = ref('扫码关注我们')
const labelColor = ref('')
const labelSize = ref(13)

// 6 种一键美化预置样式
interface PresetStyle {
  name: string
  desc: string
  fgColorMode: 'single' | 'gradient'
  fg: string
  fg2: string
  dir: 'horizontal' | 'vertical' | 'diagonal'
  bg: string
  dot: DotShape
  eyeShape: EyeShape
  eyeMode: 'follow' | 'custom'
  eyeOut: string
  eyeIn: string
}
const presets: PresetStyle[] = [
  {
    name: '经典黑白',
    desc: '传统的标准商务黑白二维码',
    fgColorMode: 'single', fg: '#000000', fg2: '#000000', dir: 'horizontal', bg: '#ffffff',
    dot: 'square', eyeShape: 'square', eyeMode: 'follow', eyeOut: '#000000', eyeIn: '#000000'
  },
  {
    name: '科技蓝海',
    desc: '渐变蓝色圆点二维码，搭配圆形码眼',
    fgColorMode: 'gradient', fg: '#0052d9', fg2: '#00c6ff', dir: 'diagonal', bg: '#ffffff',
    dot: 'round', eyeShape: 'circle', eyeMode: 'custom', eyeOut: '#0052d9', eyeIn: '#00c6ff'
  },
  {
    name: '极光紫境',
    desc: '酷炫暗黑风格，霓虹紫蓝渐变与外圆内方码眼',
    fgColorMode: 'gradient', fg: '#8a58ff', fg2: '#00f0ff', dir: 'diagonal', bg: '#12131a',
    dot: 'dot-small', eyeShape: 'circle-square', eyeMode: 'custom', eyeOut: '#8a58ff', eyeIn: '#00f0ff'
  },
  {
    name: '金秋落日',
    desc: '温暖橙红渐变，左斜叶形码眼与璀璨星点',
    fgColorMode: 'gradient', fg: '#ff5e62', fg2: '#ff9966', dir: 'horizontal', bg: '#fffaf6',
    dot: 'star', eyeShape: 'leaf-left', eyeMode: 'follow', eyeOut: '#ff5e62', eyeIn: '#ff5e62'
  },
  {
    name: '森林融合',
    desc: '清新淡雅绿，平滑液态融合，右斜叶形定位角',
    fgColorMode: 'gradient', fg: '#11998e', fg2: '#38ef7d', dir: 'vertical', bg: '#f4fcf7',
    dot: 'liquid', eyeShape: 'leaf-right', eyeMode: 'follow', eyeOut: '#11998e', eyeIn: '#11998e'
  },
  {
    name: '至尊黑金',
    desc: '奢华暗黑底色，方形嵌套圆形定位角',
    fgColorMode: 'gradient', fg: '#bf953f', fg2: '#fcf6ba', dir: 'diagonal', bg: '#1c1c1e',
    dot: 'square', eyeShape: 'square-circle', eyeMode: 'custom', eyeOut: '#bf953f', eyeIn: '#fcf6ba'
  }
]

function applyPreset(p: PresetStyle) {
  fgColorMode.value = p.fgColorMode
  fgColor.value = p.fg
  fgColor2.value = p.fg2
  fgGradientDirection.value = p.dir
  bgColor.value = p.bg
  dotType.value = p.dot
  eyeType.value = p.eyeShape
  eyeColorMode.value = p.eyeMode
  eyeOuterColor.value = p.eyeOut
  eyeInnerColor.value = p.eyeIn
  generateQr()
}

// 触发 LOGO 选择
function triggerLogoSelect() {
  logoFileInput.value?.click()
}

function handleLogoUpload(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files[0]) {
    const file = target.files[0]
    if (!file.type.startsWith('image/')) {
      alert('请选择有效的图片作为 LOGO！')
      return
    }
    const reader = new FileReader()
    reader.onload = (event) => {
      const dataUrl = event.target?.result as string
      logoSrc.value = dataUrl

      const img = new Image()
      img.onload = () => {
        logoImageCached.value = img
        generateQr() // 加载完成后重新渲染
      }
      img.src = dataUrl
    }
    reader.readAsDataURL(file)
  }
}

function clearLogo() {
  logoSrc.value = ''
  logoImageCached.value = null
  if (logoFileInput.value) logoFileInput.value.value = ''
  generateQr()
}

// JC CLI NINE 立体 Logo SVG 路径定义
const LOGO_P1 = new Path2D("M3805 7343 c-33 -14 -475 -264 -870 -493 -49 -29 -263 -152 -475 -273 -376 -215 -693 -398 -1140 -657 -143 -84 -209 -142 -242 -214 l-23 -51 0 -1535 0 -1535 28 -57 c15 -32 48 -74 71 -95 24 -21 189 -124 367 -228 296 -173 328 -190 371 -190 41 0 52 5 80 33 l33 32 5 1513 5 1512 33 67 c39 79 71 109 188 176 100 56 298 170 614 354 118 69 431 250 695 403 264 153 532 308 595 345 63 37 214 124 335 194 121 70 232 139 248 154 20 19 27 35 27 65 0 64 12 56 -500 350 -241 138 -262 147 -340 146 -36 0 -83 -7 -105 -16z")
const LOGO_P2 = new Path2D("M5389 6441 c-40 -13 -34 -9 -1139 -649 -410 -237 -529 -306 -970 -562 -135 -78 -305 -176 -379 -219 -161 -92 -211 -134 -248 -209 l-28 -57 0 -625 0 -625 35 -64 c21 -39 50 -75 75 -93 22 -16 195 -117 385 -225 190 -108 408 -232 485 -276 168 -95 210 -114 274 -123 99 -13 106 -10 566 258 138 79 311 180 385 223 74 42 176 101 225 130 366 214 596 345 618 351 18 4 36 1 53 -11 48 -31 52 -58 47 -304 -5 -213 -7 -229 -30 -282 -13 -31 -41 -72 -61 -92 -20 -19 -161 -107 -312 -195 -151 -87 -417 -241 -590 -342 -609 -355 -737 -428 -775 -440 -58 -19 -154 -16 -206 7 -51 23 -571 314 -894 502 -167 96 -196 102 -251 47 l-35 -35 3 -418 3 -418 27 -57 c14 -31 44 -73 65 -93 21 -20 130 -89 243 -153 113 -65 329 -189 480 -277 363 -209 384 -219 479 -219 97 1 105 4 443 202 249 145 475 277 1548 902 195 113 429 249 518 301 194 111 241 150 279 227 l28 57 3 1527 2 1527 -21 56 c-34 91 -99 149 -284 256 -200 115 -288 165 -545 314 -327 189 -303 178 -395 182 -44 1 -92 -1 -106 -6z m156 -1106 c84 -22 153 -77 195 -156 30 -56 34 -73 35 -139 0 -89 -30 -162 -92 -222 -21 -20 -227 -146 -458 -280 -1220 -710 -1189 -692 -1255 -714 -136 -44 -295 27 -360 162 -32 65 -38 164 -15 234 33 100 79 135 505 381 212 123 491 284 620 359 636 369 629 365 687 379 56 13 73 13 138 -4z")

// 绘制官方立体徽标
function drawPresetLogo(ctx: CanvasRenderingContext2D, x: number, y: number, size: number) {
  ctx.save()
  // 平移缩放到指定正方形区域内，映射 800x800 的 SVG 空间
  ctx.translate(x, y)
  ctx.scale(size / 800, size / 800)

  // 1. 绘制背景圆盘
  ctx.beginPath()
  ctx.arc(400, 400, 400, 0, Math.PI * 2)
  ctx.fillStyle = '#8a58ff'
  ctx.fill()

  // 2. 绘制白色图形主体 (套用原 SVG transform)
  ctx.translate(146.710431, 668.255454)
  ctx.scale(0.064986, -0.064986)
  ctx.fillStyle = '#ffffff'
  ctx.fill(LOGO_P1)
  ctx.fill(LOGO_P2)

  ctx.restore()
}

// 获取前景色填充样式 (单色或渐变色)
function getFgStyle(ctx: CanvasRenderingContext2D, width: number, height: number) {
  if (fgColorMode.value === 'single') {
    return fgColor.value
  }
  let grad: CanvasGradient
  if (fgGradientDirection.value === 'horizontal') {
    grad = ctx.createLinearGradient(0, 0, width, 0)
  } else if (fgGradientDirection.value === 'vertical') {
    grad = ctx.createLinearGradient(0, 0, 0, height)
  } else {
    grad = ctx.createLinearGradient(0, 0, width, height)
  }
  grad.addColorStop(0, fgColor.value)
  grad.addColorStop(1, fgColor2.value)
  return grad
}

// 获取定位角颜色
function getEyeStyle(ctx: CanvasRenderingContext2D, width: number, height: number, type: 'outer' | 'inner') {
  if (eyeColorMode.value === 'custom') {
    return type === 'outer' ? eyeOuterColor.value : eyeInnerColor.value
  }
  return getFgStyle(ctx, width, height)
}

// ================= 核心手绘 Canvas 算法 =================
function generateQr() {
  generateError.value = ''
  if (!qrCanvasRef.value) return

  if (!qrText.value.trim()) {
    const ctx = qrCanvasRef.value.getContext('2d')
    ctx?.clearRect(0, 0, qrCanvasRef.value.width, qrCanvasRef.value.height)
    return
  }

  try {
    const hasLogo = (logoType.value === 'preset') || (logoType.value === 'upload' && logoSrc.value)
    // 1. 获取原始 QR 数据矩阵。有 Logo 时，强制设为最高级别 H 容错
    const level = hasLogo ? 'H' : errorCorrectionLevel.value
    const qrData = QRCode.create(qrText.value, { errorCorrectionLevel: level })
    const modules = qrData.modules
    const size = modules.size // 二维码核心格子数

    // 2. 自定义边距与 Canvas 高度计算（考虑底部文字空间）
    const logicalMargin = margin.value
    const totalGridSize = size + logicalMargin * 2 // 包含边距的逻辑格子数
    
    const canvas = qrCanvasRef.value
    const textSpaceHeight = (drawText.value && labelText.value.trim()) ? Math.max(34, labelSize.value + 18) : 0
    
    canvas.width = qrSize.value
    canvas.height = qrSize.value + textSpaceHeight
    
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    // 3. 清空画布并绘制全局背景色
    ctx.fillStyle = bgColor.value
    ctx.fillRect(0, 0, canvas.width, canvas.height)

    // 单个单元格的物理像素宽度
    const w = qrSize.value / totalGridSize

    // 定义定位角(Finder Pattern)坐标范围，用于在绘制普通码点时避开
    const isFinderArea = (x: number, y: number): boolean => {
      if (x >= 0 && x <= 6 && y >= 0 && y <= 6) return true // 左上
      if (x >= size - 7 && x < size && y >= 0 && y <= 6) return true // 右上
      if (x >= 0 && x <= 6 && y >= size - 7 && y < size) return true // 左下
      return false
    }

    // 定义 LOGO 避让中央区域
    const center = Math.floor(size / 2)
    const logoRadius = Math.ceil(size * 0.12) // 避开中心约占 24% 的码点区域
    const isLogoArea = (x: number, y: number): boolean => {
      if (!hasLogo) return false
      return x >= center - logoRadius && x <= center + logoRadius &&
             y >= center - logoRadius && y <= center + logoRadius
    }

    // 液态融合辅助函数：检测该点是否真的存在有效码点且不属于 Finder/Logo 干扰区
    const hasModule = (nx: number, ny: number): boolean => {
      if (nx < 0 || nx >= size || ny < 0 || ny >= size) return false
      if (!modules.get(nx, ny)) return false
      if (isFinderArea(nx, ny) || isLogoArea(nx, ny)) return false
      return true
    }

    // 4. 绘制普通码点 (Data Modules)
    const fgStyle = getFgStyle(ctx, qrSize.value, qrSize.value)
    ctx.fillStyle = fgStyle

    for (let x = 0; x < size; x++) {
      for (let y = 0; y < size; y++) {
        if (modules.get(x, y)) {
          // 跳过定位角与 LOGO 区域
          if (isFinderArea(x, y) || isLogoArea(x, y)) continue

          // 计算包含边距后的物理绘制坐标
          const px = (x + logicalMargin) * w
          const py = (y + logicalMargin) * w

          if (dotType.value === 'round') {
            ctx.beginPath()
            ctx.arc(px + w / 2, py + w / 2, w / 2 * 0.85, 0, Math.PI * 2)
            ctx.fill()
          } else if (dotType.value === 'dot-small') {
            ctx.beginPath()
            ctx.arc(px + w / 2, py + w / 2, w / 2 * 0.52, 0, Math.PI * 2)
            ctx.fill()
          } else if (dotType.value === 'overlap') {
            ctx.beginPath()
            ctx.arc(px + w / 2, py + w / 2, w / 2 * 1.16, 0, Math.PI * 2)
            ctx.fill()
          } else if (dotType.value === 'star') {
            // 贝塞尔四角星形（向内收缩弯曲）
            const cx = px + w / 2
            const cy = py + w / 2
            const r = w / 2 * 0.95
            ctx.beginPath()
            ctx.moveTo(cx, cy - r)
            ctx.quadraticCurveTo(cx, cy, cx + r, cy)
            ctx.quadraticCurveTo(cx, cy, cx, cy + r)
            ctx.quadraticCurveTo(cx, cy, cx - r, cy)
            ctx.quadraticCurveTo(cx, cy, cx, cy - r)
            ctx.closePath()
            ctx.fill()
          } else if (dotType.value === 'diamond') {
            // 45度斜切菱形
            const cx = px + w / 2
            const cy = py + w / 2
            const r = w / 2 * 0.95
            ctx.beginPath()
            ctx.moveTo(cx, cy - r)
            ctx.lineTo(cx + r, cy)
            ctx.lineTo(cx, cy + r)
            ctx.lineTo(cx - r, cy)
            ctx.closePath()
            ctx.fill()
          } else if (dotType.value === 'cross') {
            // 简约十字
            const cx = px + w / 2
            const cy = py + w / 2
            const thickness = w * 0.25
            ctx.fillRect(px, cy - thickness / 2, w, thickness)
            ctx.fillRect(cx - thickness / 2, py, thickness, w)
          } else if (dotType.value === 'liquid') {
            // 平滑液态融合算法 (检测邻近格子并设置四个角的圆角半径)
            const top = hasModule(x, y - 1)
            const bottom = hasModule(x, y + 1)
            const left = hasModule(x - 1, y)
            const right = hasModule(x + 1, y)

            const rTL = (!left && !top) ? w / 2 : 0
            const rTR = (!right && !top) ? w / 2 : 0
            const rBR = (!right && !bottom) ? w / 2 : 0
            const rBL = (!left && !bottom) ? w / 2 : 0

            ctx.beginPath()
            ctx.roundRect ? ctx.roundRect(px, py, w, w, [rTL, rTR, rBR, rBL]) : ctx.rect(px, py, w, w)
            ctx.fill()
          } else {
            // square: 方点微幅溢出 0.5px，防 Canvas 缩放白缝
            ctx.fillRect(px, py, w + 0.5, w + 0.5)
          }
        }
      }
    }

    // 5. 手绘三个定位角 (Finder Pattern)
    const drawFinderPattern = (logicalX: number, logicalY: number) => {
      const isRound = eyeType.value === 'round'
      const isCircle = eyeType.value === 'circle'
      const isShield = eyeType.value === 'shield'
      const isLeafLeft = eyeType.value === 'leaf-left'
      const isLeafRight = eyeType.value === 'leaf-right'
      const isCircleSquare = eyeType.value === 'circle-square'
      const isSquareCircle = eyeType.value === 'square-circle'

      const outerC = getEyeStyle(ctx, qrSize.value, qrSize.value, 'outer')
      const innerC = getEyeStyle(ctx, qrSize.value, qrSize.value, 'inner')

      const x1 = logicalX * w
      const y1 = logicalY * w

      // A. 外圈 (7x7 逻辑格)
      const d1 = 7 * w
      ctx.fillStyle = outerC
      ctx.beginPath()
      if (isCircle || isCircleSquare) {
        ctx.arc(x1 + d1 / 2, y1 + d1 / 2, d1 / 2, 0, Math.PI * 2)
      } else if (isRound) {
        ctx.roundRect ? ctx.roundRect(x1, y1, d1, d1, w * 1.6) : ctx.rect(x1, y1, d1, d1)
      } else if (isShield) {
        ctx.roundRect ? ctx.roundRect(x1, y1, d1, d1, w * 2.2) : ctx.rect(x1, y1, d1, d1)
      } else if (isLeafLeft) {
        // 左斜叶形
        ctx.roundRect ? ctx.roundRect(x1, y1, d1, d1, [w * 3.2, 0, w * 3.2, 0]) : ctx.rect(x1, y1, d1, d1)
      } else if (isLeafRight) {
        // 右斜叶形
        ctx.roundRect ? ctx.roundRect(x1, y1, d1, d1, [0, w * 3.2, 0, w * 3.2]) : ctx.rect(x1, y1, d1, d1)
      } else {
        ctx.rect(x1, y1, d1, d1)
      }
      ctx.fill()

      // B. 中空背景 (5x5 逻辑格，向内偏移 1 格)
      const x2 = (logicalX + 1) * w
      const y2 = (logicalY + 1) * w
      const d2 = 5 * w
      ctx.fillStyle = bgColor.value
      ctx.beginPath()
      if (isCircle || isSquareCircle) {
        ctx.arc(x2 + d2 / 2, y2 + d2 / 2, d2 / 2, 0, Math.PI * 2)
      } else if (isRound) {
        ctx.roundRect ? ctx.roundRect(x2, y2, d2, d2, w * 1.0) : ctx.rect(x2, y2, d2, d2)
      } else if (isLeafLeft) {
        ctx.roundRect ? ctx.roundRect(x2, y2, d2, d2, [w * 2.0, 0, w * 2.0, 0]) : ctx.rect(x2, y2, d2, d2)
      } else if (isLeafRight) {
        ctx.roundRect ? ctx.roundRect(x2, y2, d2, d2, [0, w * 2.0, 0, w * 2.0]) : ctx.rect(x2, y2, d2, d2)
      } else {
        ctx.rect(x2, y2, d2, d2)
      }
      ctx.fill()

      // C. 内芯 (3x3 逻辑格，向内偏移 2 格)
      const x3 = (logicalX + 2) * w
      const y3 = (logicalY + 2) * w
      const d3 = 3 * w
      ctx.fillStyle = innerC
      ctx.beginPath()
      if (isCircle || isSquareCircle) {
        ctx.arc(x3 + d3 / 2, y3 + d3 / 2, d3 / 2, 0, Math.PI * 2)
      } else if (isRound) {
        ctx.roundRect ? ctx.roundRect(x3, y3, d3, d3, w * 0.6) : ctx.rect(x3, y3, d3, d3)
      } else if (isLeafLeft) {
        ctx.roundRect ? ctx.roundRect(x3, y3, d3, d3, [w * 1.2, 0, w * 1.2, 0]) : ctx.rect(x3, y3, d3, d3)
      } else if (isLeafRight) {
        ctx.roundRect ? ctx.roundRect(x3, y3, d3, d3, [0, w * 1.2, 0, w * 1.2]) : ctx.rect(x3, y3, d3, d3)
      } else {
        ctx.rect(x3, y3, d3, d3)
      }
      ctx.fill()
    }

    drawFinderPattern(logicalMargin, logicalMargin) // 左上
    drawFinderPattern(size - 7 + logicalMargin, logicalMargin) // 右上
    drawFinderPattern(logicalMargin, size - 7 + logicalMargin) // 左下

    // 6. 居中绘制 LOGO 与白色安全遮罩层
    if (hasLogo) {
      const cx = logicalMargin * w + (size * w) / 2
      const cy = logicalMargin * w + (size * w) / 2
      
      const logoSize = size * w * 0.21
      const maskSize = size * w * 0.25

      // A. 圆角白底保护层
      ctx.fillStyle = '#ffffff'
      ctx.beginPath()
      ctx.roundRect
        ? ctx.roundRect(cx - maskSize / 2, cy - maskSize / 2, maskSize, maskSize, maskSize * 0.16)
        : ctx.rect(cx - maskSize / 2, cy - maskSize / 2, maskSize, maskSize)
      ctx.fill()

      // B. 绘制 LOGO
      if (logoType.value === 'upload' && logoSrc.value && logoImageCached.value) {
        ctx.save()
        ctx.beginPath()
        ctx.roundRect
          ? ctx.roundRect(cx - logoSize / 2, cy - logoSize / 2, logoSize, logoSize, logoSize * 0.16)
          : ctx.rect(cx - logoSize / 2, cy - logoSize / 2, logoSize, logoSize)
        ctx.clip()
        ctx.drawImage(logoImageCached.value, cx - logoSize / 2, cy - logoSize / 2, logoSize, logoSize)
        ctx.restore()
      } else if (logoType.value === 'preset') {
        drawPresetLogo(ctx, cx - logoSize / 2, cy - logoSize / 2, logoSize)
      }
    }

    // 7. 绘制底部文本
    if (drawText.value && labelText.value.trim()) {
      ctx.fillStyle = labelColor.value || (fgColorMode.value === 'single' ? fgColor.value : fgColor2.value)
      ctx.font = `bold ${labelSize.value}px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`
      ctx.textAlign = 'center'
      ctx.textBaseline = 'middle'
      
      const textX = qrSize.value / 2
      const textY = qrSize.value + textSpaceHeight / 2
      ctx.fillText(labelText.value, textX, textY)
    }

  } catch (err: any) {
    generateError.value = '二维码美化渲染失败: ' + err.message
  }
}

watch(
  [
    qrText, errorCorrectionLevel, margin, fgColorMode, fgColor, fgColor2,
    fgGradientDirection, bgColor, dotType, eyeType, eyeColorMode, eyeOuterColor,
    eyeInnerColor, logoType, drawText, labelText, labelColor, labelSize
  ],
  () => {
    generateQr()
  }
)

function downloadQr() {
  if (!qrCanvasRef.value || !qrText.value.trim()) return
  try {
    const dataUrl = qrCanvasRef.value.toDataURL('image/png')
    const a = document.createElement('a')
    a.href = dataUrl
    a.download = `qrcode_beautified_${Date.now()}.png`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
  } catch (e) {
    alert('保存高清美化二维码失败')
  }
}

// ================= 解析二维码 =================
const parseFile = ref<File | null>(null)
const parsePreview = ref('')
const parsedResult = ref('')
const parseError = ref('')
const isDragging = ref(false)
const parseFileInput = ref<HTMLInputElement | null>(null)

function triggerParseFileInput() {
  parseFileInput.value?.click()
}

function handleParseFile(file: File) {
  parseError.value = ''
  parsedResult.value = ''
  parsePreview.value = ''
  parseFile.value = file

  if (!file.type.startsWith('image/')) {
    parseError.value = '请上传有效的图片格式二维码'
    return
  }

  const reader = new FileReader()
  reader.onload = (e) => {
    const dataUrl = e.target?.result as string
    parsePreview.value = dataUrl

    const img = new Image()
    img.onload = () => {
      const canvas = document.createElement('canvas')
      canvas.width = img.width
      canvas.height = img.height
      const ctx = canvas.getContext('2d')
      if (!ctx) {
        parseError.value = 'Canvas 初始化失败，无法解析'
        return
      }
      ctx.drawImage(img, 0, 0)
      
      try {
        const imageData = ctx.getImageData(0, 0, img.width, img.height)
        const code = jsQR(imageData.data, imageData.width, imageData.height, {
          inversionAttempts: 'dontInvert'
        })
        if (code && code.data) {
          parsedResult.value = code.data
        } else {
          parseError.value = '未能检测到有效的二维码，请尝试提高对比度或更换清晰的图片'
        }
      } catch (err: any) {
        parseError.value = '解析二维码错误: ' + err.message
      }
    }
    img.onerror = () => {
      parseError.value = '二维码图片载入失败'
    }
    img.src = dataUrl
  }
  reader.readAsDataURL(file)
}

function onFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files[0]) {
    handleParseFile(target.files[0])
  }
}

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
    handleParseFile(e.dataTransfer.files[0])
  }
}

// 自动生成初始状态
onMounted(() => {
  if (activeTab.value === 'generate') {
    generateQr()
  }
})

function copyParsedResult() {
  if (!parsedResult.value) return
  navigator.clipboard.writeText(parsedResult.value)
}

function clearParse() {
  parseFile.value = null
  parsePreview.value = ''
  parsedResult.value = ''
  parseError.value = ''
}
</script>

<template>
  <ToolShell title="二维码生成 / 解析">
    <template #actions>
      <JcSegmented
        :model-value="activeTab"
        :options="tabOptions"
        size="small"
        @update:model-value="(v) => activeTab = v as 'generate' | 'parse'"
      />
    </template>

    <!-- Tab 1: 生成与个性化美化 -->
    <div v-if="activeTab === 'generate'" class="tool-body-split">
      <div class="settings-scroll-pane">
        <!-- 模块一：内容 -->
        <div class="setting-section">
          <div class="section-subtitle">1. 文本与链接内容</div>
          <JcTextarea v-model="qrText" beam :beam-size-ratio="0.6" :rows="4" placeholder="在此输入需要转换成二维码的文本或 URL 链接..." />
        </div>

        <!-- 模块二：一键美化预置样式 -->
        <div class="setting-section">
          <div class="section-subtitle">2. 一键应用预设样式</div>
          <div class="presets-row-grid">
            <button v-for="p in presets" :key="p.name" class="preset-card" @click="applyPreset(p)" :title="p.desc">
              <div class="preset-badge" :style="{ background: p.fgColorMode === 'single' ? p.fg : `linear-gradient(135deg, ${p.fg}, ${p.fg2})`, color: p.bg === '#ffffff' ? '#333' : '#fff' }">Aa</div>
              <span class="preset-name">{{ p.name }}</span>
            </button>
          </div>
        </div>

        <!-- 模块三：LOGO 嵌入设置 -->
        <div class="setting-section">
          <div class="section-subtitle">3. 二维码 LOGO 设置</div>
          <div class="logo-type-row">
            <label class="radio-label">
              <input type="radio" value="none" v-model="logoType" />
              <span>无 Logo</span>
            </label>
            <label class="radio-label">
              <input type="radio" value="preset" v-model="logoType" />
              <span>程序 LOGO</span>
            </label>
            <label class="radio-label">
              <input type="radio" value="upload" v-model="logoType" />
              <span>上传 LOGO</span>
            </label>
          </div>

          <!-- 自定义上传选项 -->
          <div v-if="logoType === 'upload'" class="logo-control-row mt-8">
            <input type="file" ref="logoFileInput" @change="handleLogoUpload" accept="image/*" class="file-input-hidden" />
            <JcButton type="primary" size="small" @click="triggerLogoSelect">上传 LOGO 图片</JcButton>
            <JcButton v-if="logoSrc" size="small" danger @click="clearLogo">清除 LOGO</JcButton>
            <div v-if="logoSrc" class="logo-preview-box">
              <img :src="logoSrc" alt="logo-preview" />
            </div>
          </div>
        </div>

        <!-- 模块四：个性化细节配置 -->
        <div class="setting-section">
          <div class="section-subtitle">4. 颜色与细节微调</div>
          
          <!-- 二维码颜色 -->
          <div class="config-group-card">
            <div class="group-title">前景色模式</div>
            <div class="logo-type-row">
              <label class="radio-label">
                <input type="radio" value="single" v-model="fgColorMode" />
                <span>纯色模式</span>
              </label>
              <label class="radio-label">
                <input type="radio" value="gradient" v-model="fgColorMode" />
                <span>渐变色模式</span>
              </label>
            </div>
            
            <div class="flex-config-row mt-8">
              <div class="config-field">
                <label>{{ fgColorMode === 'gradient' ? '渐变起始色' : '码点颜色' }}</label>
                <div class="color-picker-input-wrap">
                  <input type="color" v-model="fgColor" />
                  <input type="text" v-model="fgColor" class="color-hex-text" />
                </div>
              </div>
              <div v-if="fgColorMode === 'gradient'" class="config-field">
                <label>渐变终止色</label>
                <div class="color-picker-input-wrap">
                  <input type="color" v-model="fgColor2" />
                  <input type="text" v-model="fgColor2" class="color-hex-text" />
                </div>
              </div>
              <div v-if="fgColorMode === 'gradient'" class="config-field">
                <label>渐变方向</label>
                <JcSelect beam :model-value="fgGradientDirection" :options="gradientDirectionOptions" style="width: 100%" @update:model-value="(v) => fgGradientDirection = v as 'horizontal' | 'vertical' | 'diagonal'" />
              </div>
            </div>
          </div>

          <!-- 背景色与细节样式 -->
          <div class="flex-config-row mt-10">
            <div class="config-field">
              <label>背景颜色</label>
              <div class="color-picker-input-wrap">
                <input type="color" v-model="bgColor" />
                <input type="text" v-model="bgColor" class="color-hex-text" />
              </div>
            </div>
            <div class="config-field">
              <label>码点形状 (Data Dots)</label>
              <JcSelect beam :model-value="dotType" :options="dotTypeOptions" style="width: 100%" @update:model-value="(v) => dotType = v as DotShape" />
            </div>
            <div class="config-field">
              <label>码眼形状 (Finder Eyes)</label>
              <JcSelect beam :model-value="eyeType" :options="eyeTypeOptions" style="width: 100%" @update:model-value="(v) => eyeType = v as EyeShape" />
            </div>
          </div>

          <!-- 码眼颜色配置 -->
          <div class="config-group-card mt-10">
            <div class="group-title">定位码眼颜色</div>
            <div class="logo-type-row">
              <label class="radio-label">
                <input type="radio" value="follow" v-model="eyeColorMode" />
                <span>跟随前景色</span>
              </label>
              <label class="radio-label">
                <input type="radio" value="custom" v-model="eyeColorMode" />
                <span>自定义颜色</span>
              </label>
            </div>
            <div v-if="eyeColorMode === 'custom'" class="flex-config-row mt-8">
              <div class="config-field">
                <label>外框颜色 (Outer)</label>
                <div class="color-picker-input-wrap">
                  <input type="color" v-model="eyeOuterColor" />
                  <input type="text" v-model="eyeOuterColor" class="color-hex-text" />
                </div>
              </div>
              <div class="config-field">
                <label>内芯颜色 (Inner)</label>
                <div class="color-picker-input-wrap">
                  <input type="color" v-model="eyeInnerColor" />
                  <input type="text" v-model="eyeInnerColor" class="color-hex-text" />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 模块五：底部添加文字 -->
        <div class="setting-section">
          <div class="section-subtitle">5. 底部添加文字</div>
          <div class="checkbox-control-row">
            <label class="checkbox-label">
              <input type="checkbox" v-model="drawText" />
              <span>在二维码下方绘制文字</span>
            </label>
          </div>
          <div v-if="drawText" class="flex-config-row mt-8">
            <div class="config-field flex-2">
              <label>文字内容</label>
              <JcInput beam v-model="labelText" placeholder="例如：扫码查看详情" />
            </div>
            <div class="config-field">
              <label>文字颜色</label>
              <div class="color-picker-input-wrap">
                <input type="color" v-model="labelColor" />
                <input type="text" v-model="labelColor" class="color-hex-text" placeholder="为空则默认" />
              </div>
            </div>
            <div class="config-field">
              <label>字号大小</label>
              <JcSelect beam :model-value="labelSize" :options="labelSizeOptions" style="width: 100%" @update:model-value="(v) => labelSize = Number(v)" />
            </div>
          </div>
        </div>

        <!-- 模块六：参数规格 -->
        <div class="setting-section">
          <div class="section-subtitle">6. 生成规格参数</div>
          <div class="flex-config-row">
            <div class="config-field">
              <label>码边距 (Margins)</label>
              <JcSelect beam :model-value="margin" :options="marginOptions" style="width: 100%" @update:model-value="(v) => margin = Number(v)" />
            </div>
            <div class="config-field">
              <label>二维码容错率</label>
              <JcSelect beam :model-value="errorCorrectionLevel" :options="errorCorrectionOptions" :disabled="logoType !== 'none'" style="width: 100%" @update:model-value="(v) => errorCorrectionLevel = v as 'L' | 'M' | 'Q' | 'H'" />
              <span v-if="logoType !== 'none'" class="field-hint">嵌入 Logo 时强制为最高容错 (30%)</span>
            </div>
          </div>
        </div>
      </div>

      <div class="display-pane">
        <div class="pane-label">实时生成预览</div>
        <div class="qr-canvas-wrapper" :style="{ backgroundColor: bgColor }">
          <canvas ref="qrCanvasRef" class="qr-canvas"></canvas>
        </div>
        <JcButton type="primary" block @click="downloadQr" :disabled="!qrText.trim()">下载高清美化二维码</JcButton>
        <div v-if="generateError" class="tool-footer-error style-inline">{{ generateError }}</div>
      </div>
    </div>

    <!-- Tab 2: 解析二维码 -->
    <div v-else class="tool-body-split">
      <div class="upload-pane">
        <div 
          class="drop-zone"
          :class="{ dragging: isDragging }"
          @dragover="onDragOver"
          @dragleave="onDragLeave"
          @drop="onDrop"
          @click="triggerParseFileInput"
        >
          <input type="file" ref="parseFileInput" @change="onFileSelect" accept="image/*" class="file-input-hidden" />
          <div v-if="!parsePreview" class="drop-msg">
            <svg viewBox="0 0 24 24" width="36" height="36" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12"/></svg>
            <p>点击或拖入一张二维码图片</p>
            <span class="file-tip">支持常见格式（PNG, JPG, WEBP 等）</span>
          </div>
          <div v-else class="preview-img-container" @click.stop>
            <img :src="parsePreview" alt="qr-preview" />
            <button class="remove-upload-btn" @click="clearParse" title="清除重选">✕</button>
          </div>
        </div>
      </div>

      <div class="results-pane">
        <div class="pane-label-bar">
          <span>二维码解码结果</span>
          <div class="pane-acts">
            <JcButton type="primary" size="small" @click="copyParsedResult" :disabled="!parsedResult">复制内容</JcButton>
            <JcButton size="small" danger @click="clearParse">重置</JcButton>
          </div>
        </div>
        
        <div class="decode-result-box" :class="{ error: parseError }">
          <div v-if="parsedResult" class="result-text">{{ parsedResult }}</div>
          <div v-else-if="parseError" class="result-error-msg">{{ parseError }}</div>
          <div v-else class="result-empty">等待上传二维码图片解码...</div>
        </div>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped lang="scss">
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 16px;
  min-height: 0;
}

/* 左侧美化配置面板滚动区 */
.settings-scroll-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 14px;
  border-radius: 4px;
  gap: 16px;
  overflow-y: auto;
}

.setting-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  padding-bottom: 12px;
  &:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }
}
.section-subtitle {
  font-size: 11px;
  font-weight: 700;
  color: var(--jc-text-primary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-left: 2px solid var(--jc-color-accent);
  padding-left: 6px;
  line-height: 1.2;
}

/* 预设行网格 */
.presets-row-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 6px;
}
.preset-card {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  padding: 6px 3px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  color: var(--jc-text-primary);
  transition: all 0.2s;
  &:hover {
    border-color: var(--jc-color-accent);
    background: var(--jc-bg-hover);
  }
}
.preset-badge {
  font-size: 12px;
  font-weight: 900;
  width: 26px;
  height: 26px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(0, 0, 0, 0.1);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}
.preset-name {
  font-size: 9px;
  font-weight: 600;
  text-align: center;
  white-space: nowrap;
}

/* 选项卡/选择布局 */
.logo-type-row {
  display: flex;
  gap: 16px;
  padding: 4px 0;
}
.radio-label, .checkbox-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--jc-text-primary);
  cursor: pointer;
  
  input[type="radio"], input[type="checkbox"] {
    accent-color: var(--jc-color-accent);
    cursor: pointer;
    margin: 0;
  }
}
.checkbox-control-row {
  display: flex;
  align-items: center;
  padding: 2px 0;
}

/* LOGO 控制 */
.logo-control-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.logo-preview-box {
  width: 26px;
  height: 26px;
  border-radius: 3px;
  background: #ffffff;
  border: 1px solid var(--jc-border-strong);
  padding: 2px;
  display: flex;
  align-items: center;
  justify-content: center;
  img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 1px;
  }
}

/* 细节配置卡 */
.config-group-card {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  padding: 10px;
  border-radius: 4px;
  .group-title {
    font-size: 10px;
    font-weight: 700;
    color: var(--jc-text-secondary);
    margin-bottom: 6px;
    text-transform: uppercase;
  }
}

/* 细节配置排 */
.flex-config-row {
  display: flex;
  gap: 10px;
  &.mt-10 {
    margin-top: 10px;
  }
  &.mt-8 {
    margin-top: 8px;
  }
}
.config-field {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  &.flex-2 {
    flex: 2;
  }
  label {
    font-size: 10px;
    color: var(--jc-text-secondary);
    font-weight: 600;
  }
}
.field-hint {
  font-size: 9px;
  color: var(--jc-color-warning);
  margin-top: 2px;
}
.color-picker-input-wrap {
  display: flex;
  align-items: center;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 3px;
  padding: 2px 4px;
  height: 26px;
  
  input[type="color"] {
    border: none;
    background: none;
    width: 20px;
    height: 18px;
    cursor: pointer;
    padding: 0;
  }
  .color-hex-text {
    flex: 1;
    border: none;
    background: none;
    color: var(--jc-text-primary);
    font-family: 'Cascadia Code', Consolas, monospace;
    font-size: 10px;
    padding-left: 6px;
    width: 100%;
    outline: none;
  }
}

/* 右侧预览 */
.display-pane {
  display: flex;
  flex-direction: column;
  flex: 0 0 310px;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 14px;
  border-radius: 4px;
  align-items: center;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  align-self: flex-start;
  margin-bottom: 12px;
  text-transform: uppercase;
}
.qr-canvas-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  padding: 12px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
  margin-bottom: 12px;
  width: 100%;
  max-height: 290px;
  overflow: hidden;
  transition: background-color 0.2s;
}
.qr-canvas {
  max-width: 100%;
  max-height: 100%;
}

/* ================= Tab 2: Parse ================= */
.upload-pane {
  display: flex;
  flex-direction: column;
  flex: 0 0 280px;
  gap: 12px;
}
.drop-zone {
  flex: 1;
  border: 2px dashed var(--jc-border-strong);
  border-radius: 6px;
  background: var(--jc-bg-panel);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  overflow: hidden;
  position: relative;
  transition: border-color 0.2s, background-color 0.2s;
  &:hover, &.dragging {
    border-color: var(--jc-color-accent);
    background: var(--jc-bg-hover);
  }
}
.file-input-hidden {
  position: absolute;
  width: 0;
  height: 0;
  opacity: 0;
  pointer-events: none;
}
.drop-msg {
  text-align: center;
  padding: 16px;
  color: var(--jc-text-secondary);
  svg {
    margin: 0 auto 10px auto;
    color: var(--jc-color-accent);
  }
  p {
    font-size: 12px;
    font-weight: 600;
    margin: 0 0 6px 0;
    color: var(--jc-text-primary);
  }
  .file-tip {
    font-size: 10px;
  }
}
.preview-img-container {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 10px;
  position: relative;
  background: var(--jc-bg-app);
  img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  }
}
.remove-upload-btn {
  position: absolute;
  top: 8px;
  right: 8px;
  background: rgba(0,0,0,0.6);
  color: #fff;
  border: none;
  border-radius: 50%;
  width: 20px;
  height: 20px;
  font-size: 10px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  &:hover {
    background: var(--jc-color-error);
  }
}

.results-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 14px;
  border-radius: 4px;
  min-height: 0;
}
.pane-label-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
  font-weight: 600;
}
.pane-acts {
  display: flex;
  gap: 6px;
}
.decode-result-box {
  flex: 1;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  padding: 12px;
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  overflow-y: auto;
  min-height: 0;
  display: flex;

  .result-text {
    color: var(--jc-color-success);
    white-space: pre-wrap;
    width: 100%;
  }
  .result-error-msg {
    color: var(--jc-color-error);
    white-space: pre-wrap;
    width: 100%;
  }
  .result-empty {
    color: var(--jc-text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    font-family: inherit;
    font-size: 11px;
  }

  &.error {
    border-color: rgba(244, 71, 71, 0.4);
    background: rgba(244, 71, 71, 0.05);
  }
}

.mt-8 { margin-top: 8px; }
.mt-10 { margin-top: 10px; }
</style>
