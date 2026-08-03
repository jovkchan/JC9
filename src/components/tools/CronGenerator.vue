<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'

const cronDialect = ref<'linux' | 'spring'>('linux') // Linux Crontab (5位) 还是 Java Spring (6位)

// 每一个部分的状态类型定义
type PartMode = 'all' | 'interval' | 'specific' | 'question'

interface PartState {
  mode: PartMode
  intervalStart: number
  intervalStep: number
  specifics: number[]
}

const secondState = ref<PartState>({ mode: 'all', intervalStart: 0, intervalStep: 5, specifics: [0] })
const minuteState = ref<PartState>({ mode: 'all', intervalStart: 0, intervalStep: 5, specifics: [0] })
const hourState = ref<PartState>({ mode: 'all', intervalStart: 0, intervalStep: 1, specifics: [0] })
const dayState = ref<PartState>({ mode: 'all', intervalStart: 1, intervalStep: 1, specifics: [1] })
const monthState = ref<PartState>({ mode: 'all', intervalStart: 1, intervalStep: 1, specifics: [1] })
const weekState = ref<PartState>({ mode: 'all', intervalStart: 0, intervalStep: 1, specifics: [0] })

const activeTab = ref<'second' | 'minute' | 'hour' | 'day' | 'month' | 'week'>('minute')

// 手动输入的表达式绑定
const manualCron = ref('0 2 * * *')
const isManualEditing = ref(false)

// 构建单个字段的值
function buildPartExpression(state: PartState, minVal: number, maxVal: number): string {
  if (state.mode === 'question') {
    return '?'
  }
  if (state.mode === 'all') {
    return '*'
  }
  if (state.mode === 'interval') {
    const start = Math.max(minVal, Math.min(maxVal, state.intervalStart))
    const step = Math.max(1, state.intervalStep)
    return `${start}/${step}`
  }
  if (state.mode === 'specific') {
    if (state.specifics.length === 0) {
      return '*'
    }
    const sorted = [...state.specifics].sort((a, b) => a - b)
    return sorted.join(',')
  }
  return '*'
}

// 自动生成 Cron 表达式
const generatedCron = computed(() => {
  const parts: string[] = []
  if (cronDialect.value === 'spring') {
    parts.push(buildPartExpression(secondState.value, 0, 59))
  }
  parts.push(buildPartExpression(minuteState.value, 0, 59))
  parts.push(buildPartExpression(hourState.value, 0, 23))
  parts.push(buildPartExpression(dayState.value, 1, 31))
  parts.push(buildPartExpression(monthState.value, 1, 12))
  parts.push(buildPartExpression(weekState.value, 0, 6)) // 0=周日，1-6=周一至周六
  
  return parts.join(' ')
})

// 监听生成的 cron 变更，同步到手动输入框
watch(generatedCron, (newVal) => {
  if (!isManualEditing.value) {
    manualCron.value = newVal
  }
})

// 智能互斥：在 Spring 规范下，日与周有且仅能有一个使用 ?
watch(() => dayState.value.mode, (newMode) => {
  if (cronDialect.value === 'spring' && newMode !== 'question') {
    weekState.value.mode = 'question'
  }
})

watch(() => weekState.value.mode, (newMode) => {
  if (cronDialect.value === 'spring' && newMode !== 'question') {
    dayState.value.mode = 'question'
  }
})

// 切换规范时的全局处理
watch(cronDialect, (newDialect) => {
  if (newDialect === 'linux') {
    // Linux 下不支持 ? 符号，全部重置为 all
    if (dayState.value.mode === 'question') dayState.value.mode = 'all'
    if (weekState.value.mode === 'question') weekState.value.mode = 'all'
    // 重建 5 位
    manualCron.value = '0 2 * * *'
  } else {
    // Spring 下强制让其中一个使用 ?
    if (dayState.value.mode !== 'question' && weekState.value.mode !== 'question') {
      weekState.value.mode = 'question'
    }
    // 重建 6 位
    manualCron.value = '0 0 2 * * ?'
  }
  parseAndApplyCron()
})

