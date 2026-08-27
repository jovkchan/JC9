/**
 * Android Vector XML / SF Symbols SVG → 标准 SVG 转换器
 *
 * 输入检测：
 *  - 根节点 <vector ...>（Android VectorDrawable）→ 转标准 <svg>
 *  - 根节点 <svg ...>（含 SF Symbols 的 24×24 模板）→ 规范化输出标准 SVG
 */

import { roundNum } from './useSvgToAndroidVector'

export interface VectorXmlToSvgResult {
  svg: string
  /** 检测到的输入类型 */
  kind: 'vector' | 'svg' | 'unknown'
  warnings: string[]
  error?: string
}

const FAIL: VectorXmlToSvgResult = { svg: '', warnings: [], kind: 'unknown' }

/** Android vector group 的 transform 属性 → SVG transform 字符串 */
function groupTransform(el: Element): string {
  const get = (n: string) => parseFloat(el.getAttribute(n) || '')
  const tx = get('android:translateX')
  const ty = get('android:translateY')
  const sx = get('android:scaleX')
  const sy = get('android:scaleY')
  const rot = get('android:rotate')
  const px = get('android:pivotX')
  const py = get('android:pivotY')
  const parts: string[] = []
  const hasPivot = !isNaN(px) && !isNaN(py) && (px !== 0 || py !== 0)
  const hasRot = !isNaN(rot) && rot !== 0
  if (hasPivot && (hasRot || (!isNaN(sx) && sx !== 1) || (!isNaN(sy) && sy !== 1))) {
    parts.push(`translate(${roundNum(px, 2)} ${roundNum(py, 2)})`)
  }
  if (!isNaN(tx) && tx !== 0) parts.push(`translate(${roundNum(tx, 2)} 0)`)
  if (!isNaN(ty) && ty !== 0) parts.push(`translate(0 ${roundNum(ty, 2)})`)
  if (hasRot) parts.push(`rotate(${roundNum(rot, 2)}${hasPivot ? ` ${roundNum(px, 2)} ${roundNum(py, 2)}` : ''})`)
  if (!isNaN(sx) && sx !== 1) parts.push(`scale(${roundNum(sx, 2)} 1)`)
  if (!isNaN(sy) && sy !== 1) parts.push(`scale(1 ${roundNum(sy, 2)})`)
  return parts.join(' ')
}

/** Android vector <path> → SVG 属性 */
function vectorPathAttrs(el: Element): Record<string, string> {
  const attrs: Record<string, string> = {}
  const get = (n: string) => el.getAttribute(n)
  const d = get('android:pathData')
  if (d) attrs['d'] = d

  // 填充
  const fill = get('android:fillColor')
  const fillAlpha = get('android:fillAlpha')
  if (fill && fill !== 'none' && fill !== 'transparent') {
    attrs['fill'] = fill
    if (fillAlpha && fillAlpha !== '1') attrs['fill-opacity'] = fillAlpha
  } else {
    attrs['fill'] = 'none' // Android 默认无填充，SVG 默认黑需显式 none
  }
  const fillType = get('android:fillType')
  if (fillType === 'evenOdd') attrs['fill-rule'] = 'evenodd'
  else if (fillType === 'nonZero') attrs['fill-rule'] = 'nonzero'

  // 描边
  const stroke = get('android:strokeColor')
  const strokeAlpha = get('android:strokeAlpha')
  if (stroke && stroke !== 'none' && stroke !== 'transparent') {
    attrs['stroke'] = stroke
    if (strokeAlpha && strokeAlpha !== '1') attrs['stroke-opacity'] = strokeAlpha
  }
  const sw = get('android:strokeWidth')
  if (sw) attrs['stroke-width'] = sw
  const cap = get('android:strokeLineCap')
  if (cap) attrs['stroke-linecap'] = cap
  const join = get('android:strokeLineJoin')
  if (join) attrs['stroke-linejoin'] = join
  const ml = get('android:strokeMiterLimit')
  if (ml) attrs['stroke-miterlimit'] = ml

  return attrs
}

