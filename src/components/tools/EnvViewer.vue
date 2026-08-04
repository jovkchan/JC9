<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcModal from '@/components/ui/JcModal.vue'

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
  <ToolShell title="系统环境变量" subtitle="进程级">
    <template #actions>
      <JcInput beam glow v-model="filterText" placeholder="搜索变量名或内容..." style="width: 220px" />
      <JcButton type="primary" @click="openAddModal">+ 新增变量</JcButton>
      <JcButton :loading="loading" @click="fetchEnv">{{ loading ? '刷新中...' : '刷新' }}</JcButton>
    </template>

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
            <JcButton size="small" @click="copyValue(k, v)">
              {{ copiedKey === k ? '已复制' : '复制' }}
            </JcButton>
            <JcButton size="small" @click="openEditModal(k, v)">编辑</JcButton>
            <JcButton size="small" danger @click="handleDelete(k)">删除</JcButton>
          </div>
        </div>
        <div v-if="filteredEnvs.length === 0 && !loading" class="empty-tip">
          没有找到匹配的环境变量
        </div>
      </div>
    </div>

    <!-- 新增/编辑模态框 -->
    <JcModal v-model:open="showModal" :title="modalMode === 'add' ? '新增环境变量' : '编辑环境变量'" width="440">
      <div class="fld">
        <label>变量名 (Key)</label>
        <JcInput
          beam glow
          v-model="modalKey"
          placeholder="如: NODE_ENV"
          :disabled="modalMode === 'edit'"
          style="font-family: 'Cascadia Code', Consolas, monospace"
          @keyup.enter="handleSave"
        />
      </div>
      <div class="fld">
        <label>变量值 (Value)</label>
        <JcTextarea v-model="modalValue" beam glow :beam-size-ratio="0.6" placeholder="请输入环境变量的值" :rows="4" mono />
      </div>

      <div v-if="modalError" class="modal-error">
        {{ modalError }}
      </div>

      <template #footer>
        <JcButton @click="showModal = false">取消</JcButton>
        <JcButton type="primary" @click="handleSave">保存</JcButton>
      </template>
    </JcModal>
  </ToolShell>
</template>

<style scoped lang="scss">
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
.empty-tip {
  text-align: center;
  padding: 40px;
  font-size: 12px;
  color: var(--jc-text-secondary);
}

// 模态框样式
.modal-error {
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid var(--jc-color-error);
  color: var(--jc-color-error);
  padding: 8px 12px;
  border-radius: 3px;
  font-size: 11px;
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
}
</style>
