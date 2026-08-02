<script setup lang="ts">
import { ref, onMounted } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'

const activeTab = ref<'business-days' | 'date-diff'>('business-days')

// 今天日期格式化为 YYYY-MM-DD
function getTodayString() {
  const d = new Date()
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

// 格式化为 YYYY-MM-DD THH:MM
function getTodayDateTimeString() {
  const d = new Date()
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  const hours = String(d.getHours()).padStart(2, '0')
  const minutes = String(d.getMinutes()).padStart(2, '0')
  return `${year}-${month}-${day}T${hours}:${minutes}`
}

// ================= 工作日计算 =================
const startDate = ref(getTodayString())
const daysOffset = ref(3)
const workDayResult = ref('')
const dayNames = ['星期日', '星期一', '星期二', '星期三', '星期四', '星期五', '星期六']

function calculateWorkDays() {
  if (!startDate.value) {
    workDayResult.value = '请选择起始日期'
    return
  }

  const offset = Number(daysOffset.value)
  if (isNaN(offset)) {
    workDayResult.value = '偏移天数输入不合法'
    return
  }

  let currentDate = new Date(startDate.value)
  if (isNaN(currentDate.getTime())) {
    workDayResult.value = '起始日期不合法'
    return
  }

  let remaining = Math.abs(offset)
  const step = offset >= 0 ? 1 : -1

  // 如果偏移量为0，直接输出当前日期
  if (offset !== 0) {
    while (remaining > 0) {
      currentDate.setDate(currentDate.getDate() + step)
      const day = currentDate.getDay()
      // 0 = 周日, 6 = 周六
      if (day !== 0 && day !== 6) {
        remaining--
      }
    }
  }

  const year = currentDate.getFullYear()
  const month = String(currentDate.getMonth() + 1).padStart(2, '0')
  const date = String(currentDate.getDate()).padStart(2, '0')
  const dayStr = dayNames[currentDate.getDay()]

  workDayResult.value = `${year}-${month}-${date} (${dayStr})`
}

// ================= 日期差值计算 =================
const diffStart = ref(getTodayDateTimeString())
const diffEnd = ref(getTodayDateTimeString())
const diffResultStr = ref('')
const diffMs = ref(0)
const diffStats = ref({ days: 0, hours: 0, mins: 0, secs: 0 })

function calculateDiff() {
  if (!diffStart.value || !diffEnd.value) {
    diffResultStr.value = '请选择完整的日期时间范围'
    return
  }

  const start = new Date(diffStart.value)
  const end = new Date(diffEnd.value)

  if (isNaN(start.getTime()) || isNaN(end.getTime())) {
    diffResultStr.value = '日期时间格式不合法'
    return
  }

  const ms = Math.abs(end.getTime() - start.getTime())
  diffMs.value = ms

  // 折合天/时/分/秒/毫秒
  let temp = ms
  const oneDay = 24 * 60 * 60 * 1000
  const oneHour = 60 * 60 * 1000
  const oneMin = 60 * 1000
  const oneSec = 1000

  const days = Math.floor(temp / oneDay)
  temp %= oneDay

  const hours = Math.floor(temp / oneHour)
  temp %= oneHour

  const mins = Math.floor(temp / oneMin)
  temp %= oneMin

  const secs = Math.floor(temp / oneSec)
  const msecs = temp % oneSec

  diffStats.value = {
    days: parseFloat((ms / oneDay).toFixed(3)),
    hours: parseFloat((ms / oneHour).toFixed(2)),
    mins: parseFloat((ms / oneMin).toFixed(1)),
    secs: Math.floor(ms / oneSec)
  }

  let finalStr = ''
  if (days > 0) finalStr += `${days} 天 `
  if (hours > 0 || days > 0) finalStr += `${hours} 小时 `
  if (mins > 0 || hours > 0 || days > 0) finalStr += `${mins} 分钟 `
  finalStr += `${secs} 秒 ${msecs} 毫秒`

  diffResultStr.value = finalStr
}

function copyText(text: string) {
  navigator.clipboard.writeText(text)
}

onMounted(() => {
  calculateWorkDays()
  calculateDiff()
})
</script>

<template>
  <ToolShell title="时间计算器">
    <template #actions>
      <JcButton size="small" :type="activeTab === 'business-days' ? 'primary' : 'default'" @click="activeTab = 'business-days'">工作日计算</JcButton>
      <JcButton size="small" :type="activeTab === 'date-diff' ? 'primary' : 'default'" @click="activeTab = 'date-diff'">日期时间相差</JcButton>
    </template>

    <div v-if="activeTab === 'business-days'" class="tool-body-split">
      <div class="control-pane">
        <div class="pane-label">设定计算参数</div>

        <div class="field-group">
          <label class="field-label">起始日期</label>
          <input type="date" v-model="startDate" @change="calculateWorkDays" class="date-input-element" />
        </div>

        <div class="field-group">
          <label class="field-label">工作日偏移量（正数往后，负数往前）</label>
          <div class="offset-input-row">
            <input type="number" v-model.number="daysOffset" @input="calculateWorkDays" class="num-input-element" placeholder="如 3" />
            <span class="offset-unit">工作日</span>
          </div>
        </div>

        <JcButton type="primary" block @click="calculateWorkDays">立即计算</JcButton>
      </div>

      <div class="result-display-pane">
        <div class="pane-label">计算目标日期</div>
        <div class="result-show-box">
          <div class="calculated-date">{{ workDayResult }}</div>
          <JcButton size="small" :disabled="!workDayResult" @click="copyText(workDayResult.split(' ')[0])">复制 YYYY-MM-DD</JcButton>
        </div>
        <div class="alert-tip-info">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>
          注意：本计算仅单纯过滤周六日周末，并未引入各国家地区的法定节假日与调休数据，结果仅供本地开发调试或排程参考。
        </div>
      </div>
    </div>

    <div v-else class="tool-body-split">
      <div class="control-pane">
        <div class="pane-label">输入两个日期时间</div>

        <div class="field-group">
          <label class="field-label">起始时间 (Start Time)</label>
          <input type="datetime-local" v-model="diffStart" @change="calculateDiff" class="date-input-element" />
        </div>

        <div class="field-group">
          <label class="field-label">结束时间 (End Time)</label>
          <input type="datetime-local" v-model="diffEnd" @change="calculateDiff" class="date-input-element" />
        </div>

        <JcButton type="primary" block @click="calculateDiff">立即计算差值</JcButton>
      </div>

      <div class="result-display-pane">
        <div class="pane-label">时间差值折算</div>

        <div class="result-show-box text-left">
          <div class="diff-span-label">相差时长跨度：</div>
          <div class="diff-span-value">{{ diffResultStr }}</div>

          <div class="divider-line"></div>

          <div class="diff-span-label">总毫秒差 (Milliseconds)：</div>
          <div class="diff-span-value code-font">
            {{ diffMs }} ms
            <JcButton size="small" @click="copyText(String(diffMs))">复制</JcButton>
          </div>
        </div>

        <div class="stats-grid">
          <div class="stat-card">
            <span class="stat-num">{{ diffStats.days }}</span>
            <span class="stat-lbl">折合天数 (Days)</span>
          </div>
          <div class="stat-card">
            <span class="stat-num">{{ diffStats.hours }}</span>
            <span class="stat-lbl">折合小时 (Hours)</span>
          </div>
          <div class="stat-card">
            <span class="stat-num">{{ diffStats.mins }}</span>
            <span class="stat-lbl">折合分钟 (Minutes)</span>
          </div>
          <div class="stat-card">
            <span class="stat-num">{{ diffStats.secs }}</span>
            <span class="stat-lbl">总共秒数 (Seconds)</span>
          </div>
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
.control-pane {
  display: flex;
  flex-direction: column;
  flex: 0 0 300px;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 14px;
  border-radius: 4px;
  gap: 12px;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 4px;
  text-transform: uppercase;
  font-weight: 600;
}
.field-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  font-weight: 600;
}
.date-input-element {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-family: inherit;
  font-size: 12px;
  padding: 6px 10px;
  outline: none;
  border-radius: 3px;
  width: 100%;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.offset-input-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.num-input-element {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-family: inherit;
  font-size: 12px;
  padding: 6px 10px;
  outline: none;
  border-radius: 3px;
  flex: 1;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.offset-unit {
  font-size: 11px;
  color: var(--jc-text-secondary);
  white-space: nowrap;
}

.result-display-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 14px;
  border-radius: 4px;
  gap: 12px;
}
.result-show-box {
  background: var(--jc-bg-app);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  padding: 16px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  &.text-left {
    align-items: flex-start;
    justify-content: flex-start;
    text-align: left;
  }
}
.calculated-date {
  font-size: 24px;
  font-weight: 700;
  color: var(--jc-color-success);
}
.alert-tip-info {
  margin-top: auto;
  font-size: 10px;
  color: var(--jc-text-secondary);
  background: rgba(215, 186, 125, 0.08);
  border-left: 3px solid var(--jc-color-warning);
  padding: 8px 12px;
  display: flex;
  gap: 8px;
  align-items: flex-start;
  svg {
    color: var(--jc-color-warning);
    flex-shrink: 0;
    margin-top: 1px;
  }
}

.diff-span-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 2px;
}
.diff-span-value {
  font-size: 18px;
  font-weight: 700;
  color: var(--jc-text-highlight);
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 10px;
  &.code-font {
    font-family: 'Cascadia Code', Consolas, monospace;
    color: var(--jc-color-success);
  }
}
.divider-line {
  width: 100%;
  height: 1;
  border-bottom: 1px solid var(--jc-border-default);
  margin: 6px 0 12px 0;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}
.stat-card {
  background: var(--jc-bg-app);
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  align-items: center;
}
.stat-num {
  font-size: 20px;
  font-weight: 700;
  color: var(--jc-color-accent);
  font-family: 'Cascadia Code', Consolas, monospace;
}
.stat-lbl {
  font-size: 10px;
  color: var(--jc-text-secondary);
  margin-top: 4px;
}
</style>