// 解析输入的表达式并应用到状态中
function parseAndApplyCron() {
  isManualEditing.value = true
  const cleanStr = manualCron.value.trim().replace(/\s+/g, ' ')
  const tokens = cleanStr.split(' ')

  // 判定模式
  let secStr = '0'
  let minStr = '*'
  let hrStr = '*'
  let dayStr = '*'
  let monStr = '*'
  let wkStr = '*'

  if (tokens.length === 6) {
    cronDialect.value = 'spring'
    secStr = tokens[0]
    minStr = tokens[1]
    hrStr = tokens[2]
    dayStr = tokens[3]
    monStr = tokens[4]
    wkStr = tokens[5]
  } else if (tokens.length === 5) {
    cronDialect.value = 'linux'
    secStr = '0'
    minStr = tokens[0]
    hrStr = tokens[1]
    dayStr = tokens[2]
    monStr = tokens[3]
    wkStr = tokens[4]
  } else {
    isManualEditing.value = false
    return
  }

  function parsePart(partStr: string, state: PartState, minVal: number, maxVal: number) {
    if (partStr === '?') {
      state.mode = 'question'
    } else if (partStr === '*') {
      state.mode = 'all'
    } else if (partStr.includes('/')) {
      state.mode = 'interval'
      const parts = partStr.split('/')
      state.intervalStart = parts[0] === '*' ? minVal : (parseInt(parts[0]) || minVal)
      state.intervalStep = parseInt(parts[1]) || 1
    } else {
      state.mode = 'specific'
      // 兼容范围 - 如 9-17，转换为具体值
      if (partStr.includes('-')) {
        const parts = partStr.split('-').map(v => parseInt(v))
        if (parts.length === 2 && !isNaN(parts[0]) && !isNaN(parts[1])) {
          const list: number[] = []
          for (let i = Math.min(parts[0], parts[1]); i <= Math.max(parts[0], parts[1]); i++) {
            if (i >= minVal && i <= maxVal) list.push(i)
          }
          state.specifics = list.length > 0 ? list : [minVal]
          return
        }
      }
      const values = partStr.split(',').map(v => parseInt(v)).filter(v => !isNaN(v) && v >= minVal && v <= maxVal)
      state.specifics = values.length > 0 ? values : [minVal]
    }
  }

  parsePart(secStr, secondState.value, 0, 59)
  parsePart(minStr, minuteState.value, 0, 59)
  parsePart(hrStr, hourState.value, 0, 23)
  parsePart(dayStr, dayState.value, 1, 31)
  parsePart(monStr, monthState.value, 1, 12)
  parsePart(wkStr, weekState.value, 0, 6)

  isManualEditing.value = false
}

// 常用模板，自适应所选的模式
interface CronPreset {
  name: string
  exprLinux: string
  exprSpring: string
  desc: string
}
const PRESETS: CronPreset[] = [
  { name: '每分钟', exprLinux: '* * * * *', exprSpring: '0 * * * * ?', desc: '每分钟执行一次' },
  { name: '每小时零分', exprLinux: '0 * * * *', exprSpring: '0 0 * * * ?', desc: '每小时的第 0 分钟执行一次' },
  { name: '每天凌晨2点', exprLinux: '0 2 * * *', exprSpring: '0 0 2 * * ?', desc: '每天的凌晨 02:00 执行一次' },
  { name: '每周一凌晨3点', exprLinux: '0 3 * * 1', exprSpring: '0 0 3 ? * 1', desc: '每周一的凌晨 03:00 执行一次' },
  { name: '每月1号零点', exprLinux: '0 0 1 * *', exprSpring: '0 0 0 1 * ?', desc: '每个月的 1 号 00:00 执行一次' },
  { name: '工作日朝九整点', exprLinux: '0 9 * * 1-5', exprSpring: '0 0 9 ? * 1-5', desc: '周一至周五的 9 点整点执行' }
]

function applyPreset(p: CronPreset) {
  manualCron.value = cronDialect.value === 'linux' ? p.exprLinux : p.exprSpring
  parseAndApplyCron()
}

// 勾选/取消勾选多选框时的辅助
function toggleSpecific(state: PartState, val: number) {
  state.mode = 'specific'
  const idx = state.specifics.indexOf(val)
  if (idx > -1) {
    state.specifics.splice(idx, 1)
  } else {
    state.specifics.push(val)
  }
}

