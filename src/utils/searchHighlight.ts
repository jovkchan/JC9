/**
 * 搜索高亮 / 摘要工具
 *
 * 用于搜索下拉与搜索 Tab 页：把笔记正文（可能是 HTML / Markdown）转纯文本，
 * 提取命中位置附近的摘要片段，并把查询词用 <mark> 高亮。
 * 所有返回 HTML 字符串的地方，调用方必须用 v-html 渲染。
 */

/** 把 HTML 转为纯文本（保留换行），Markdown 源码原样保留 */
export function htmlToText(html: string): string {
  if (!html) return ''
  if (!/<[a-z][\s\S]*>/i.test(html)) return html
  const el = document.createElement('div')
  el.innerHTML = html
  // 块级元素与 <br> 之后补换行，保证段落可读
  el.querySelectorAll('br').forEach(b => b.replaceWith('\n'))
  el.querySelectorAll('p,div,li,h1,h2,h3,h4,h5,h6,pre,blockquote,tr,details').forEach(n => {
    n.append('\n')
  })
  return (el.textContent || '').replace(/\u00a0/g, ' ')
}

/** HTML 转义（高亮前必须先转义，防止注入） */
export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

/** 从查询串解析出「纯关键词」（剔除 is:xxx / tag:xxx 语法指令） */
export function searchKeywords(query: string): string[] {
  return query
    .split(/\s+/)
    .map(w => w.trim())
    .filter(Boolean)
    .filter(w => !w.startsWith('is:') && !w.startsWith('tag:'))
}

/**
 * 把 text 中出现的每个关键词用 <mark> 包裹（返回 HTML）。
 * 先转义文本，再按「长词优先」逐词替换，降低重叠词嵌套概率。
 */
export function highlightKeywords(text: string, query: string): string {
  let words = searchKeywords(query)
  if (!words.length || !text) return escapeHtml(text || '')
  words = [...words].sort((a, b) => b.length - a.length)
  let escaped = escapeHtml(text)
  for (const w of words) {
    const ew = escapeHtml(w)
    if (!ew) continue
    escaped = escaped.split(ew).join(`<mark>${ew}</mark>`)
  }
  return escaped
}

/**
 * 从正文（HTML）提取命中位置附近的摘要片段（百度式：命中词居中前后留白）。
 * 标题/标签命中但正文无命中时回退到开头截断。
 */
export function makeSnippet(text: string, query: string, maxLen = 140): string {
  const clean = htmlToText(text).replace(/\s+/g, ' ').trim()
  if (!clean) return ''
  const words = searchKeywords(query)
  let idx = -1
  for (const w of words) {
    const i = clean.toLowerCase().indexOf(w.toLowerCase())
    if (i !== -1 && (idx === -1 || i < idx)) idx = i
  }
  if (idx === -1) {
    return clean.length > maxLen ? clean.slice(0, maxLen) + '…' : clean
  }
  const start = Math.max(0, idx - 40)
  const end = Math.min(clean.length, start + maxLen)
  return (start > 0 ? '…' : '') + clean.slice(start, end) + (end < clean.length ? '…' : '')
}

/** 取正文首个非空行作为缺省标题 */
export function autoTitle(content: string): string {
  if (!content) return ''
  const line = htmlToText(content).split('\n').find(l => l.trim()) || ''
  return line.replace(/^#+\s*/, '').trim()
}
