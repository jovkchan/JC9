<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const input = ref('')
const output = ref('')
const errorMsg = ref('')
const indentSize = ref(2)
const caseOption = ref<'uppercase' | 'lowercase' | 'preserve'>('uppercase')

const caseOptions = [
  { label: '关键字大写', value: 'uppercase' },
  { label: '关键字小写', value: 'lowercase' },
  { label: '保持原样', value: 'preserve' },
]
const indentOptions = [
  { label: '2 空格', value: 2 },
  { label: '4 空格', value: 4 },
]

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
  <ToolShell title="SQL 格式化 / 压缩" subtitle="Format / Minify" split>
    <template #actions>
      <JcSelect v-model="caseOption" :options="caseOptions" size="small" @change="formatSql" />
      <JcSelect v-model="indentSize" :options="indentOptions" size="small" @change="formatSql" />
      <JcButton type="primary" size="small" @click="formatSql">格式化</JcButton>
      <JcButton size="small" @click="minifySql">一键压缩</JcButton>
      <JcButton size="small" :disabled="!output" @click="copyResult">复制结果</JcButton>
      <JcButton size="small" danger ghost @click="clearAll">清空</JcButton>
    </template>
    <template #left-label>输入 SQL 语句</template>
    <template #left>
      <JcTextarea v-model="input" mono :spellcheck="false" class="jc-fill" placeholder="在此粘贴需要美化或压缩的一长串 SQL..." @input="formatSql" />
    </template>
    <template #right-label>处理结果</template>
    <template #right>
      <div class="sql-right">
        <JcTextarea v-model="output" mono readonly :spellcheck="false" class="jc-fill" placeholder="等待处理..." />
        <div v-if="errorMsg" class="sql-error">{{ errorMsg }}</div>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped>
.sql-right { display: flex; flex-direction: column; gap: 8px; flex: 1; min-height: 0; }
.sql-error {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>
