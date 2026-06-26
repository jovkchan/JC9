<script setup lang="ts">
import { ref } from 'vue'

const input = ref('')
const output = ref('')
const errorMsg = ref('')
const indentSize = ref(2)
const caseOption = ref<'uppercase' | 'lowercase' | 'preserve'>('uppercase')

const mainKeywords = [
  'SELECT', 'FROM', 'WHERE', 'GROUP BY', 'HAVING', 'ORDER BY', 
  'LIMIT', 'OFFSET', 'JOIN', 'LEFT JOIN', 'RIGHT JOIN', 'INNER JOIN', 
  'OUTER JOIN', 'ON', 'VALUES', 'SET', 'INSERT INTO', 'UPDATE', 'DELETE FROM',
  'UNION', 'UNION ALL', 'IN', 'AND', 'OR', 'AS'
]

function formatSql() {
  errorMsg.value = ''
  if (!input.value.trim()) {
    output.value = ''
    return
  }

  try {
    let sql = input.value;
    // 移除多行和单行注释（格式化时也可以保留，但暂作清洗以防格式错乱）
    sql = sql.replace(/\/\*[\s\S]*?\*\//g, '').replace(/--.*?\n/g, '\n')

    // 简单词法分析分词
    const tokens: string[] = []
    let currentToken = ''
    let inString = false
    let stringChar = ''

    for (let i = 0; i < sql.length; i++) {
      const char = sql[i]

      // 字符串字面量处理
      if ((char === "'" || char === '"' || char === '`') && (i === 0 || sql[i - 1] !== '\\')) {
        if (!inString) {
          inString = true;
          stringChar = char
          currentToken += char
        } else if (char === stringChar) {
          inString = false
          currentToken += char
          tokens.push(currentToken)
          currentToken = ''
        } else {
          currentToken += char
        }
        continue
      }

      if (inString) {
        currentToken += char
        continue
      }

      // 括号与标点
      if (char === '(' || char === ')' || char === ',' || char === ';') {
        if (currentToken.trim()) {
          tokens.push(currentToken.trim())
          currentToken = ''
        }
        tokens.push(char)
        continue
      }

      // 空白字符
      if (/\s/.test(char)) {
        if (currentToken.trim()) {
          tokens.push(currentToken.trim())
          currentToken = ''
        }
        continue
      }

      currentToken += char
    }
    if (currentToken.trim()) {
      tokens.push(currentToken.trim())
    }

    // 格式化输出构建
    let formatted = ''
    let indentLevel = 0
    const indent = () => ' '.repeat(indentLevel * indentSize.value)

    const parentKeywords = ['SELECT', 'FROM', 'WHERE', 'GROUP BY', 'HAVING', 'ORDER BY', 'VALUES', 'SET', 'INSERT INTO', 'UPDATE', 'DELETE FROM', 'UNION', 'UNION ALL']
    const joinKeywords = ['JOIN', 'LEFT JOIN', 'RIGHT JOIN', 'INNER JOIN', 'OUTER JOIN']
    const logicalKeywords = ['AND', 'OR']

    for (let i = 0; i < tokens.length; i++) {
      const token = tokens[i]
      const upperToken = token.toUpperCase()

      // 括号换行与缩进
      if (token === '(') {
        formatted += ' (\n'
        indentLevel++
        formatted += indent()
        continue
      }
      if (token === ')') {
        formatted = formatted.trimEnd() + '\n'
        indentLevel = Math.max(0, indentLevel - 1)
        formatted += indent() + ')'
        continue
      }
      if (token === ',') {
        formatted = formatted.trimEnd() + ',\n' + indent()
        continue
      }
      if (token === ';') {
        formatted = formatted.trimEnd() + ';\n'
        continue
      }

      // 检查复合多词关键字
      let matchedKeyword = ''
      // 先检测双词或三词组合，比如 "GROUP BY", "ORDER BY", "LEFT JOIN", "UNION ALL"
      const combos = [
        [tokens[i], tokens[i+1], tokens[i+2]],
        [tokens[i], tokens[i+1]]
      ]
      for (const combo of combos) {
        if (combo.every(Boolean)) {
          const joined = combo.join(' ').toUpperCase()
          if (mainKeywords.includes(joined)) {
            matchedKeyword = joined
            i += combo.length - 1
            break
          }
        }
      }

      if (!matchedKeyword && mainKeywords.includes(upperToken)) {
        matchedKeyword = upperToken
      }

      if (matchedKeyword) {
        let displayKw = matchedKeyword
        if (caseOption.value === 'uppercase') {
          displayKw = matchedKeyword.toUpperCase()
        } else if (caseOption.value === 'lowercase') {
          displayKw = matchedKeyword.toLowerCase()
        }

        if (parentKeywords.includes(matchedKeyword)) {
          // 主语句结构换行
          if (formatted.length > 0) {
            formatted = formatted.trimEnd() + '\n'
          }
          formatted += indent() + displayKw
        } else if (joinKeywords.includes(matchedKeyword)) {
          // 连表结构换行并加倍对齐
          if (formatted.length > 0) {
            formatted = formatted.trimEnd() + '\n'
          }
          formatted += indent() + displayKw
        } else if (logicalKeywords.includes(matchedKeyword)) {
          // 条件逻辑换行
          if (formatted.length > 0) {
            formatted = formatted.trimEnd() + '\n'
          }
          formatted += indent() + '  ' + displayKw
        } else {
          // 其他关键字（如 ON, AS 等）
          if (formatted.length > 0 && !formatted.endsWith('\n') && !formatted.endsWith(' ') && !formatted.endsWith('(')) {
            formatted += ' '
          }
          formatted += displayKw
        }
        continue
      }

      // 普通标识符/字面量
      if (formatted.length > 0 && !formatted.endsWith('\n') && !formatted.endsWith(' ') && !formatted.endsWith('(')) {
        formatted += ' '
      }
      
      // 非字面量可以根据大小写配置进行大写化或小写化
      const isLiteral = (token.startsWith("'") || token.startsWith('"') || token.startsWith('`') || !isNaN(Number(token)))
      if (!isLiteral) {
        if (caseOption.value === 'uppercase') {
          formatted += token.toUpperCase()
        } else if (caseOption.value === 'lowercase') {
          formatted += token.toLowerCase()
        } else {
          formatted += token
        }
      } else {
        formatted += token
      }
    }

    output.value = formatted.trim().replace(/\n\s*\n/g, '\n')
  } catch (e: any) {
    errorMsg.value = 'SQL 格式化失败: ' + (e.message || '语法错误')
    output.value = ''
  }
}

function minifySql() {
  errorMsg.value = ''
  if (!input.value.trim()) {
    output.value = ''
    return
  }

  try {
    let sql = input.value
    // 移除多行注释
    sql = sql.replace(/\/\*[\s\S]*?\*\//g, '')
    // 移除单行注释
    sql = sql.replace(/--.*?\n/g, '\n')
    // 将所有多余空字符合并为单个空格
    sql = sql.replace(/\s+/g, ' ')
    // 移除主要符号两边无意义的空格
    sql = sql.replace(/\s*([,=\(\)\+\-\*\/;])\s*/g, '$1')
    output.value = sql.trim()
  } catch (e: any) {
    errorMsg.value = 'SQL 压缩失败: ' + (e.message || '未知错误')
    output.value = ''
  }
}

function copyResult() {
  if (!output.value) return
  navigator.clipboard.writeText(output.value)
}

function clearAll() {
  input.value = ''
  output.value = ''
  errorMsg.value = ''
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">SQL 格式化 / 压缩</div>
      <div class="tool-actions">
        <label>大小写：</label>
        <select v-model="caseOption" @change="formatSql" class="tool-select">
          <option value="uppercase">关键字大写</option>
          <option value="lowercase">关键字小写</option>
          <option value="preserve">保持原样</option>
        </select>
        <label>缩进：</label>
        <select v-model="indentSize" @change="formatSql" class="tool-select">
          <option :value="2">2 空格</option>
          <option :value="4">4 空格</option>
        </select>
        <button class="tool-btn pri" @click="formatSql">格式化</button>
        <button class="tool-btn" @click="minifySql">一键压缩</button>
        <button class="tool-btn" @click="copyResult" :disabled="!output">复制结果</button>
        <button class="tool-btn err" @click="clearAll">清空</button>
      </div>
    </div>
    <div class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label">输入 SQL 语句</div>
        <textarea v-model="input" @input="formatSql" placeholder="在此粘贴需要美化或压缩的一长串 SQL..." spellcheck="false"></textarea>
      </div>
      <div class="editor-pane">
        <div class="pane-label">处理结果</div>
        <textarea v-model="output" readonly placeholder="等待处理..." spellcheck="false" class="readonly-output"></textarea>
      </div>
    </div>
    <div v-if="errorMsg" class="tool-footer-error">{{ errorMsg }}</div>
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
  margin-bottom: 10px;
  flex-shrink: 0;
}
.tool-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}
.tool-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}
.tool-select {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 3px 6px;
  font-size: 11px;
  outline: none;
  border-radius: 2px;
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
.tool-footer-error {
  flex-shrink: 0;
  margin-top: 8px;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>
