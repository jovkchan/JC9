/**
 * SVG → Android Vector Drawable（<vector> XML）转换器
 *
 * 将 <svg> 图标转换为 Android res/drawable 下的 vector 可绘制 XML。
 * 支持：
 *  - 图形元素：path / rect / circle / ellipse / line / polyline / polygon
 *  - 属性：fill / fill-opacity / fill-rule / stroke 系列 / opacity / transform
 *  - 分组继承（<g> 内属性向下传递）、transform 矩阵变换（translate/scale/rotate/skew/matrix）
 *  - pathData 精度规整（Android Studio 风格：逗号分隔坐标、去掉多余小数位）
 *  - 命名色 / rgb() / rgba() → Android 可用十六进制
 *
 * 局限（会以 warnings 提示）：渐变、<text>、<use>、裁剪/遮罩不支持，需手动处理。
 */

export interface SvgToVectorOptions {
  /** 输出宽度 (dp) */
  widthDp: number
  /** 输出高度 (dp) */
  heightDp: number
  /** pathData / strokeWidth 等数值保留的小数位 */
  precision: number
}

export interface SvgToVectorResult {
  xml: string
  warnings: string[]
  error?: string
}

/** 一个 path 命令：字母 + 参数 */
export interface PathCmd {
  cmd: string
  args: number[]
}

const CMD_ARITY: Record<string, number> = {
  M: 2, m: 2, L: 2, l: 2, H: 1, h: 1, V: 1, v: 1,
  C: 6, c: 6, S: 4, s: 4, Q: 4, q: 4, T: 2, t: 2,
  A: 7, a: 7, Z: 0, z: 0,
}

/** 拆分 SVG path d 为命令字母与数字 token（兼容科学计数法） */
function tokenizePath(d: string): Array<string | number> {
  const tokens: Array<string | number> = []
  const re = /[a-zA-Z]|-?\d*\.?\d+(?:[eE][+-]?\d+)?/g
  let m: RegExpExecArray | null
  while ((m = re.exec(d)) !== null) {
    const t = m[0]
    tokens.push(/[a-zA-Z]/.test(t) ? t : parseFloat(t))
  }
  return tokens
}

/**
 * 将 token 按命令分组（处理隐式重复参数；
 * M 之后的隐式坐标对视为 L / l）。
 */
export function parsePathData(d: string): PathCmd[] {
  const tokens = tokenizePath(d)
  const cmds: PathCmd[] = []
  let idx = 0
  let prev = ''
  while (idx < tokens.length) {
    let explicit = false
    let letter = ''
    if (typeof tokens[idx] === 'string') {
      letter = tokens[idx] as string
      idx++
      explicit = true
    } else if (prev) {
      letter = prev
    } else {
      break
    }
    let eff = letter
    if (/[Mm]/.test(letter) && !explicit && /[Mm]/.test(prev)) {
      eff = letter === 'M' ? 'L' : 'l'
    }
    const n = CMD_ARITY[eff] ?? 0
    const args: number[] = []
    for (let k = 0; k < n && idx < tokens.length && typeof tokens[idx] === 'number'; k++) {
      args.push(tokens[idx] as number)
      idx++
    }
    cmds.push({ cmd: eff, args })
    prev = eff
  }
  return cmds
}

/** 数字精度规整（去尾零，如 1.50 → 1.5） */
export function roundNum(v: number, precision: number): string {
  const p = Math.max(0, Math.min(6, Math.round(precision)))
  return String(Number(v.toFixed(p)))
}

/**
 * 按 Android Studio 风格序列化 path：
 * 命令字母 + 逗号分隔参数，命令之间不加空格。
 * 例：M251.74,251.74h512v512h-512z
 */
export function serializePath(cmds: PathCmd[], precision: number): string {
  return cmds
    .map((c) => c.cmd + c.args.map((a) => roundNum(a, precision)).join(','))
    .join('')
}