// 中文规则直白翻译函数
const translatedText = computed(() => {
  const cleanStr = manualCron.value.trim().replace(/\s+/g, ' ')
  const tokens = cleanStr.split(' ')
  
  if (tokens.length !== 5 && tokens.length !== 6) {
    return '【解析错误】Cron 表达式格式应为 5 位或 6 位空格分隔符。'
  }

  let secStr = '0'
  let minStr = '*'
  let hrStr = '*'
  let dayStr = '*'
  let monStr = '*'
  let wkStr = '*'

  const isSpringMode = tokens.length === 6

  if (isSpringMode) {
    secStr = tokens[0]
    minStr = tokens[1]
    hrStr = tokens[2]
    dayStr = tokens[3]
    monStr = tokens[4]
    wkStr = tokens[5]
  } else {
    minStr = tokens[0]
    hrStr = tokens[1]
    dayStr = tokens[2]
    monStr = tokens[3]
    wkStr = tokens[4]
  }

  const translatePart = (str: string, label: string, formatter?: (v: number) => string): string => {
    if (str === '?') return '（不指定，由另一项决定）'
    if (str === '*') return `每${label}`
    if (str.includes('/')) {
      const parts = str.split('/')
      const start = parts[0] === '*' ? '从 0 开始' : `从 ${parts[0]} ${label}开始`
      return `${start}，每隔 ${parts[1]} ${label}执行一次`
    }
    if (str.includes('-')) {
      const parts = str.split('-')
      return `在 ${parts[0]} 到 ${parts[1]} ${label}期间`
    }
    const values = str.split(',').map(v => parseInt(v)).filter(v => !isNaN(v))
    if (values.length > 0) {
      const formatted = formatter ? values.map(formatter) : values.map(v => `${v}${label}`)
      return `在 [${formatted.join(', ')}]`
    }
    return `每${label}`
  }

  const WEEK_DAYS = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
  const formatWeek = (v: number) => WEEK_DAYS[v] || `周${v}`
  const formatMonth = (v: number) => `${v}月`
  const formatHour = (v: number) => `${String(v).padStart(2, '0')}点`
  const formatMinute = (v: number) => `${String(v).padStart(2, '0')}分`
  const formatSecond = (v: number) => `${String(v).padStart(2, '0')}秒`

  // 拼接说明
  let result = '【运行逻辑】：\n'
  
  // 周
  if (wkStr !== '*' && wkStr !== '?') {
    result += `📅 ${translatePart(wkStr, '', formatWeek)}\n`
  }
  // 月
  if (monStr !== '*') {
    result += `🌙 ${translatePart(monStr, '月份', formatMonth)}\n`
  }
  // 日
  if (dayStr !== '*' && dayStr !== '?') {
    result += `📆 ${translatePart(dayStr, '号')}\n`
  } else if (wkStr === '*' || wkStr === '?') {
    result += `📆 每天\n`
  }
  // 时、分、秒
  result += `⏰ 时间点为：`
  if (isSpringMode && secStr !== '0') {
    result += `${translatePart(hrStr, '点', formatHour)} ${translatePart(minStr, '分', formatMinute)} ${translatePart(secStr, '秒', formatSecond)}`
  } else {
    result += `${translatePart(hrStr, '点', formatHour)} ${translatePart(minStr, '分', formatMinute)}`
  }

  return result
})

function copyCron() {
  navigator.clipboard.writeText(manualCron.value)
}

onMounted(() => {
  activeTab.value = 'minute'
})
</script>

