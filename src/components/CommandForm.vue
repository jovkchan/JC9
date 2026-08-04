<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useProjectStore } from '@/stores/project'
import { open } from '@tauri-apps/plugin-dialog'
import type { Command } from '@/types'
import JcInput from '@/components/ui/JcInput.vue'
import JcButton from '@/components/ui/JcButton.vue'

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
        <JcInput beam glow v-model="name" placeholder="命令名称 (如: 启动前端)" style="flex: 2; min-width: 0" />
        <div class="dir-input-wrap">
          <JcInput beam glow v-model="workingDir" placeholder="工作目录 (如: D:\\code\\my-project)" style="flex: 1; min-width: 0" />
          <JcButton size="small" @click="pickDirectory" title="选择文件夹">📂</JcButton>
        </div>
      </div>
      <div class="form-row">
        <JcInput
          beam glow
          v-model="command"
          placeholder="启动命令 (如: npm run dev)"
          @keyup.enter="handleSubmit"
        />
      </div>
      <div class="form-actions">
        <JcButton type="primary" size="small" @click="handleSubmit">
          {{ editing ? '保存' : '添加' }}
        </JcButton>
        <JcButton size="small" @click="cancel">取消</JcButton>
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

.dir-input-wrap {
  flex: 3;
  display: flex;
  gap: 4px;
}

.form-actions {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
}
</style>