/** 解析 SVG transform 字符串为 DOMMatrix（顺序：右→左，先应用的在后） */
export function parseTransform(transform: string | null | undefined): DOMMatrix {
  const m = new DOMMatrix()
  if (!transform) return m
  const re = /([a-zA-Z]+)\s*\(([^)]*)\)/g
  let match: RegExpExecArray | null
  while ((match = re.exec(transform)) !== null) {
    const name = match[1]
    const args = match[2].trim().split(/[\s,]+/).filter(Boolean).map(Number)
    if (name === 'matrix' && args.length === 6) {
      m.multiplySelf(new DOMMatrix([args[0], args[1], args[2], args[3], args[4], args[5]]))
    } else if (name === 'translate') {
      m.translateSelf(args[0] || 0, args.length > 1 ? args[1] : 0)
    } else if (name === 'scale') {
      const sx = args[0] || 1
      m.scaleSelf(sx, args.length > 1 ? args[1] : sx)
    } else if (name === 'rotate') {
      const deg = args[0] || 0
      if (args.length >= 3) {
        m.translateSelf(args[1], args[2]).rotateSelf(0, 0, deg).translateSelf(-args[1], -args[2])
      } else {
        m.rotateSelf(0, 0, deg)
      }
    } else if (name === 'skewX') {
      m.skewXSelf(args[0] || 0)
    } else if (name === 'skewY') {
      m.skewYSelf(args[0] || 0)
    }
  }
  return m
}

const isRelCmd = (cmd: string) => cmd === cmd.toLowerCase()

/**
 * 对 path 命令序列应用矩阵变换，输出为绝对坐标的 M/L/C/A/Z 命令串。
 * （H/V → L，S → C，Q/T → C；A 弧线按矩阵等比例缩放近似处理）
 */