/** 递归把 Android vector 节点 → SVG 字符串行 */
function vectorNodeToSvgLines(el: Element, warnings: string[]): string[] {
  const lines: string[] = []
  for (const child of Array.from(el.children)) {
    const c = child as Element
    const tag = c.localName.toLowerCase()
    if (tag === 'group') {
      const tr = groupTransform(c)
      if (tr) {
        lines.push(`  <g transform="${tr}">`)
        lines.push(...vectorNodeToSvgLines(c, warnings).map((l) => l === '  <g ' || l.trim() === '</g>' ? l : `  ${l}`))
        lines.push(`  </g>`)
      } else {
        lines.push(...vectorNodeToSvgLines(c, warnings))
      }
    } else if (tag === 'path') {
      const attrs = vectorPathAttrs(c)
      if (!attrs['d']) continue
      const attrStr = Object.entries(attrs)
        .map(([k, v]) => `${k}="${v}"`)
        .join(' ')
      lines.push(`  <path ${attrStr}/>`)
    } else if (tag === 'clip-path' || tag === 'clipPath') {
      warnings.push('包含 <clip-path> 裁剪，SVG 输出已忽略（请手动处理）')
    }
  }
  return lines
}

function convertVectorToSvg(root: Element, warnings: string[]): VectorXmlToSvgResult {
  const get = (n: string) => root.getAttribute(n)
  const vw = parseFloat(get('android:viewportWidth') || '24')
  const vh = parseFloat(get('android:viewportHeight') || '24')
  const width = parseFloat(get('android:width') || '') // "24dp" → 24
  const height = parseFloat(get('android:height') || '')

  const lines: string[] = []
  const vb = `0 0 ${isNaN(vw) ? 24 : vw} ${isNaN(vh) ? 24 : vh}`
  lines.push(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}"`)
  if (!isNaN(width) && width > 0) lines.push(`  width="${width}"`)
  if (!isNaN(height) && height > 0) lines.push(`  height="${height}"`)
  lines.push(`>`)
  lines.push(...vectorNodeToSvgLines(root, warnings))
  lines.push('</svg>')
  return { svg: lines.join('\n'), kind: 'vector', warnings }
}

/** 规范化已有 SVG（补 xmlns / viewBox / 尺寸），用于 SF Symbols 模板等输入 */
function normalizeSvg(root: Element): string {
  const lines: string[] = []
  const vb = root.getAttribute('viewBox')
  const w = root.getAttribute('width')
  const h = root.getAttribute('height')
  const header: string[] = []
  header.push('<svg xmlns="http://www.w3.org/2000/svg"')
  if (vb) header.push(` viewBox="${vb}"`)
  if (w) header.push(` width="${w}"`)
  if (h) header.push(` height="${h}"`)
  header.push('>')
  lines.push(header.join(''))

  // 提取根内所有子元素序列化
  for (const child of Array.from(root.childNodes)) {
    if (child.nodeType === Node.COMMENT_NODE) continue
    if (child.nodeType === Node.ELEMENT_NODE) {
      lines.push(new XMLSerializer().serializeToString(child).replace(/^<\?xml[^>]*\?>/i, ''))
    }
  }
  lines.push('</svg>')
  return lines.join('\n')
}

export function convertToSvg(text: string): VectorXmlToSvgResult {
  const warnings: string[] = []
  const t = (text || '').trim()
  if (!t) return { ...FAIL, error: '请输入 SVG / Android Vector XML' }

  let doc: Document
  try {
    doc = new DOMParser().parseFromString(t, 'image/svg+xml')
  } catch {
    return { ...FAIL, error: 'XML 解析失败' }
  }
  const parserError = doc.querySelector('parsererror')
  if (parserError) return { ...FAIL, error: parserError.textContent || 'XML 语法不正确' }

  const root = doc.documentElement
  if (!root) return { ...FAIL, error: 'XML 内容为空' }
  const tag = root.localName.toLowerCase()

  if (tag === 'vector') return convertVectorToSvg(root, warnings)
  if (tag === 'svg') return { svg: normalizeSvg(root), kind: 'svg', warnings }
  return { ...FAIL, error: '根节点必须是 <vector> 或 <svg>' }
}
