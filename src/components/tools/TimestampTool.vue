<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'

const nowSeconds = ref(Math.floor(Date.now() / 1000))
const nowMs = ref(Date.now())
const isTicking = ref(true)

let intervalId: any = null

function startTick() {
  if (intervalId) return
  isTicking.value = true
  intervalId = setInterval(() => {
    nowSeconds.value = Math.floor(Date.now() / 1000)
    nowMs.value = Date.now()
  }, 100)
}

function stopTick() {
  isTicking.value = false
  if (intervalId) {
    clearInterval(intervalId)
    intervalId = null
  }
}

function copyCurrent(value: number) {
  navigator.clipboard.writeText(value.toString())
}

// Convert 1: Timestamp to Date
const inputTimestamp = ref('')
const outputDate = ref('')
function convertToDate() {
  if (!inputTimestamp.value) {
    outputDate.value = ''
    return
  }
  try {
    const val = parseInt(inputTimestamp.value.trim())
    if (isNaN(val)) throw new Error('无效数字')
    
    // 自动适配秒和毫秒（位数大于11位当毫秒）
    const isMs = inputTimestamp.value.trim().length > 11
    const date = new Date(isMs ? val : val * 1000)
    if (isNaN(date.getTime())) throw new Error('无效时间')
    
    // 格式化输出为 YYYY-MM-DD HH:mm:ss.SSS
    const y = date.getFullYear()
    const m = String(date.getMonth() + 1).padStart(2, '0')
    const d = String(date.getDate()).padStart(2, '0')
    const hh = String(date.getHours()).padStart(2, '0')
    const mm = String(date.getMinutes()).padStart(2, '0')
    const ss = String(date.getSeconds()).padStart(2, '0')
    const ms = String(date.getMilliseconds()).padStart(3, '0')
    outputDate.value = `${y}-${m}-${d} ${hh}:${mm}:${ss}.${ms}`
  } catch (e) {
    outputDate.value = '转换失败，请输入有效时间戳'
  }
}

// Convert 2: Date to Timestamp
const inputDateStr = ref('')
const outputTimestampSec = ref('')
const outputTimestampMs = ref('')

function convertToTimestamp() {
  if (!inputDateStr.value) {
    outputTimestampSec.value = ''
    outputTimestampMs.value = ''
    return
  }
  try {
    const parsed = Date.parse(inputDateStr.value.trim())
    if (isNaN(parsed)) throw new Error('无效日期格式')
    outputTimestampSec.value = Math.floor(parsed / 1000).toString()
    outputTimestampMs.value = parsed.toString()
  } catch (e) {
    outputTimestampSec.value = '解析失败'
    outputTimestampMs.value = '解析失败'
  }
}

function fillNow() {
  const date = new Date()
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  const hh = String(date.getHours()).padStart(2, '0')
  const mm = String(date.getMinutes()).padStart(2, '0')
  const ss = String(date.getSeconds()).padStart(2, '0')
  inputDateStr.value = `${y}-${m}-${d} ${hh}:${mm}:${ss}`
  convertToTimestamp()
}

onMounted(() => {
  startTick()
})

onUnmounted(() => {
  stopTick()
})
</script>

<template>
  <ToolShell title="时间戳转换器">
    <div class="tool-body">
      <!-- 实时时钟 -->
      <div class="card now-card">
        <div class="card-title">当前时间</div>
        <div class="now-row">
          <div class="now-item">
            <span class="label">秒 (10位)</span>
            <span class="val-mono">{{ nowSeconds }}</span>
            <JcButton size="small" @click="copyCurrent(nowSeconds)">复制</JcButton>
          </div>
          <div class="now-item">
            <span class="label">毫秒 (13位)</span>
            <span class="val-mono">{{ nowMs }}</span>
            <JcButton size="small" @click="copyCurrent(nowMs)">复制</JcButton>
          </div>
          <JcButton size="small" :danger="isTicking" @click="isTicking ? stopTick() : startTick()">
            {{ isTicking ? '暂停' : '启动' }}
          </JcButton>
        </div>
      </div>

      <!-- 时间戳转日期 -->
      <div class="card">
        <div class="card-title">时间戳 ➔ 本地时间</div>
        <div class="form-row">
          <div class="fld flex-2">
            <label>Unix 时间戳 (秒或毫秒)</label>
            <JcInput v-model="inputTimestamp" @update:model-value="convertToDate" placeholder="如: 1719385623" />
          </div>
          <div class="fld flex-3">
            <label>格式化时间</label>
            <div class="row">
              <JcInput v-model="outputDate" readonly placeholder="等待转换..." style="flex: 1; min-width: 0" />
              <JcButton size="small" @click="copyCurrent(parseInt(outputDate))" :disabled="!outputDate || outputDate.includes('失败')">复制</JcButton>
            </div>
          </div>
        </div>
      </div>

      <!-- 日期转时间戳 -->
      <div class="card">
        <div class="card-title">本地时间 ➔ 时间戳</div>
        <div class="form-row">
          <div class="fld flex-2">
            <label>本地日期时间字符串</label>
            <div class="row">
              <JcInput v-model="inputDateStr" @update:model-value="convertToTimestamp" placeholder="格式: YYYY-MM-DD HH:mm:ss" style="flex: 1; min-width: 0" />
              <JcButton size="small" @click="fillNow">当前时间</JcButton>
            </div>
          </div>
          <div class="fld flex-3">
            <label>转换结果</label>
            <div class="row gap-12">
              <div class="row flex-1">
                <span class="suffix-label">秒 (10位)：</span>
                <JcInput v-model="outputTimestampSec" readonly placeholder="秒" style="flex: 1; min-width: 0" />
              </div>
              <div class="row flex-1">
                <span class="suffix-label">毫秒 (13位)：</span>
                <JcInput v-model="outputTimestampMs" readonly placeholder="毫秒" style="flex: 1; min-width: 0" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped lang="scss">
.tool-body {
  display: flex;
  flex-direction: column;
  gap: 15px;
}
.card {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.now-card {
  border-left: 3px solid var(--jc-color-accent);
}
.card-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  text-transform: uppercase;
}
.now-row {
  display: flex;
  align-items: center;
  gap: 24px;
}
.now-item {
  display: flex;
  align-items: center;
  gap: 8px;
  .label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
  .val-mono {
    font-family: 'Cascadia Code', Consolas, monospace;
    font-size: 16px;
    font-weight: 700;
    color: var(--jc-color-success);
    min-width: 120px;
  }
}
.form-row {
  display: flex;
  gap: 16px;
  width: 100%;
}
.fld {
  display: flex;
  flex-direction: column;
  gap: 4px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}
.flex-2 { flex: 2; }
.flex-3 { flex: 3; }
.row {
  display: flex;
  gap: 6px;
  align-items: center;
}
.gap-12 {
  gap: 12px;
}
.suffix-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  white-space: nowrap;
}
</style>