export function applyMatrixToPath(cmds: PathCmd[], m: DOMMatrix, precision: number): string {
  const out: PathCmd[] = []
  let cx = 0
  let cy = 0
  let sx = 0
  let sy = 0
  let last = ''
  let lcp: [number, number] | null = null // 上一次三次曲线第二控制点
  let lqp: [number, number] | null = null // 上一次二次曲线控制点

  const t = (x: number, y: number): [number, number] => {
    const p = new DOMPoint(x, y).matrixTransform(m)
    return [p.x, p.y]
  }

  for (const c of cmds) {
    const A = c.args
    const up = c.cmd.toUpperCase()
    const isRel = isRelCmd(c.cmd)
    if (up === 'Z') {
      out.push({ cmd: 'Z', args: [] })
      cx = sx
      cy = sy
      last = 'Z'
    } else if (up === 'M') {
      const x = isRel ? cx + A[0] : A[0]
      const y = isRel ? cy + A[1] : A[1]
      const [tx, ty] = t(x, y)
      out.push({ cmd: 'M', args: [tx, ty] })
      sx = x
      sy = y
      cx = x
      cy = y
      last = 'M'
    } else if (up === 'L') {
      const x = isRel ? cx + A[0] : A[0]
      const y = isRel ? cy + A[1] : A[1]
      const [tx, ty] = t(x, y)
      out.push({ cmd: 'L', args: [tx, ty] })
      cx = x
      cy = y
      last = 'L'
    } else if (up === 'H') {
      const x = isRel ? cx + A[0] : A[0]
      const [tx, ty] = t(x, cy)
      out.push({ cmd: 'L', args: [tx, ty] })
      cx = x
      last = 'H'
    } else if (up === 'V') {
      const y = isRel ? cy + A[0] : A[0]
      const [tx, ty] = t(cx, y)
      out.push({ cmd: 'L', args: [tx, ty] })
      cy = y
      last = 'V'
    } else if (up === 'C') {
      const x1 = isRel ? cx + A[0] : A[0]
      const y1 = isRel ? cy + A[1] : A[1]
      const x2 = isRel ? cx + A[2] : A[2]
      const y2 = isRel ? cy + A[3] : A[3]
      const x = isRel ? cx + A[4] : A[4]
      const y = isRel ? cy + A[5] : A[5]
      const [tx1, ty1] = t(x1, y1)
      const [tx2, ty2] = t(x2, y2)
      const [tx, ty] = t(x, y)
      out.push({ cmd: 'C', args: [tx1, ty1, tx2, ty2, tx, ty] })
      cx = x
      cy = y
      lcp = [x2, y2]
      last = 'C'
    } else if (up === 'S') {
      const x2 = isRel ? cx + A[0] : A[0]
      const y2 = isRel ? cy + A[1] : A[1]
      const x = isRel ? cx + A[2] : A[2]
      const y = isRel ? cy + A[3] : A[3]
      let x1: number
      let y1: number
      if (last === 'C' || last === 'S') {
        x1 = 2 * cx - (lcp ? lcp[0] : cx)
        y1 = 2 * cy - (lcp ? lcp[1] : cy)
      } else {
        x1 = cx
        y1 = cy
      }
      const [tx1, ty1] = t(x1, y1)
      const [tx2, ty2] = t(x2, y2)
      const [tx, ty] = t(x, y)
      out.push({ cmd: 'C', args: [tx1, ty1, tx2, ty2, tx, ty] })
      cx = x
      cy = y
      lcp = [x2, y2]
      last = 'S'
    } else if (up === 'Q') {
      const x1 = isRel ? cx + A[0] : A[0]
      const y1 = isRel ? cy + A[1] : A[1]
      const x = isRel ? cx + A[2] : A[2]
      const y = isRel ? cy + A[3] : A[3]
      // 二次贝塞尔 → 三次贝塞尔
      const c1x = cx + (2 / 3) * (x1 - cx)
      const c1y = cy + (2 / 3) * (y1 - cy)
      const c2x = x1 + (1 / 3) * (x - x1)
      const c2y = y1 + (1 / 3) * (y - y1)
      const [tc1x, tc1y] = t(c1x, c1y)
      const [tc2x, tc2y] = t(c2x, c2y)
      const [tx, ty] = t(x, y)
      out.push({ cmd: 'C', args: [tc1x, tc1y, tc2x, tc2y, tx, ty] })
      cx = x
      cy = y
      lcp = [c2x, c2y]
      lqp = [x1, y1]
      last = 'Q'
    } else if (up === 'T') {
      let x1: number
      let y1: number
      if (last === 'Q' || last === 'T') {
        x1 = 2 * cx - (lqp ? lqp[0] : cx)
        y1 = 2 * cy - (lqp ? lqp[1] : cy)
      } else {
        x1 = cx
        y1 = cy
      }
      const x = isRel ? cx + A[0] : A[0]
      const y = isRel ? cy + A[1] : A[1]
      const c1x = cx + (2 / 3) * (x1 - cx)
      const c1y = cy + (2 / 3) * (y1 - cy)
      const c2x = x1 + (1 / 3) * (x - x1)
      const c2y = y1 + (1 / 3) * (y - y1)
      const [tc1x, tc1y] = t(c1x, c1y)
      const [tc2x, tc2y] = t(c2x, c2y)
      const [tx, ty] = t(x, y)
      out.push({ cmd: 'C', args: [tc1x, tc1y, tc2x, tc2y, tx, ty] })
      cx = x
      cy = y
      lcp = [c2x, c2y]
      lqp = [x1, y1]
      last = 'T'
    } else if (up === 'A') {
      const rx = Math.abs(A[0])
      const ry = Math.abs(A[1])
      const rot = A[2]
      const laf = A[3]
      const sf = A[4]
      const x = isRel ? cx + A[5] : A[5]
      const y = isRel ? cy + A[6] : A[6]
      const [tx, ty] = t(x, y)
      // 弧线半径按矩阵等比例缩放近似（纯平移/缩放/旋转精确，斜切近似）
      const scale = Math.sqrt(Math.abs(m.a * m.d - m.b * m.c))
      out.push({ cmd: 'A', args: [rx * scale, ry * scale, rot, laf, sf, tx, ty] })
      cx = x
      cy = y
      last = 'A'
    }
  }
  return serializePath(out, precision)
}

