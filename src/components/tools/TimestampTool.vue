<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

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
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">时间戳转换器</div>
    </div>
    <div class="tool-body">
      <!-- 实时时钟 -->
      <div class="card now-card">
        <div class="card-title">当前时间</div>
        <div class="now-row">
          <div class="now-item">
            <span class="label">秒 (10位)</span>
            <span class="val-mono">{{ nowSeconds }}</span>
            <button class="now-copy-btn" @click="copyCurrent(nowSeconds)">复制</button>
          </div>
          <div class="now-item">
            <span class="label">毫秒 (13位)</span>
            <span class="val-mono">{{ nowMs }}</span>
            <button class="now-copy-btn" @click="copyCurrent(nowMs)">复制</button>
          </div>
          <button class="tool-btn" :class="{ err: isTicking }" @click="isTicking ? stopTick() : startTick()">
            {{ isTicking ? '暂停' : '启动' }}
          </button>
        </div>
      </div>

      <!-- 时间戳转日期 -->
      <div class="card">
        <div class="card-title">时间戳 ➔ 本地时间</div>
        <div class="form-row">
          <div class="fld flex-2">
            <label>Unix 时间戳 (秒或毫秒)</label>
            <input v-model="inputTimestamp" @input="convertToDate" placeholder="如: 1719385623" />
          </div>
          <div class="fld flex-3">
            <label>格式化时间</label>
            <div class="row">
              <input v-model="outputDate" readonly placeholder="等待转换..." class="readonly-output" />
              <button class="tool-btn" @click="copyCurrent(parseInt(outputDate))" :disabled="!outputDate || outputDate.includes('失败')">复制</button>
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
              <input v-model="inputDateStr" @input="convertToTimestamp" placeholder="格式: YYYY-MM-DD HH:mm:ss" />
              <button class="tool-btn" @click="fillNow">当前时间</button>
            </div>
          </div>
          <div class="fld flex-3">
            <label>转换结果</label>
            <div class="row gap-12">
              <div class="row flex-1">
                <span class="suffix-label">秒 (10位)：</span>
                <input v-model="outputTimestampSec" readonly placeholder="秒" class="readonly-output inline-input" />
              </div>
              <div class="row flex-1">
                <span class="suffix-label">毫秒 (13位)：</span>
                <input v-model="outputTimestampMs" readonly placeholder="毫秒" class="readonly-output inline-input" />
              </div>
            </div>
          </div>
        </div>
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
  overflow-y: auto;
}
.tool-header {
  margin-bottom: 15px;
  flex-shrink: 0;
}
.tool-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}
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
.now-copy-btn {
  background: none;
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 2px 8px;
  font-size: 10px;
  cursor: pointer;
  &:hover {
    background: var(--jc-bg-hover);
    border-color: var(--jc-color-accent);
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
input {
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 6px 10px;
  font-size: 12px;
  outline: none;
  font-family: 'Cascadia Code', Consolas, monospace;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.readonly-output {
  background: var(--jc-bg-app);
  color: var(--jc-color-success);
}
.inline-input {
  padding: 4px 8px;
}
.suffix-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  white-space: nowrap;
}
.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 6px 12px;
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
  &:hover:not(:disabled) {
    background: var(--jc-bg-btn-hover);
  }
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  &.err {
    &:hover {
      background: var(--jc-color-error);
      color: var(--jc-color-white);
    }
  }
}
</style>
