<script setup lang="ts">
import { ref, computed } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const regexStr = ref('')
const flags = ref('g')
const testText = ref('')

const matches = computed(() => {
  if (!regexStr.value || !testText.value) return []
  try {
    const re = new RegExp(regexStr.value, flags.value)
    const result: { text: string; index: number; groups: string[] }[] = []
    
    if (flags.value.includes('g')) {
      let match
      let lastIndex = -1
      while ((match = re.exec(testText.value)) !== null) {
        // 防止零宽匹配死循环
        if (re.lastIndex === lastIndex) {
          re.lastIndex++
        }
        lastIndex = re.lastIndex

        result.push({
          text: match[0],
          index: match.index,
          groups: match.slice(1).map(g => g || '(空)')
        })
        if (re.lastIndex === 0) break // 防止死循环
      }
    } else {
      const match = re.exec(testText.value)
      if (match) {
        result.push({
          text: match[0],
          index: match.index,
          groups: match.slice(1).map(g => g || '(空)')
        })
      }
    }
    return result
  } catch (e: any) {
    return [{ text: `正则语法错误: ${e.message}`, index: -1, groups: [] }]
  }
})

// 生成 HTML 带高亮文本，模拟实时渲染
const highlightedHtml = computed(() => {
  if (!testText.value) return '等待输入测试文本...'
  if (!regexStr.value) return testText.value

  try {
    const re = new RegExp(regexStr.value, flags.value)
    
    // 如果没有全局修饰符，只高亮第一个匹配
    if (!flags.value.includes('g')) {
      const match = re.exec(testText.value)
      if (match) {
        const start = match.index
        const end = start + match[0].length
        return escapeHtml(testText.value.slice(0, start)) +
               `<span class="hl-match">${escapeHtml(match[0])}</span>` +
               escapeHtml(testText.value.slice(end))
      }
      return escapeHtml(testText.value)
    }

    // 全局替换匹配
    let lastIndex = 0
    let html = ''
    let match
    let count = 0

    while ((match = re.exec(testText.value)) !== null && count < 500) {
      count++
      const start = match.index
      const matchText = match[0]
      if (matchText.length === 0) {
        re.lastIndex++
        continue
      }
      html += escapeHtml(testText.value.slice(lastIndex, start))
      html += `<span class="hl-match">${escapeHtml(matchText)}</span>`
      lastIndex = re.lastIndex
    }
    html += escapeHtml(testText.value.slice(lastIndex))
    return html
  } catch {
    return escapeHtml(testText.value)
  }
})

function escapeHtml(text: string) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
    .replace(/\n/g, '<br/>')
}
</script>

<template>
  <ToolShell title="正则表达式测试器" split>
    <template #left-label>正则输入与测试</template>
    <template #left>
      <div class="regex-input-card">
        <div class="fld-row">
          <span class="regex-slash">/</span>
          <JcInput beam v-model="regexStr" placeholder="在此输入正则表达式，如: [a-zA-Z]+" style="flex: 1; min-width: 0; font-family: 'Cascadia Code', Consolas, monospace" />
          <span class="regex-slash">/</span>
          <JcInput beam v-model="flags" title="修饰符：g=全局, i=忽略大小写, m=多行" style="width: 64px; font-family: 'Cascadia Code', Consolas, monospace; font-weight: 700" />
        </div>
      </div>

      <div class="editor-pane flex-1">
        <div class="pane-label">测试文本</div>
        <JcTextarea v-model="testText" mono beam :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="在此粘贴待匹配的测试文本..." />
      </div>

      <div class="editor-pane flex-1">
        <div class="pane-label">实时匹配高亮预览</div>
        <div class="highlight-preview jc-fill" v-html="highlightedHtml"></div>
      </div>
    </template>

    <template #right-label>匹配结果 (共 {{ matches.filter(m => m.index !== -1).length }} 处)</template>
    <template #right>
      <div class="editor-pane height-100">
        <div class="results-list">
          <div v-for="(m, idx) in matches" :key="idx" class="match-item" :class="{ error: m.index === -1 }">
            <template v-if="m.index === -1">
              <div class="match-error-text">{{ m.text }}</div>
            </template>
            <template v-else>
              <div class="match-meta">
                <span class="match-index">匹配 #{{ idx + 1 }}</span>
                <span class="match-pos">位置: {{ m.index }}..{{ m.index + m.text.length }}</span>
              </div>
              <div class="match-text">{{ m.text }}</div>
              <div v-if="m.groups.length > 0" class="match-groups">
                <div v-for="(g, gIdx) in m.groups" :key="gIdx" class="group-row">
                  <span class="group-label">捕获组 ${{ gIdx + 1 }}:</span>
                  <span class="group-val">{{ g }}</span>
                </div>
              </div>
            </template>
          </div>
          <div v-if="matches.length === 0" class="empty-tip">
            无匹配项
          </div>
        </div>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped lang="scss">
.regex-input-card {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 8px 12px;
  flex-shrink: 0;
}
.fld-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.regex-slash {
  font-size: 18px;
  font-weight: bold;
  color: var(--jc-text-secondary);
}
.editor-pane {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  padding: 8px;
  min-height: 0;
}
.flex-1 {
  flex: 1;
}
.height-100 {
  height: 100%;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 6px;
  text-transform: uppercase;
}
.highlight-preview {
  flex: 1;
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  padding: 8px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-all;
  user-select: text;
  &:deep(.hl-match) {
    background: rgba(138, 88, 255, 0.35);
    border-bottom: 1.5px solid var(--jc-color-accent);
    color: var(--jc-color-white);
    border-radius: 2px;
    padding: 0 1px;
  }
}
.results-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.match-item {
  background: var(--jc-bg-app);
  border: 1px solid var(--jc-border-default);
  padding: 8px;
  font-size: 12px;
  border-left: 3px solid var(--jc-color-success);
  &.error {
    border-left-color: var(--jc-color-error);
    background: rgba(244, 71, 71, 0.05);
  }
}
.match-error-text {
  color: var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
.match-meta {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  color: var(--jc-text-secondary);
  margin-bottom: 4px;
}
.match-text {
  font-family: 'Cascadia Code', Consolas, monospace;
  color: var(--jc-color-success);
  word-break: break-all;
}
.match-groups {
  margin-top: 6px;
  border-top: 1px dashed var(--jc-border-default);
  padding-top: 4px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.group-row {
  display: flex;
  font-size: 11px;
  font-family: 'Cascadia Code', Consolas, monospace;
}
.group-label {
  color: var(--jc-text-secondary);
  padding-right: 6px;
}
.group-val {
  color: var(--jc-text-highlight);
  word-break: break-all;
}
.empty-tip {
  text-align: center;
  padding: 40px;
  font-size: 12px;
  color: var(--jc-text-secondary);
}
</style>
