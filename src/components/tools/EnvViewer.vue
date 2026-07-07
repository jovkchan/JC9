<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const envs = ref<[string, string][]>([])
const filterText = ref('')
const loading = ref(false)
const copiedKey = ref('')

// 弹窗表单状态
const showModal = ref(false)
const modalMode = ref<'add' | 'edit'>('add')
const modalKey = ref('')
const modalValue = ref('')
const modalError = ref('')

async function fetchEnv() {
  loading.value = true
  try {
    envs.value = await invoke<[string, string][]>('get_env_vars')
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

const filteredEnvs = computed(() => {
  const query = filterText.value.toLowerCase().trim()
  if (!query) return envs.value
  return envs.value.filter(([k, v]) => {
    return k.toLowerCase().includes(query) || v.toLowerCase().includes(query)
  })
})

function copyValue(key: string, value: string) {
  navigator.clipboard.writeText(value)
  copiedKey.value = key
  setTimeout(() => {
    if (copiedKey.value === key) {
      copiedKey.value = ''
    }
  }, 1000)
}

function openAddModal() {
  modalMode.value = 'add'
  modalKey.value = ''
  modalValue.value = ''
  modalError.value = ''
  showModal.value = true
}

function openEditModal(key: string, value: string) {
  modalMode.value = 'edit'
  modalKey.value = key
  modalValue.value = value
  modalError.value = ''
  showModal.value = true
}

async function handleSave() {
  const k = modalKey.value.trim()
  const v = modalValue.value
  if (!k) {
    modalError.value = '变量名 (Key) 不能为空'
    return
  }
  
  try {
    await invoke('set_env_var', { key: k, value: v })
    showModal.value = false
    await fetchEnv()
  } catch (err: any) {
    modalError.value = err.toString()
  }
}

async function handleDelete(key: string) {
  if (confirm(`确定要删除进程环境变量 "${key}" 吗？\n该操作仅对当前应用运行期及其后续子进程生效。`)) {
    try {
      await invoke('remove_env_var', { key })
      await fetchEnv()
    } catch (err: any) {
      alert(`删除失败: ${err}`)
    }
  }
}

onMounted(() => {
  fetchEnv()
})
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">系统环境变量 (进程级)</div>
      <div class="tool-actions">
        <input v-model="filterText" placeholder="搜索变量名或内容..." class="filter-input" />
        <button class="tool-btn pri" @click="openAddModal">
          + 新增变量
        </button>
        <button class="tool-btn" @click="fetchEnv" :disabled="loading">
          {{ loading ? '刷新中...' : '刷新' }}
        </button>
      </div>
    </div>
    
    <div class="tool-body-table">
      <div class="table-header-row">
        <div class="col-key">变量名 (Key)</div>
        <div class="col-val">变量值 (Value)</div>
        <div class="col-act">操作</div>
      </div>
      <div class="table-body">
        <div v-for="[k, v] in filteredEnvs" :key="k" class="table-row">
          <div class="col-key" :title="k">{{ k }}</div>
          <div class="col-val" :title="v">{{ v }}</div>
          <div class="col-act">
            <button class="row-btn" @click="copyValue(k, v)">
              {{ copiedKey === k ? '已复制' : '复制' }}
            </button>
            <button class="row-btn" @click="openEditModal(k, v)">
              编辑
            </button>
            <button class="row-btn danger" @click="handleDelete(k)">
              删除
            </button>
          </div>
        </div>
        <div v-if="filteredEnvs.length === 0 && !loading" class="empty-tip">
          没有找到匹配的环境变量
        </div>
      </div>
    </div>

    <!-- 新增/编辑模态框 -->
    <Teleport to="body">
      <div v-if="showModal" class="mbg" @mousedown.self="showModal = false">
        <div class="mw">
          <div class="mt">{{ modalMode === 'add' ? '新增环境变量' : '编辑环境变量' }}</div>
          <div class="mb">
            <div class="fld">
              <label>变量名 (Key)</label>
              <input 
                v-model="modalKey" 
                placeholder="如: NODE_ENV" 
                :disabled="modalMode === 'edit'" 
                :class="{ 'readonly-input': modalMode === 'edit' }"
                @keyup.enter="handleSave"
                autofocus
              />
            </div>
            <div class="fld">
              <label>变量值 (Value)</label>
              <textarea 
                v-model="modalValue" 
                placeholder="请输入环境变量的值" 
                rows="4"
                class="value-textarea"
              ></textarea>
            </div>
            
            <div v-if="modalError" class="modal-error">
              {{ modalError }}
            </div>

            <div class="acts">
              <button class="tool-btn" @click="showModal = false">取消</button>
              <button class="tool-btn pri" @click="handleSave">保存</button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>
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
}
.filter-input {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 4px 8px;
  font-size: 12px;
  width: 220px;
  outline: none;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: 1px solid var(--jc-border-strong);
  padding: 4px 12px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 3px;
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
    border-color: var(--jc-color-accent);
    &:hover {
      background: var(--jc-color-accent-hover, #007acc);
    }
  }
}
.tool-body-table {
  display: flex;
  flex-direction: column;
  flex: 1;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  min-height: 0;
  overflow: hidden;
}
.table-header-row {
  display: flex;
  background: var(--jc-bg-elevated);
  border-bottom: 1px solid var(--jc-border-default);
  font-size: 11px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  padding: 6px 12px;
  flex-shrink: 0;
}
.table-body {
  flex: 1;
  overflow-y: auto;
}
.table-row {
  display: flex;
  padding: 6px 12px;
  border-bottom: 1px solid var(--jc-border-default);
  font-size: 12px;
  align-items: center;
  &:hover {
    background: var(--jc-bg-hover);
  }
}
.col-key {
  width: 25%;
  font-family: 'Cascadia Code', Consolas, monospace;
  word-break: break-all;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding-right: 8px;
}
.col-val {
  width: 55%;
  font-family: 'Cascadia Code', Consolas, monospace;
  color: var(--jc-text-secondary);
  word-break: break-all;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding-right: 8px;
}
.col-act {
  width: 20%;
  text-align: right;
  display: flex;
  justify-content: flex-end;
  gap: 4px;
}
.row-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: 1px solid var(--jc-border-strong);
  padding: 2px 8px;
  font-size: 10px;
  cursor: pointer;
  border-radius: 3px;
  transition: all 0.2s;
  &:hover {
    background: var(--jc-bg-btn-hover);
    border-color: var(--jc-color-accent);
  }
  
  &.danger {
    &:hover {
      background: var(--jc-color-error);
      color: var(--jc-color-white);
      border-color: var(--jc-color-error);
    }
  }
}
.empty-tip {
  text-align: center;
  padding: 40px;
  font-size: 12px;
  color: var(--jc-text-secondary);
}

