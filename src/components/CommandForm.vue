<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useProjectStore } from '@/stores/project'
import { open } from '@tauri-apps/plugin-dialog'
import type { Command } from '@/types'

const props = defineProps<{
  projectId: string
  editing?: Command | null
}>()

const emit = defineEmits<{ close: [] }>()

const store = useProjectStore()

const name = ref('')
const command = ref('')
const workingDir = ref('')
const showForm = ref(false)

async function pickDirectory() {
  const selected = await open({ directory: true, multiple: false, title: '选择工作目录' })
  if (selected && typeof selected === 'string') {
    workingDir.value = selected
  }
}

onMounted(() => {
  if (props.editing) {
    name.value = props.editing.name
    command.value = props.editing.command
    workingDir.value = props.editing.workingDir
    showForm.value = true
  }
})

function handleSubmit() {
  const n = name.value.trim()
  const c = command.value.trim()
  const d = workingDir.value.trim()
  if (!n || !c) return

  if (props.editing) {
    store.updateCommand(props.projectId, {
      ...props.editing,
      name: n,
      command: c,
      workingDir: d,
    })
    emit('close')
  } else {
    store.addCommand(props.projectId, { name: n, command: c, workingDir: d })
  }

  name.value = ''
  command.value = ''
  workingDir.value = ''
  showForm.value = false
}

function cancel() {
  if (props.editing) {
    emit('close')
  } else {
    showForm.value = false
    name.value = ''
    command.value = ''
    workingDir.value = ''
  }
}
</script>

<template>
  <div>
    <button
      v-if="!editing && !showForm"
      class="btn-secondary add-btn"
      @click="showForm = true"
    >
      + 添加启动命令
    </button>

    <div v-if="showForm || editing" class="cmd-form">
      <div class="form-row">
        <input v-model="name" placeholder="命令名称 (如: 启动前端)" class="name-input" />
        <div class="dir-input-wrap">
          <input v-model="workingDir" placeholder="工作目录 (如: D:\\code\\my-project)" class="dir-input" />
          <button type="button" class="btn-secondary btn-sm dir-btn" @click="pickDirectory" title="选择文件夹">
            📂
          </button>
        </div>
      </div>
      <div class="form-row">
        <input
          v-model="command"
          placeholder="启动命令 (如: npm run dev)"
          class="cmd-input"
          @keyup.enter="handleSubmit"
        />
      </div>
      <div class="form-actions">
        <button class="btn-primary btn-sm" @click="handleSubmit">
          {{ editing ? '保存' : '添加' }}
        </button>
        <button class="btn-secondary btn-sm" @click="cancel">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.add-btn {
  width: 100%;
  padding: 12px;
  border-style: dashed;
  color: var(--text-secondary);
  font-size: 13px;
}

.add-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
}

.cmd-form {
  padding: 12px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-row {
  display: flex;
  gap: 8px;
}

.name-input {
  flex: 2;
}

.dir-input {
  flex: 3;
}

.dir-input-wrap {
  flex: 3;
  display: flex;
  gap: 4px;
}

.dir-input-wrap .dir-input {
  flex: 1;
  min-width: 0;
}

.dir-btn {
  flex-shrink: 0;
  padding: 4px 8px;
  font-size: 14px;
}

.cmd-input {
  width: 100%;
  font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
}

.form-actions {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
}
</style>