<template>
  <ToolShell title="Cron 表达式生成与解析器" split>
    <template #left-label>点选生成配置</template>
    <template #left>
      <div class="control-panel">
        <!-- 头部预置模板 -->
        <div class="setting-section border-b pb-12">
          <div class="section-subtitle">常用快捷模板</div>
          <div class="presets-row-grid">
            <button 
              v-for="p in PRESETS" 
              :key="p.name" 
              class="preset-card-item" 
              @click="applyPreset(p)" 
              :title="p.desc"
            >
              <div class="preset-title">{{ p.name }}</div>
              <div class="preset-code">{{ cronDialect === 'linux' ? p.exprLinux : p.exprSpring }}</div>
            </button>
          </div>
        </div>

        <!-- 选项卡选择 -->
        <div class="setting-section flex-fill">
          <div class="section-subtitle">点选生成配置</div>
          <div class="tabs-row">
            <button 
              v-if="cronDialect === 'spring'" 
              :class="['tab-item', { on: activeTab === 'second' }]" 
              @click="activeTab = 'second'"
            >
              秒
            </button>
            <button :class="['tab-item', { on: activeTab === 'minute' }]" @click="activeTab = 'minute'">分</button>
            <button :class="['tab-item', { on: activeTab === 'hour' }]" @click="activeTab = 'hour'">时</button>
            <button :class="['tab-item', { on: activeTab === 'day' }]" @click="activeTab = 'day'">日</button>
            <button :class="['tab-item', { on: activeTab === 'month' }]" @click="activeTab = 'month'">月</button>
            <button :class="['tab-item', { on: activeTab === 'week' }]" @click="activeTab = 'week'">周</button>
          </div>

          <!-- 各段的卡片控制选项 -->
          <div class="config-detail-area">
            <!-- 秒配置 -->
            <div v-if="activeTab === 'second' && cronDialect === 'spring'" class="config-part">
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="all" v-model="secondState.mode" />
                  <span>每秒钟 (*)</span>
                </label>
              </div>
              <div class="mode-row flex-align">
                <label class="radio-label">
                  <input type="radio" value="interval" v-model="secondState.mode" />
                  <span>间隔秒数</span>
                </label>
                <div v-if="secondState.mode === 'interval'" class="interval-controls">
                  从第 <input type="number" v-model.number="secondState.intervalStart" min="0" max="59" /> 秒开始，
                  每隔 <input type="number" v-model.number="secondState.intervalStep" min="1" max="59" /> 秒一次
                </div>
              </div>
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="specific" v-model="secondState.mode" />
                  <span>指定具体秒 (可多选)</span>
                </label>
                <div v-if="secondState.mode === 'specific'" class="numbers-grid-60">
                  <button 
                    v-for="v in 60" 
                    :key="v-1" 
                    :class="['num-box', { on: secondState.specifics.includes(v-1) }]"
                    @click="toggleSpecific(secondState, v-1)"
                  >
                    {{ v-1 }}
                  </button>
                </div>
              </div>
            </div>

            <!-- 分配置 -->
            <div v-if="activeTab === 'minute'" class="config-part">
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="all" v-model="minuteState.mode" />
                  <span>每分钟 (*)</span>
                </label>
              </div>
              <div class="mode-row flex-align">
                <label class="radio-label">
                  <input type="radio" value="interval" v-model="minuteState.mode" />
                  <span>间隔分钟</span>
                </label>
                <div v-if="minuteState.mode === 'interval'" class="interval-controls">
                  从第 <input type="number" v-model.number="minuteState.intervalStart" min="0" max="59" /> 分开始，
                  每隔 <input type="number" v-model.number="minuteState.intervalStep" min="1" max="59" /> 分钟一次
                </div>
              </div>
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="specific" v-model="minuteState.mode" />
                  <span>指定具体分钟 (可多选)</span>
                </label>
                <div v-if="minuteState.mode === 'specific'" class="numbers-grid-60">
                  <button 
                    v-for="v in 60" 
                    :key="v-1" 
                    :class="['num-box', { on: minuteState.specifics.includes(v-1) }]"
                    @click="toggleSpecific(minuteState, v-1)"
                  >
                    {{ v-1 }}
                  </button>
                </div>
              </div>
            </div>

            <!-- 时配置 -->
            <div v-if="activeTab === 'hour'" class="config-part">
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="all" v-model="hourState.mode" />
                  <span>每小时 (*)</span>
                </label>
              </div>
              <div class="mode-row flex-align">
                <label class="radio-label">
                  <input type="radio" value="interval" v-model="hourState.mode" />
                  <span>间隔小时</span>
                </label>
                <div v-if="hourState.mode === 'interval'" class="interval-controls">
                  从第 <input type="number" v-model.number="hourState.intervalStart" min="0" max="23" /> 点开始，
                  每隔 <input type="number" v-model.number="hourState.intervalStep" min="1" max="23" /> 小时一次
                </div>
              </div>
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="specific" v-model="hourState.mode" />
                  <span>指定具体小时 (可多选)</span>
                </label>
                <div v-if="hourState.mode === 'specific'" class="numbers-grid-24">
                  <button 
                    v-for="v in 24" 
                    :key="v-1" 
                    :class="['num-box', { on: hourState.specifics.includes(v-1) }]"
                    @click="toggleSpecific(hourState, v-1)"
                  >
                    {{ v-1 }}点
                  </button>
                </div>
              </div>
            </div>

            <!-- 日配置 -->
            <div v-if="activeTab === 'day'" class="config-part">
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="all" v-model="dayState.mode" />
                  <span>每日 (*)</span>
                </label>
              </div>
              <div v-if="cronDialect === 'spring'" class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="question" v-model="dayState.mode" />
                  <span>不限日期，由星期决定 (?)</span>
                </label>
              </div>
              <div class="mode-row flex-align">
                <label class="radio-label">
                  <input type="radio" value="interval" v-model="dayState.mode" />
                  <span>间隔天数</span>
                </label>
                <div v-if="dayState.mode === 'interval'" class="interval-controls">
                  从第 <input type="number" v-model.number="dayState.intervalStart" min="1" max="31" /> 号开始，
                  每隔 <input type="number" v-model.number="dayState.intervalStep" min="1" max="31" /> 天一次
                </div>
              </div>
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="specific" v-model="dayState.mode" />
                  <span>指定具体号数 (可多选)</span>
                </label>
                <div v-if="dayState.mode === 'specific'" class="numbers-grid-31">
                  <button 
                    v-for="v in 31" 
                    :key="v" 
                    :class="['num-box', { on: dayState.specifics.includes(v) }]"
                    @click="toggleSpecific(dayState, v)"
                  >
                    {{ v }}号
                  </button>
                </div>
              </div>
            </div>

            <!-- 月配置 -->
            <div v-if="activeTab === 'month'" class="config-part">
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="all" v-model="monthState.mode" />
                  <span>每月 (*)</span>
                </label>
              </div>
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="specific" v-model="monthState.mode" />
                  <span>指定具体月份 (可多选)</span>
                </label>
                <div v-if="monthState.mode === 'specific'" class="numbers-grid-12">
                  <button 
                    v-for="v in 12" 
                    :key="v" 
                    :class="['num-box', { on: monthState.specifics.includes(v) }]"
                    @click="toggleSpecific(monthState, v)"
                  >
                    {{ v }}月
                  </button>
                </div>
              </div>
            </div>

            <!-- 周配置 -->
            <div v-if="activeTab === 'week'" class="config-part">
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="all" v-model="weekState.mode" />
                  <span>每周 (*)</span>
                </label>
              </div>
              <div v-if="cronDialect === 'spring'" class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="question" v-model="weekState.mode" />
                  <span>不限星期，由日期决定 (?)</span>
                </label>
              </div>
              <div class="mode-row">
                <label class="radio-label">
                  <input type="radio" value="specific" v-model="weekState.mode" />
                  <span>指定星期几 (可多选)</span>
                </label>
                <div v-if="weekState.mode === 'specific'" class="numbers-grid-7">
                  <button 
                    v-for="(wName, idx) in ['周日', '周一', '周二', '周三', '周四', '周五', '周六']" 
                    :key="idx" 
                    :class="['num-box', { on: weekState.specifics.includes(idx) }]"
                    @click="toggleSpecific(weekState, idx)"
                  >
                    {{ wName }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <template #right-label>结果与解析</template>
    <template #right>
      <div class="result-panel">
        <!-- 表达式规范选择 -->
        <div class="setting-section border-b pb-12">
          <div class="section-subtitle">表达式规范与方言</div>
          <div class="dialect-choice-row">
            <label class="radio-label">
              <input type="radio" value="linux" v-model="cronDialect" />
              <span>Linux crontab (5位标准)</span>
            </label>
            <label class="radio-label">
              <input type="radio" value="spring" v-model="cronDialect" />
              <span>Java Spring / Quartz (6位标准)</span>
            </label>
          </div>
        </div>

        <div class="setting-section">
          <div class="section-subtitle">1. 表达式绑定输出 (Cron Expression)</div>
          <div class="cron-input-row">
            <JcInput
              beam
              v-model="manualCron"
              @input="parseAndApplyCron"
              placeholder="请输入 Cron 表达式"
              style="flex: 1; min-width: 0"
            />
            <JcButton type="primary" size="small" @click="copyCron">复制表达式</JcButton>
          </div>
        </div>

        <div class="setting-section flex-fill flex flex-col">
          <div class="section-subtitle">2. 中文直白语义解释</div>
          <div class="explanation-box">
            <pre class="translate-text">{{ translatedText }}</pre>
          </div>
        </div>
      </div>
    </template>
  </ToolShell>
</template>

<style scoped lang="scss">
/* 左侧配置栏 */
.control-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  gap: 14px;
  overflow-y: auto;
}