/** 基础图形元素 → path 数据串 */
export function shapeToPathData(elem: Element): string | null {
  const tag = elem.localName.toLowerCase()
  const get = (name: string) => elem.getAttribute(name)
  const num = (name: string) => {
    const v = get(name)
    return v ? parseFloat(v) : 0
  }
  switch (tag) {
    case 'path': {
      const d = get('d')
      return d && d.trim() ? d : null
    }
    case 'rect': {
      const x = num('x')
      const y = num('y')
      const w = num('width')
      const h = num('height')
      let rx = num('rx')
      let ry = num('ry')
      if (!get('rx') && get('ry')) rx = ry
      if (!get('ry') && get('rx')) ry = rx
      rx = Math.max(0, Math.min(rx || 0, w / 2))
      ry = Math.max(0, Math.min(ry || 0, h / 2))
      if (rx <= 0 && ry <= 0) {
        return `M${x} ${y}H${x + w}V${y + h}H${x}Z`
      }
      return `M${x + rx} ${y}H${x + w - rx}A${rx} ${ry} 0 0 1 ${x + w} ${y + ry}V${y + h - ry}A${rx} ${ry} 0 0 1 ${x + w - rx} ${y + h}H${x + rx}A${rx} ${ry} 0 0 1 ${x} ${y + h - ry}V${y + ry}A${rx} ${ry} 0 0 1 ${x + rx} ${y}Z`
    }
    case 'circle': {
      const cx = num('cx')
      const cy = num('cy')
      const r = num('r')
      if (!r) return null
      return `M${cx - r} ${cy}A${r} ${r} 0 1 0 ${cx + r} ${cy}A${r} ${r} 0 1 0 ${cx - r} ${cy}Z`
    }
    case 'ellipse': {
      const cx = num('cx')
      const cy = num('cy')
      const rx = num('rx')
      const ry = num('ry')
      if (!rx || !ry) return null
      return `M${cx - rx} ${cy}A${rx} ${ry} 0 1 0 ${cx + rx} ${cy}A${rx} ${ry} 0 1 0 ${cx - rx} ${cy}Z`
    }
    case 'line':
      return `M${num('x1')} ${num('y1')}L${num('x2')} ${num('y2')}`
    case 'polyline':
    case 'polygon': {
      const pts = (get('points') || '').trim().split(/[\s,]+/).filter(Boolean).map(Number)
      if (pts.length < 4) return null
      let d = ''
      for (let i = 0; i + 1 < pts.length; i += 2) {
        d += (i === 0 ? 'M' : 'L') + pts[i] + ' ' + pts[i + 1]
      }
      return tag === 'polygon' ? d + 'Z' : d
    }
    default:
      return null
  }
}

/** 解析内联 style="k:v;k:v" */
function parseStyle(style: string): Record<string, string> {
  const out: Record<string, string> = {}
  for (const part of style.split(';')) {
    const idx = part.indexOf(':')
    if (idx > 0) {
      const k = part.slice(0, idx).trim().toLowerCase()
      const v = part.slice(idx + 1).trim()
      if (k) out[k] = v
    }
  }
  return out
}

const PRESENTATION_KEYS = [
  'fill',
  'fill-opacity',
  'fill-rule',
  'stroke',
  'stroke-width',
  'stroke-opacity',
  'stroke-linecap',
  'stroke-linejoin',
  'stroke-miterlimit',
  'opacity',
  'transform',
] as const

