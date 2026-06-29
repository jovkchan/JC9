<script setup lang="ts">
import { ref, computed } from 'vue'

const WEEKDAY_LABELS = ['一', '二', '三', '四', '五', '六', '日']
const MONTH_NAMES = ['一月','二月','三月','四月','五月','六月','七月','八月','九月','十月','十一月','十二月']

const props = defineProps<{
  notes: Array<{ createdAt: string; updatedAt?: string }>
}>()

const emit = defineEmits<{
  selectDate: [date: string | null]
}>()

const now = new Date()
const viewYear = ref(now.getFullYear())
const viewMonth = ref(now.getMonth())
const selectedDate = ref<string | null>(null)
const todayStr = `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')}`

const dayCounts = computed(() => {
  const map: Record<string, number> = {}
  for (const n of props.notes) {
    const d = (n.updatedAt || n.createdAt).slice(0, 10)
    map[d] = (map[d] || 0) + 1
  }
  return map
})

interface DayInfo { date: string; day: number; isCurrentMonth: boolean; isToday: boolean; count: number }

const monthGrid = computed(() => {
  const weeks: DayInfo[][] = []
  const firstDay = new Date(viewYear.value, viewMonth.value, 1)
  const lastDay = new Date(viewYear.value, viewMonth.value + 1, 0)
  const totalDays = lastDay.getDate()
  let startDow = firstDay.getDay() - 1
  if (startDow < 0) startDow = 6

  let day = 1
  let prevDay = new Date(viewYear.value, viewMonth.value, 0).getDate() - startDow + 1
  const prevMonth = viewMonth.value === 0 ? 11 : viewMonth.value - 1
  const prevYear = viewMonth.value === 0 ? viewYear.value - 1 : viewYear.value
  const nextMonth = viewMonth.value === 11 ? 0 : viewMonth.value + 1
  const nextYear = viewMonth.value === 11 ? viewYear.value + 1 : viewYear.value

  for (let w = 0; w < 6; w++) {
    const week: DayInfo[] = []
    for (let d = 0; d < 7; d++) {
      if (w === 0 && d < startDow) {
        const ds = `${prevYear}-${String(prevMonth+1).padStart(2,'0')}-${String(prevDay).padStart(2,'0')}`
        week.push({ date: ds, day: prevDay, isCurrentMonth: false, isToday: false, count: dayCounts.value[ds] || 0 })
        prevDay++
      } else if (day > totalDays) {
        const nd = day - totalDays
        const ds = `${nextYear}-${String(nextMonth+1).padStart(2,'0')}-${String(nd).padStart(2,'0')}`
        week.push({ date: ds, day: nd, isCurrentMonth: false, isToday: false, count: dayCounts.value[ds] || 0 })
        day++
      } else {
        const ds = `${viewYear.value}-${String(viewMonth.value+1).padStart(2,'0')}-${String(day).padStart(2,'0')}`
        week.push({ date: ds, day, isCurrentMonth: true, isToday: ds === todayStr, count: dayCounts.value[ds] || 0 })
        day++
      }
    }
    weeks.push(week)
    if (day > totalDays && w >= 3) break
  }
  return weeks
})

function prevMonth() {
  if (viewMonth.value === 0) { viewMonth.value = 11; viewYear.value-- }
  else viewMonth.value--
}
function nextMonth() {
  if (viewMonth.value === 11) { viewMonth.value = 0; viewYear.value++ }
  else viewMonth.value++
}
function goToday() {
  viewYear.value = now.getFullYear(); viewMonth.value = now.getMonth()
  handleSelect(todayStr)
}
function handleSelect(date: string) {
  if (selectedDate.value === date) { selectedDate.value = null; emit('selectDate', null) }
  else { selectedDate.value = date; emit('selectDate', date) }
}

const selectedInfo = computed(() => selectedDate.value
  ? { date: selectedDate.value, count: dayCounts.value[selectedDate.value] || 0 }
  : null
)
const totalNotes = computed(() => props.notes.length)
</script>