.setting-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  
  &.border-b {
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }
  &.pb-12 {
    padding-bottom: 12px;
  }
  &.flex-fill {
    flex: 1;
    min-height: 0;
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

/* 常用预置模板网格 */
.presets-row-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}
.preset-card-item {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  padding: 6px 10px;
  cursor: pointer;
  text-align: left;
  transition: all 0.2s;
  
  &:hover {
    border-color: var(--jc-color-accent);
    background: var(--jc-bg-hover);
  }
  
  .preset-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--jc-text-highlight);
  }
  .preset-code {
    font-size: 10px;
    font-family: 'Cascadia Code', Consolas, monospace;
    color: var(--jc-color-success);
    margin-top: 2px;
  }
}

/* 选项卡按钮 */
.tabs-row {
  display: flex;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  padding: 2px;
  flex-shrink: 0;
  gap: 2px;
}
.tab-item {
  flex: 1;
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  padding: 4px 0;
  font-size: 11px;
  cursor: pointer;
  border-radius: 3px;
  font-weight: 600;
  transition: all 0.2s;
  &:hover {
    color: var(--jc-text-primary);
  }
  &.on {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
  }
}

.config-detail-area {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  padding: 10px;
}

.config-part {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.mode-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
  
  &.flex-align {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
}

.radio-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--jc-text-primary);
  cursor: pointer;
  font-weight: 600;
  input[type="radio"] {
    accent-color: var(--jc-color-accent);
    margin: 0;
  }
}

