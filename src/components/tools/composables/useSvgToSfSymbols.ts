/**
 * SVG → Apple SF Symbols（.symbolset）转换器
 *
 * 把普通 <svg> 图标转换为 SF Symbols 模板：
 *  - 归一化到 24×24 viewBox（等比缩放 + 居中）
 *  - 所有图形统一为黑色填充（SF Symbols 靠模板着色，不含描边）
 *  - 生成 .symbolset 目录结构（Contents.json + <名称>.svg），可打包下载
 */

import {
  parsePathData,
  serializePath,
  applyMatrixToPath,
  parseTransform,
  shapeToPathData,
} from './useSvgToAndroidVector'

export interface SfSymbolsResult {
  /** 归一化后的 24×24 黑色模板 SVG */
  svg: string
  /** symbolset 内嵌 SVG 文件名（如 MySymbol.svg） */
  svgFileName: string
  /** symbolset 目录名（如 MySymbol.symbolset） */
  symbolName: string
  /** Contents.json 内容 */
  contentsJson: string
  warnings: string[]
  error?: string
}

const SKIP_TAGS = new Set([
  'defs', 'metadata', 'title', 'desc', 'style', 'script',
  'clippath', 'mask', 'pattern', 'marker', 'filter', 'symbol',
  'lineargradient', 'radialgradient', 'text', 'tspan', 'foreignobject',
  'use', 'image', 'switch', 'a',
])

/** 清洗 symbol 名称：仅保留字母数字与 -_，其余替换为 - */
function sanitizeName(name: string): string {
  const cleaned = (name || '')
    .trim()
    .replace(/[^\w.-]+/g, '-')
    .replace(/^-+|-+$/g, '')
  return cleaned || 'MySymbol'
}

export function convertSvgToSfSymbols(svgText: string, name = 'MySymbol'): SfSymbolsResult {
  const warnings: string[] = []
  const symbolName = sanitizeName(name)
  const svgFileName = `${symbolName}.svg`
  const text = (svgText || '').trim()
  if (!text) {
    return { svg: '', svgFileName, symbolName, contentsJson: '', warnings, error: '请输入 SVG 源码' }
  }

  let doc: Document
  try {
    doc = new DOMParser().parseFromString(text, 'image/svg+xml')
  } catch {
    return { svg: '', svgFileName, symbolName, contentsJson: '', warnings, error: 'SVG 解析失败' }
  }
  const parserError = doc.querySelector('parsererror')
  if (parserError) {
    return { svg: '', svgFileName, symbolName, contentsJson: '', warnings, error: parserError.textContent || 'SVG 语法格式不正确' }
  }

  const svg = doc.documentElement
  if (!svg || svg.localName.toLowerCase() !== 'svg') {
    return { svg: '', svgFileName, symbolName, contentsJson: '', warnings, error: '根节点必须是 <svg> 元素' }
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
      return { svg: '', svgFileName, symbolName, contentsJson: '', warnings, error: 'viewBox 格式不正确（应为 "x y width height"）' }
    }
  } else {
    const w = parseFloat(svg.getAttribute('width') || '')
    const h = parseFloat(svg.getAttribute('height') || '')
    if (!isNaN(w) && w > 0) vbw = w
    if (!isNaN(h) && h > 0) vbh = h
  }
  if (vbw <= 0 || vbh <= 0) {
    return { svg: '', svgFileName, symbolName, contentsJson: '', warnings, error: 'viewBox 宽高必须大于 0' }
  }

  // 归一化矩阵：等比缩放到 24×24 并居中
  const scale = 24 / Math.max(vbw, vbh)
  const tx = (24 - vbw * scale) / 2
  const ty = (24 - vbh * scale) / 2
  const baseMatrix = new DOMMatrix().translate(-vbx, -vby).translate(tx, ty).scale(scale)

  const paths: string[] = []

  function walk(elem: Element, matrix: DOMMatrix): void {
    for (const child of Array.from(elem.children)) {
      const el = child as Element
      const tag = el.localName.toLowerCase()
      const m = matrix.multiply(parseTransform(el.getAttribute('transform') || ''))
      if (tag === 'g' || tag === 'svg') {
        walk(el, m)
        continue
      }
      if (SKIP_TAGS.has(tag)) continue
      const d = shapeToPathData(el)
      if (!d) continue
      const cmds = parsePathData(d)
      if (cmds.length === 0) continue
      // SF Symbols 模板统一黑色实心，精度 2 位
      paths.push(m.isIdentity ? serializePath(cmds, 2) : applyMatrixToPath(cmds, m, 2))
    }
  }
  walk(svg, baseMatrix)

  if (paths.length === 0) {
    return { svg: '', svgFileName, symbolName, contentsJson: '', warnings, error: '没有可转换的图形元素' }
  }

  // —— 生成 24×24 黑色模板 SVG ——
  const svgOut = [
    '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">',
    ...paths.map((p) => `  <path fill="#000000" d="${p}"/>`),
    '</svg>',
  ].join('\n')

  const contentsJson = JSON.stringify(
    {
      info: { author: 'xcode', version: 1 },
      symbols: [{ filename: svgFileName, idiom: 'universal' }],
    },
    null,
    2,
  )

  return { svg: svgOut, svgFileName, symbolName, contentsJson, warnings }
}