/** 解析元素呈现属性（继承 < 元素属性 < 内联 style） */
function resolveAttrs(elem: Element, inherited: Record<string, string>): Record<string, string> {
  const attrs: Record<string, string> = { ...inherited }
  for (const k of PRESENTATION_KEYS) {
    const v = elem.getAttribute(k)
    if (v !== null && v !== '') attrs[k] = v
  }
  const style = parseStyle(elem.getAttribute('style') || '')
  for (const k of PRESENTATION_KEYS) {
    if (style[k] !== undefined && style[k] !== '') attrs[k] = style[k]
  }
  return attrs
}

// CSS 命名色表
const NAMED_COLORS: Record<string, string> = {
  aliceblue: '#f0f8ff', antiquewhite: '#faebd7', aqua: '#00ffff', aquamarine: '#7fffd4',
  azure: '#f0ffff', beige: '#f5f5dc', bisque: '#ffe4c4', black: '#000000',
  blanchedalmond: '#ffebcd', blue: '#0000ff', blueviolet: '#8a2be2', brown: '#a52a2a',
  burlywood: '#deb887', cadetblue: '#5f9ea0', chartreuse: '#7fff00', chocolate: '#d2691e',
  coral: '#ff7f50', cornflowerblue: '#6495ed', cornsilk: '#fff8dc', crimson: '#dc143c',
  cyan: '#00ffff', darkblue: '#00008b', darkcyan: '#008b8b', darkgoldenrod: '#b8860b',
  darkgray: '#a9a9a9', darkgreen: '#006400', darkgrey: '#a9a9a9', darkkhaki: '#bdb76b',
  darkmagenta: '#8b008b', darkolivegreen: '#556b2f', darkorange: '#ff8c00', darkorchid: '#9932cc',
  darkred: '#8b0000', darksalmon: '#e9967a', darkseagreen: '#8fbc8f', darkslateblue: '#483d8b',
  darkslategray: '#2f4f4f', darkslategrey: '#2f4f4f', darkturquoise: '#00ced1', darkviolet: '#9400d3',
  deeppink: '#ff1493', deepskyblue: '#00bfff', dimgray: '#696969', dimgrey: '#696969',
  dodgerblue: '#1e90ff', firebrick: '#b22222', floralwhite: '#fffaf0', forestgreen: '#228b22',
  fuchsia: '#ff00ff', gainsboro: '#dcdcdc', ghostwhite: '#f8f8ff', gold: '#ffd700',
  goldenrod: '#daa520', gray: '#808080', green: '#008000', greenyellow: '#adff2f',
  grey: '#808080', honeydew: '#f0fff0', hotpink: '#ff69b4', indianred: '#cd5c5c',
  indigo: '#4b0082', ivory: '#fffff0', khaki: '#f0e68c', lavender: '#e6e6fa',
  lavenderblush: '#fff0f5', lawngreen: '#7cfc00', lemonchiffon: '#fffacd', lightblue: '#add8e6',
  lightcoral: '#f08080', lightcyan: '#e0ffff', lightgoldenrodyellow: '#fafad2', lightgray: '#d3d3d3',
  lightgreen: '#90ee90', lightgrey: '#d3d3d3', lightpink: '#ffb6c1', lightsalmon: '#ffa07a',
  lightseagreen: '#20b2aa', lightskyblue: '#87cefa', lightslategray: '#778899', lightslategrey: '#778899',
  lightsteelblue: '#b0c4de', lightyellow: '#ffffe0', lime: '#00ff00', limegreen: '#32cd32',
  linen: '#faf0e6', magenta: '#ff00ff', maroon: '#800000', mediumaquamarine: '#66cdaa',
  mediumblue: '#0000cd', mediumorchid: '#ba55d3', mediumpurple: '#9370db', mediumseagreen: '#3cb371',
  mediumslateblue: '#7b68ee', mediumspringgreen: '#00fa9a', mediumturquoise: '#48d1cc', mediumvioletred: '#c71585',
  midnightblue: '#191970', mintcream: '#f5fffa', mistyrose: '#ffe4e1', moccasin: '#ffe4b5',
  navajowhite: '#ffdead', navy: '#000080', oldlace: '#fdf5e6', olive: '#808000',
  olivedrab: '#6b8e23', orange: '#ffa500', orangered: '#ff4500', orchid: '#da70d6',
  palegoldenrod: '#eee8aa', palegreen: '#98fb98', paleturquoise: '#afeeee', palevioletred: '#db7093',
  papayawhip: '#ffefd5', peachpuff: '#ffdab9', peru: '#cd853f', pink: '#ffc0cb',
  plum: '#dda0dd', powderblue: '#b0e0e6', purple: '#800080', rebeccapurple: '#663399',
  red: '#ff0000', rosybrown: '#bc8f8f', royalblue: '#4169e1', saddlebrown: '#8b4513',
  salmon: '#fa8072', sandybrown: '#f4a460', seagreen: '#2e8b57', seashell: '#fff5ee',
  sienna: '#a0522d', silver: '#c0c0c0', skyblue: '#87ceeb', slateblue: '#6a5acd',
  slategray: '#708090', slategrey: '#708090', snow: '#fffafa', springgreen: '#00ff7f',
  steelblue: '#4682b4', tan: '#d2b48c', teal: '#008080', thistle: '#d8bfd8',
  tomato: '#ff6347', turquoise: '#40e0d0', violet: '#ee82ee', wheat: '#f5deb3',
  white: '#ffffff', whitesmoke: '#f5f5f5', yellow: '#ffff00', yellowgreen: '#9acd32',
}

