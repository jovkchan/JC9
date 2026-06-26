<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const portInput = ref('')
const logList = ref<{ text: string; type: 'info' | 'success' | 'error' }[]>([])
const loading = ref(false)

async function runKill() {
  const p = parseInt(String(portInput.value || '').trim())
  if (!p || p < 1 || p > 65535) {
    logList.value.unshift({ text: '请输入有效端口号 (1-65535)', type: 'error' })
    return
  }
  loading.value = true
  logList.value.unshift({ text: `正在尝试释放端口 ${p}...`, type: 'info' })
  try {
    const result = await invoke<string>('kill_port', { port: p })
    logList.value.unshift({ text: result, type: 'success' })
  } catch (e: any) {
    const msg = typeof e === 'string' ? e : (e?.message || String(e))
    logList.value.unshift({ text: `释放失败: ${msg}`, type: 'error' })
  } finally {
    loading.value = false
  }
}

function clearLogs() {
  logList.value = []
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">端口释放器 (Port Killer)</div>
    </div>
    <div class="tool-body">
      <div class="card form-card">
        <div class="fld">
          <label>目标端口号</label>
          <div class="input-row">
            <input v-model="portInput" type="number" placeholder="如: 8080" @keyup.enter="runKill" :disabled="loading" />
            <button class="tool-btn pri" @click="runKill" :disabled="loading">
              {{ loading ? '正在释放...' : '释放端口' }}
            </button>
            <button class="tool-btn" @click="clearLogs" :disabled="logList.length === 0">清空日志</button>
          </div>
        </div>
      </div>

      <div class="editor-pane">
        <div class="pane-label">执行日志</div>
        <div class="logs-console">
          <div v-for="(log, idx) in logList" :key="idx" class="log-line" :class="log.type">
            <span class="log-time">[{{ new Date().toLocaleTimeString() }}]</span>
            <span class="log-text">{{ log.text }}</span>
          </div>
          <div v-if="logList.length === 0" class="empty-tip">
            等待释放指令...
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
  overflow: hidden;
}
.tool-header {
  margin-bottom: 10px;
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
  gap: 12px;
  flex: 1;
  min-height: 0;
}
.card {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 12px 16px;
}
.fld {
  display: flex;
  flex-direction: column;
  gap: 6px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
    text-transform: uppercase;
  }
}
.input-row {
  display: flex;
  gap: 8px;
}
input {
  flex: 1;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 6px 12px;
  font-size: 13px;
  outline: none;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 6px 16px;
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
  &.pri {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    &:hover {
      background: var(--jc-color-accent-hover);
    }
  }
}
.editor-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  padding: 8px;
  min-height: 0;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 6px;
  text-transform: uppercase;
}
.logs-console {
  flex: 1;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  padding: 8px;
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.log-line {
  line-height: 1.5;
  &.info {
    color: var(--jc-text-primary);
  }
  &.success {
    color: var(--jc-color-success);
  }
  &.error {
    color: var(--jc-color-error);
    background: rgba(244, 71, 71, 0.05);
    padding: 0 4px;
  }
}
.log-time {
  color: var(--jc-text-secondary);
  padding-right: 8px;
}
.empty-tip {
  text-align: center;
  padding: 40px;
  font-size: 12px;
  color: var(--jc-text-secondary);
}
</style>
