<script setup lang="ts">
import { ref } from 'vue'

const count = ref(5)
const hyphens = ref(true)
const uppercase = ref(false)
const output = ref('')

function generateUuid() {
  const list: string[] = []
  for (let i = 0; i < count.value; i++) {
    // 纯前端产生 UUID v4 (对齐 crypto.randomUUID() 支持)
    let uuid = ''
    try {
      uuid = crypto.randomUUID()
    } catch {
      // 降级后备算法
      uuid = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
        const r = (Math.random() * 16) | 0
        const v = c === 'x' ? r : (r & 0x3) | 0x8
        return v.toString(16)
      })
    }

    if (!hyphens.value) {
      uuid = uuid.replace(/-/g, '')
    }
    if (uppercase.value) {
      uuid = uuid.toUpperCase()
    } else {
      uuid = uuid.toLowerCase()
    }
    list.push(uuid)
  }
  output.value = list.join('\n')
}

function copyAll() {
  if (!output.value) return
  navigator.clipboard.writeText(output.value)
}

function clearAll() {
  output.value = ''
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">UUID 生成器 (v4)</div>
    </div>
    <div class="tool-body">
      <div class="card form-card">
        <div class="form-row">
          <div class="fld">
            <label>生成数量</label>
            <input v-model.number="count" type="number" min="1" max="1000" class="num-input" />
          </div>
          <div class="fld checkbox-fld">
            <label class="checkbox-label">
              <input v-model="hyphens" type="checkbox" />
              <span>保留连字符 (-)</span>
            </label>
          </div>
          <div class="fld checkbox-fld">
            <label class="checkbox-label">
              <input v-model="uppercase" type="checkbox" />
              <span>大写格式</span>
            </label>
          </div>
          <div class="btn-group">
            <button class="tool-btn pri" @click="generateUuid">批量生成</button>
            <button class="tool-btn" @click="copyAll" :disabled="!output">复制全部</button>
            <button class="tool-btn err" @click="clearAll" :disabled="!output">清空</button>
          </div>
        </div>
      </div>

      <div class="editor-pane">
        <div class="pane-label">生成的 UUID 列表</div>
        <textarea v-model="output" readonly placeholder="等待生成..." class="readonly-output"></textarea>
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
.form-row {
  display: flex;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}
.fld {
  display: flex;
  flex-direction: column;
  gap: 4px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
    text-transform: uppercase;
  }
}
.num-input {
  width: 80px;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 4px 8px;
  font-size: 12px;
  outline: none;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.checkbox-fld {
  justify-content: center;
  padding-top: 14px;
}
.checkbox-label {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 12px;
  color: var(--jc-text-primary);
  input[type="checkbox"] {
    cursor: pointer;
    width: 14px;
    height: 14px;
  }
}
.btn-group {
  display: flex;
  gap: 8px;
  padding-top: 14px;
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
  &.err {
    &:hover {
      background: var(--jc-color-error);
      color: var(--jc-color-white);
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
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.readonly-output {
  background: var(--jc-bg-input);
  color: var(--jc-color-success);
}
</style>