function toHex(n: number): string {
  return Math.round(n).toString(16).padStart(2, '0')
}

/** 颜色归一化：支持 #hex / rgb() / rgba() / 命名色 → Android 可用十六进制 */
export function normalizeColor(value: string, warnings: string[]): string {
  const trimmed = value.trim()
  const v = trimmed.toLowerCase()
  if (v.startsWith('#')) return trimmed // 保留原始大小写
  if (v === 'currentcolor') {
    warnings.push('检测到 currentColor，已原样保留（Android 需替换为具体颜色或 @color 资源）')
    return value.trim()
  }
  if (v.startsWith('rgb')) {
    const m = v.match(/rgba?\(([^)]+)\)/)
    if (m) {
      const parts = m[1].split(/[\s,]+/).filter(Boolean).map(Number)
      const r = Math.max(0, Math.min(255, parts[0] || 0))
      const g = Math.max(0, Math.min(255, parts[1] || 0))
      const b = Math.max(0, Math.min(255, parts[2] || 0))
      if (parts.length >= 4) {
        const a = Math.max(0, Math.min(1, parts[3]))
        return '#' + toHex(a * 255) + toHex(r) + toHex(g) + toHex(b)
      }
      return '#' + toHex(r) + toHex(g) + toHex(b)
    }
  }
  if (NAMED_COLORS[v]) return NAMED_COLORS[v]
  warnings.push(`无法识别的颜色「${value.trim()}」，已原样保留（Android 可能无法编译）`)
  return value.trim()
}

const SKIP_TAGS = new Set([
  'defs', 'metadata', 'title', 'desc', 'style', 'script',
  'clippath', 'mask', 'pattern', 'marker', 'filter', 'symbol',
  'lineargradient', 'radialgradient', 'text', 'tspan', 'foreignobject',
  'use', 'image', 'switch', 'a',
])

