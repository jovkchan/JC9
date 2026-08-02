<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'

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
  <ToolShell title="端口释放器" subtitle="Port Killer">
    <div class="card form-card">
      <div class="fld">
        <label>目标端口号</label>
        <div class="input-row">
          <input v-model="portInput" type="number" placeholder="如: 8080" @keyup.enter="runKill" :disabled="loading" />
          <JcButton type="primary" :loading="loading" @click="runKill">
            {{ loading ? '正在释放...' : '释放端口' }}
          </JcButton>
          <JcButton :disabled="logList.length === 0" @click="clearLogs">清空日志</JcButton>
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
  </ToolShell>
</template>

<style scoped lang="scss">
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
