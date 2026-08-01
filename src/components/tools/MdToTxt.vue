<script setup lang="ts">
import { ref, watch } from 'vue'

const input = ref('')
const output = ref('')
const unescapeChars = ref(true)
const stripMarkdown = ref(true)
const keepCodeBlock = ref(true)

/** 示例：被污染的 Markdown（plain/markdown 反复切换后残留的转义 + 未渲染的标记） */
const SAMPLE = `# 笔记链接使用规范

## 语法

\`\`\`
[笔记标题](jclink://note/笔记ID)
\`\`\`

## 示例

- 详见 [项目架构设计](jclink://note/abc123-def456-...)
- 相关配置请参考 [Nginx 部署指南](jclink://note/789xyz-...)

::: info 🌾 这是一段被污染的纯文本，星号 \\* 和下划线 \\_ 以及方括号 \\[x\\] 都被转义了
:::

::: details 详情块标题
这里是详情块内容
:::

| 列A | 列B |
| --- | --- |
| 1 | 2 |

> 引用文字内容

1. **第一条**：加粗内容
2. 第二条：*斜体内容* 与 \`行内代码\`
`

/** 还原被污染的转义字符（plain/markdown 切换产生的 \* \_ \[ \] \` \~） */
function restoreEscapes(md: string): string {
  return md
    .replace(/\\([`*_\\[\]~])/g, '$1')
    .replace(/[ \t]+$/gm, '')
}

/** Markdown → 纯文本 */
function mdToPlain(md: string): string {
  const lines = md.split('\n')
  const out: string[] = []
  let inCode = false
  let inMath = false

  for (const raw of lines) {
    let line = raw
    const trimmed = line.trim()

    // 代码块围栏 ``` / ~~~
    if (keepCodeBlock.value && /^\s*(```+|~~~+)\s*[\w+-]*\s*$/.test(line)) {
      inCode = !inCode
      continue
    }
    if (keepCodeBlock.value && inCode) {
      out.push(line)
      continue
    }

    // 数学块 $$
    if (/^\s*\$\$\s*$/.test(line)) {
      inMath = !inMath
      continue
    }
    if (inMath) {
      out.push(line)
      continue
    }

    // ::: 块标记（含 getMarkdown 序列化产生的污染形式）
    if (trimmed.startsWith(':::')) {
      const m = trimmed.match(/^:::\s*([a-zA-Z][\w-]*)?\s*(.*)$/)
      if (!m) continue // 裸 :::
      const type = m[1]
      let rest = m[2]
      // 去掉 unique-id 残留 {#...}
      rest = rest.replace(/^\s*\{#[\w-]+\}\s*/, '')
      if (type === 'detailsSummary' || type === 'summary' || type === 'details') {
        // summary/标题类：保留其后文本
        if (rest.trim()) out.push(rest)
      } else if (type === 'callout' || type === 'column' || type === 'column-container' || type === 'aiBlock' || type === 'detailsContent') {
        // 块起始/结束标记，内容在后续行
        continue
      } else {
        // 序列化形式：::: info 🌾 内容（type 为 callout 类型）
        rest = rest.replace(/^\s*\p{Extended_Pictographic}\s*/u, '')
        if (rest.trim()) out.push(rest)
      }
      continue
    }

    // 表格分隔行 |---|---|
    if (/^\s*\|?[\s:\-|]+\|?\s*$/.test(line) && line.includes('|')) {
      continue
    }

    if (stripMarkdown.value) {
      // 标题
      line = line.replace(/^\s{0,3}#{1,6}\s+/, '')
      // 引用
      line = line.replace(/^\s{0,3}>\s?/, '')
      // 任务列表
      line = line.replace(/^\s{0,3}\[\s\]\s+/, '[ ] ')
      line = line.replace(/^\s{0,3}\[[xX]\]\s+/, '[x] ')
      // 无序 / 有序列表
      line = line.replace(/^\s{0,3}([-*+]|\d+\.)\s+/, '')
      // 分隔线
      if (/^\s*(?:-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
        continue
      }
      // 行内格式：图片/链接/行内代码/粗体/斜体/删除线/数学
      line = line
        .replace(/!\[([^\]]*)\]\(([^)]*)\)/g, '$1')
        .replace(/\[([^\]]*)\]\(([^)]*)\)/g, '$1')
        .replace(/`([^`]+)`/g, '$1')
        .replace(/\*\*([^*]+)\*\*/g, '$1')
        .replace(/__([^_]+)__/g, '$1')
        .replace(/(?<!\*)\*([^*\n]+)\*(?!\*)/g, '$1')
        .replace(/_([^_\n]+)_/g, '$1')
        .replace(/~~([^~]+)~~/g, '$1')
        .replace(/\$\$?([^$\n]+)\$\$?/g, '$1')
        .replace(/<[^>]+>/g, '')
    }

    // 表格行：去掉行首/行尾 | 并规整单元格间距
    if (line.includes('|')) {
      line = line.replace(/^\s*\|/, '').replace(/\|\s*$/, '').replace(/\s*\|\s*/g, '  ')
    }

    out.push(line)
  }

  return out
    .map(l => l.replace(/[ \t]+$/gm, ''))
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

function processText() {
  if (!input.value) {
    output.value = ''
    return
  }
  let text = input.value
  if (unescapeChars.value) {
    text = restoreEscapes(text)
  }
  if (stripMarkdown.value) {
    output.value = mdToPlain(text)
  } else {
    output.value = text
  }
}

watch([input, unescapeChars, stripMarkdown, keepCodeBlock], () => {
  processText()
}, { immediate: true })

function loadSample() {
  input.value = SAMPLE
}

function copyResult() {
  if (!output.value) return
  navigator.clipboard.writeText(output.value)
}

function clearAll() {
  input.value = ''
  output.value = ''
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">MD 转 TXT（Markdown 净化 → 纯文本）</div>
      <div class="tool-actions">
        <button class="tool-btn" @click="loadSample">示例</button>
        <button class="tool-btn pri" @click="copyResult" :disabled="!output">复制结果</button>
        <button class="tool-btn err" @click="clearAll">清空</button>
      </div>
    </div>

    <div class="tool-options">
      <label class="opt">
        <input type="checkbox" v-model="unescapeChars" /> 还原被污染的转义（\* → *）
      </label>
      <label class="opt">
        <input type="checkbox" v-model="stripMarkdown" /> 去除 Markdown 标记
      </label>
      <label class="opt">
        <input type="checkbox" v-model="keepCodeBlock" /> 保留代码块内容
      </label>
      <span class="opt-stats">输入 {{ input.length }} 字符 · 输出 {{ output.length }} 字符</span>
    </div>

    <div class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label">输入（被污染的 Markdown / 笔记内容）</div>
        <textarea v-model="input" placeholder="请粘贴被污染的 Markdown 或笔记原文..." spellcheck="false"></textarea>
      </div>
      <div class="editor-pane">
        <div class="pane-label">输出（纯净 TXT）</div>
        <textarea v-model="output" readonly placeholder="转换结果将显示在这里..." spellcheck="false" class="readonly-output"></textarea>
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
  margin-bottom: 8px;
  flex-shrink: 0;
}
.tool-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}
.tool-actions {
  display: flex;
  gap: 8px;
}
.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 4px 12px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 2px;
  transition: all 0.2s;
  &:hover:not(:disabled) {
    background: var(--jc-bg-btn-hover);
  }
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  &.pri {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    &:hover {
      background: var(--jc-color-accent-hover);
    }
  }
  &.err {
    &:hover {
      background: var(--jc-color-error);
      color: var(--jc-color-white);
    }
  }
}
.tool-options {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 8px;
  flex-shrink: 0;
  font-size: 11px;
  color: var(--jc-text-secondary);
  .opt {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    input {
      accent-color: var(--jc-color-accent);
    }
  }
  .opt-stats {
    margin-left: auto;
    opacity: 0.8;
  }
}
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 12px;
  min-height: 0;
}
.editor-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  padding: 8px;
  border-radius: 4px;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 6px;
  text-transform: uppercase;
}
textarea {
  flex: 1;
  width: 100%;
  resize: none;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  padding: 8px;
  outline: none;
  border-radius: 2px;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.readonly-output {
  background: var(--jc-bg-app);
  color: var(--jc-color-success);
}
</style>