export function convertSvgToAndroidVector(svgText: string, opts: SvgToVectorOptions): SvgToVectorResult {
  const warnings: string[] = []
  const text = (svgText || '').trim()
  if (!text) return { xml: '', warnings, error: '请输入 SVG 源码' }

  let doc: Document
  try {
    doc = new DOMParser().parseFromString(text, 'image/svg+xml')
  } catch {
    return { xml: '', warnings, error: 'SVG 解析失败' }
  }
  const parserError = doc.querySelector('parsererror')
  if (parserError) return { xml: '', warnings, error: parserError.textContent || 'SVG 语法格式不正确' }

  const svg = doc.documentElement
  if (!svg || svg.localName.toLowerCase() !== 'svg') {
    return { xml: '', warnings, error: '根节点必须是 <svg> 元素' }
  }

  // —— 视口 ——
  let vbx = 0
  let vby = 0
  let vbw = 24
  let vbh = 24
  const vb = svg.getAttribute('viewBox')
  if (vb && vb.trim()) {
    const parts = vb.trim().split(/[\s,]+/).map(Number)
    if (parts.length === 4 && parts.every((n) => !isNaN(n))) {
      vbx = parts[0]
      vby = parts[1]
      vbw = parts[2]
      vbh = parts[3]
    } else {
      return { xml: '', warnings, error: 'viewBox 格式不正确（应为 "x y width height"）' }
    }
  } else {
    const w = parseFloat(svg.getAttribute('width') || '')
    const h = parseFloat(svg.getAttribute('height') || '')
    if (!isNaN(w) && w > 0) vbw = w
    if (!isNaN(h) && h > 0) vbh = h
  }
  if (vbw <= 0 || vbh <= 0) return { xml: '', warnings, error: 'viewBox 宽高必须大于 0' }

  const precision = Math.max(0, Math.min(6, Math.round(opts.precision || 2)))
  const widthDp = Math.max(1, Math.round(opts.widthDp || 24))
  const heightDp = Math.max(1, Math.round(opts.heightDp || 24))

  const paths: Array<{ attrs: Record<string, string>; d: string }> = []

  // 根矩阵：viewBox 原点偏移 + svg 自身 transform
  const rootMatrix = new DOMMatrix().translate(-vbx, -vby)
  rootMatrix.multiplySelf(parseTransform(svg.getAttribute('transform')))

  function walk(elem: Element, inherited: Record<string, string>, opacityMul: number, matrix: DOMMatrix): void {
    const attrs = resolveAttrs(elem, inherited)
    const tag = elem.localName.toLowerCase()

    if (SKIP_TAGS.has(tag)) {
      if (tag === 'text' || tag === 'tspan') warnings.push('包含文本元素 <text>，Android VectorDrawable 不支持文本，已忽略')
      else if (tag === 'use') warnings.push('包含 <use> 引用元素，已忽略（请手动合并其引用的路径）')
      else if (tag === 'clippath' || tag === 'mask') warnings.push(`包含 <${tag}> 裁剪/遮罩元素，已忽略`)
      else if (tag === 'lineargradient' || tag === 'radialgradient') warnings.push('包含渐变定义，填充色需改用 <aapt:attr> 实现（本工具仅支持纯色）')
      return
    }

    // 累乘 opacity（group 向下传递，元素自身也计入）
    let opMul = opacityMul
    const op = parseFloat(attrs.opacity)
    if (!isNaN(op)) opMul *= op

    if (tag === 'g') {
      const childMatrix = matrix.multiply(parseTransform(attrs.transform || ''))
      // transform 只通过矩阵参数向下传递，不再作为呈现属性继承（避免重复应用）
      const childInherited = { ...attrs }
      delete childInherited.transform
      for (const child of Array.from(elem.children)) {
        walk(child as Element, childInherited, opMul, childMatrix)
      }
      return
    }

    const d = shapeToPathData(elem)
    if (!d) return

    const m = matrix.multiply(parseTransform(attrs.transform || ''))
    const pathAttrs: Record<string, string> = {}

    // —— 填充 ——
    const hasFill = 'fill' in attrs
    const fill = hasFill ? attrs.fill : undefined
    const fillIsUrl = !!fill && fill.startsWith('url(')
    const fillIsNone = hasFill && (fill === 'none' || fill === 'transparent')
    if (fillIsUrl) {
      warnings.push(`路径检测到渐变/引用填充 ${fill}，已跳过填充色（请手动补充 <aapt:attr> 渐变）`)
    } else if (!fillIsNone) {
      // 未显式设置 fill 时 SVG 默认黑色
      pathAttrs['fillColor'] = fill ? normalizeColor(fill, warnings) : '#000000'
      let fillAlpha = opMul
      const fo = parseFloat(attrs['fill-opacity'])
      if (!isNaN(fo)) fillAlpha *= fo
      if (fillAlpha < 1 - 1e-6) pathAttrs['fillAlpha'] = roundNum(fillAlpha, precision)
      const fr = (attrs['fill-rule'] || '').trim()
      if (fr === 'evenodd') pathAttrs['fillType'] = 'evenOdd'
    }

    // —— 描边 ——
    const stroke = attrs.stroke
    const strokeIsUrl = !!stroke && stroke.startsWith('url(')
    if (stroke && stroke !== 'none' && stroke !== 'transparent' && !strokeIsUrl) {
      pathAttrs['strokeColor'] = normalizeColor(stroke, warnings)
      const sw = parseFloat(attrs['stroke-width'])
      if (!isNaN(sw) && sw > 0) pathAttrs['strokeWidth'] = roundNum(sw, precision)
      let strokeAlpha = opMul
      const so = parseFloat(attrs['stroke-opacity'])
      if (!isNaN(so)) strokeAlpha *= so
      if (strokeAlpha < 1 - 1e-6) pathAttrs['strokeAlpha'] = roundNum(strokeAlpha, precision)
      const cap = (attrs['stroke-linecap'] || '').trim()
      if (cap && cap !== 'butt') pathAttrs['strokeLineCap'] = cap
      const join = (attrs['stroke-linejoin'] || '').trim()
      if (join && join !== 'miter') pathAttrs['strokeLineJoin'] = join
      const ml = parseFloat(attrs['stroke-miterlimit'])
      if (!isNaN(ml) && ml !== 4) pathAttrs['strokeMiterLimit'] = roundNum(ml, precision)
    } else if (strokeIsUrl) {
      warnings.push(`路径检测到渐变描边 ${stroke}，已跳过描边色`)
    }

    // —— path 数据（无变换保留原始命令风格；有变换转绝对坐标） ——
    const cmds = parsePathData(d)
    if (cmds.length === 0) return
    const pathData = m.isIdentity ? serializePath(cmds, precision) : applyMatrixToPath(cmds, m, precision)

    paths.push({ attrs: pathAttrs, d: pathData })
  }

  const children = Array.from(svg.children)
  if (children.length === 0) warnings.push('SVG 中没有可转换的图形元素')
  for (const child of children) {
    walk(child as Element, {}, 1, rootMatrix)
  }

  if (paths.length === 0) {
    return { xml: '', warnings, error: '没有可转换的图形元素（支持 path/rect/circle/ellipse/line/polyline/polygon）' }
  }

  // —— 生成 XML ——
  const lines: string[] = []
  lines.push('<vector xmlns:android="http://schemas.android.com/apk/res/android"')
  lines.push(`    android:width="${widthDp}dp"`)
  lines.push(`    android:height="${heightDp}dp"`)
  lines.push(`    android:viewportWidth="${roundNum(vbw, 2)}"`)
  lines.push(`    android:viewportHeight="${roundNum(vbh, 2)}">`)
  for (const p of paths) {
    lines.push('  <path')
    for (const [k, v] of Object.entries(p.attrs)) {
      lines.push(`      android:${k}="${v}"`)
    }
    lines.push(`      android:pathData="${p.d}"/>`)
  }
  lines.push('</vector>')

  return { xml: lines.join('\n'), warnings }
}