.interval-controls {
  font-size: 11px;
  color: var(--jc-text-secondary);
  display: flex;
  align-items: center;
  gap: 4px;
  
  input[type="number"] {
    background: var(--jc-bg-panel);
    border: 1px solid var(--jc-border-strong);
    color: var(--jc-text-primary);
    padding: 2px 4px;
    width: 50px;
    outline: none;
    border-radius: 3px;
    font-size: 11px;
    text-align: center;
    &:focus {
      border-color: var(--jc-color-accent);
    }
  }
}

/* 点选数字网格 */
.numbers-grid-60, .numbers-grid-24, .numbers-grid-31, .numbers-grid-12, .numbers-grid-7 {
  display: grid;
  gap: 4px;
  padding: 6px 0;
}
.numbers-grid-60 { grid-template-columns: repeat(10, 1fr); }
.numbers-grid-24 { grid-template-columns: repeat(8, 1fr); }
.numbers-grid-31 { grid-template-columns: repeat(8, 1fr); }
.numbers-grid-12 { grid-template-columns: repeat(6, 1fr); }
.numbers-grid-7 { grid-template-columns: repeat(7, 1fr); }

.num-box {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  color: var(--jc-text-primary);
  border-radius: 3px;
  padding: 4px 0;
  font-size: 10px;
  font-family: 'Cascadia Code', Consolas, monospace;
  cursor: pointer;
  text-align: center;
  transition: all 0.15s;
  
  &:hover {
    border-color: var(--jc-color-accent-hover);
  }
  &.on {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    border-color: var(--jc-color-accent);
  }
}

/* 右侧解析面板 */
.result-panel {
  display: flex;
  flex-direction: column;
  flex: 0 0 350px;
  gap: 16px;
}

.dialect-choice-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 2px 0;
}

.cron-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

/* 中文解析框 */
.explanation-box {
  flex: 1;
  background: #141416;
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 4px;
  padding: 12px;
  overflow-y: auto;
}
.translate-text {
  margin: 0;
  font-family: inherit;
  font-size: 12px;
  color: #abb2bf;
  white-space: pre-wrap;
  line-height: 1.6;
}

.flex { display: flex; }
.flex-col { flex-direction: column; }
.mt-8 { margin-top: 8px; }
</style>