// 模态框样式
.mbg {
  position: fixed;
  inset: 0;
  background: var(--jc-bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}
.mw {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  min-width: 450px;
  max-width: 90%;
  box-shadow: var(--jc-shadow-modal);
  border-radius: 4px;
}
.mt {
  background: var(--jc-bg-panel);
  padding: 12px 16px;
  font-size: 13px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  border-bottom: 1px solid var(--jc-border-default);
}
.mb {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.fld {
  display: flex;
  flex-direction: column;
  gap: 6px;
  
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
    font-weight: 600;
  }
  
  input {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-strong);
    color: var(--jc-text-primary);
    padding: 6px 10px;
    font-size: 12px;
    outline: none;
    border-radius: 3px;
    font-family: 'Cascadia Code', Consolas, monospace;
    
    &:focus {
      border-color: var(--jc-color-accent);
    }
    
    &.readonly-input {
      background: var(--jc-bg-app);
      color: var(--jc-text-secondary);
      cursor: not-allowed;
    }
  }
}

.value-textarea {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 6px 10px;
  font-size: 12px;
  outline: none;
  border-radius: 3px;
  font-family: 'Cascadia Code', Consolas, monospace;
  resize: vertical;
  
  &:focus {
    border-color: var(--jc-color-accent);
  }
}

.modal-error {
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid var(--jc-color-error);
  color: var(--jc-color-error);
  padding: 8px 12px;
  border-radius: 3px;
  font-size: 11px;
}

.acts {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}
</style>