<template>
  <div class="calendar">
    <div class="cal-header">
      <button class="cal-nav-btn" @click="prevMonth" title="上月">◀</button>
      <span class="cal-month-title" @click="goToday">{{ viewYear }}年 {{ MONTH_NAMES[viewMonth] }}</span>
      <button class="cal-nav-btn" @click="nextMonth" title="下月">▶</button>
    </div>

    <div class="cal-weekdays">
      <span v-for="d in WEEKDAY_LABELS" :key="d" class="cal-wd">{{ d }}</span>
    </div>

    <div class="cal-grid">
      <template v-for="(week, wi) in monthGrid" :key="wi">
        <div
          v-for="day in week"
          :key="day.date"
          class="cal-day"
          :class="{
            today: day.isToday,
            'other-month': !day.isCurrentMonth,
            selected: selectedDate === day.date,
            'has-notes': day.count > 0,
          }"
          @click="day.isCurrentMonth && handleSelect(day.date)"
          :title="`${day.date}: ${day.count} 篇笔记`"
        >
          <span class="cal-day-num">{{ day.day }}</span>
          <span v-if="day.count > 0 && day.isCurrentMonth" class="cal-day-dot"></span>
        </div>
      </template>
    </div>

    <div class="cal-footer" v-if="selectedInfo">
      <span class="cal-sel-info">{{ selectedInfo.date }} · {{ selectedInfo.count }} 篇</span>
      <button class="cal-clear-btn" @click="handleSelect(selectedDate!)">清除</button>
    </div>
    <div class="cal-footer" v-else>
      <span class="cal-sel-info">{{ totalNotes }} 篇笔记</span>
      <button class="cal-clear-btn" @click="goToday">今天</button>
    </div>
  </div>
</template>

<style scoped lang="scss">
.calendar {
  padding: 10px 10px 8px;
  border-bottom: 1px solid var(--jc-border-default);
  user-select: none;
}
.cal-header {
  display: flex; align-items: center; justify-content: center; gap: 8px; margin-bottom: 8px;
}
.cal-nav-btn {
  background: none; border: none; color: var(--jc-text-secondary); font-size: 10px;
  padding: 2px 6px; cursor: pointer; border-radius: 3px;
  &:hover { color: var(--jc-text-primary); background: var(--jc-bg-hover); }
}
.cal-month-title {
  font-size: 13px; font-weight: 600; color: var(--jc-text-highlight);
  cursor: pointer; min-width: 120px; text-align: center;
  &:hover { color: var(--jc-color-accent); }
}
.cal-weekdays {
  display: grid; grid-template-columns: repeat(7, 1fr); margin-bottom: 2px;
}
.cal-wd {
  text-align: center; font-size: 10px; color: var(--jc-text-secondary); padding: 2px;
}
.cal-grid {
  display: grid; grid-template-columns: repeat(7, 1fr); gap: 1px;
}
.cal-day {
  aspect-ratio: 1;
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1px;
  cursor: pointer; border-radius: 3px; font-size: 12px; color: var(--jc-text-primary);
  &:hover { background: var(--jc-bg-hover); }
  &.other-month { color: var(--jc-text-secondary); opacity: 0.3; cursor: default; }
  &.today { background: var(--jc-color-accent); color: #fff; font-weight: 700; }
  &.selected { outline: 1.5px solid var(--jc-color-accent); outline-offset: -1px; }
  &.has-notes:not(.today) { color: var(--jc-color-success); }
  &.today .cal-day-dot { background: #fff; }
}
.cal-day-num { line-height: 1; }
.cal-day-dot {
  width: 4px; height: 4px; border-radius: 50%; background: var(--jc-color-success);
}
.cal-footer {
  display: flex; justify-content: space-between; align-items: center; margin-top: 6px;
}
.cal-sel-info { font-size: 10px; color: var(--jc-text-secondary); }
.cal-clear-btn {
  background: none; border: none; color: var(--jc-color-accent); font-size: 10px;
  cursor: pointer; padding: 0 4px;
  &:hover { text-decoration: underline; }
}
</style>
