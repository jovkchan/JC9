<script setup lang="ts">
import { ref, watch } from 'vue'
import { useProjectStore } from '@/stores/project'
import { open } from '@tauri-apps/plugin-dialog'
import type { Command } from '@/types'
import JcModal from '@/components/ui/JcModal.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcButton from '@/components/ui/JcButton.vue'

const props = defineProps<{ projectId: string; editing: Command | null }>()
const emit = defineEmits<{ close: [] }>()
const store = useProjectStore()
const visible = ref(false)
const name = ref(''); const command = ref(''); const workingDir = ref('')

watch(()=>props.editing,(val)=>{if(val){name.value=val.name;command.value=val.command;workingDir.value=val.workingDir;visible.value=true}},{immediate:true})

function openDialog(){if(!props.editing){name.value='';command.value='';workingDir.value=''}visible.value=true}
async function pickDir(){const d=await open({directory:true,multiple:false,title:'选择工作目录'});if(d&&typeof d==='string')workingDir.value=d}
function save(){const n=name.value.trim();const c=command.value.trim();if(!n||!c)return
  if(props.editing)store.updateCommand(props.projectId,{...props.editing,name:n,command:c,workingDir:workingDir.value.trim()})
  else store.addCommand(props.projectId,{name:n,command:c,workingDir:workingDir.value.trim()})
  close()}
function close(){visible.value=false;emit('close')}
defineExpose({openDialog})
</script>

<template>
  <JcModal v-model:open="visible" :title="editing ? '编辑命令' : '添加命令'" width="440" @cancel="close">
    <div class="mb">
      <div class="fld"><label>命令名称</label><JcInput beam v-model="name" placeholder="如: 启动前端" @keyup.enter="save" autofocus /></div>
      <div class="fld"><label>工作目录</label><div class="row"><JcInput beam v-model="workingDir" placeholder="如: D:\code\my-project" style="flex:1;min-width:0" @keyup.enter="save" /><JcButton @click="pickDir">...</JcButton></div></div>
      <div class="fld"><label>启动命令</label><JcInput beam v-model="command" placeholder="如: npm run dev" @keyup.enter="save" style="font-family:'Cascadia Code',Consolas,monospace" /></div>
    </div>
    <template #footer>
      <JcButton @click="close">取消</JcButton>
      <JcButton type="primary" @click="save">{{ editing ? '保存' : '添加' }}</JcButton>
    </template>
  </JcModal>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;
.mb { padding:16px; display:flex; flex-direction:column; gap:12px; }
.fld { display:flex; flex-direction:column; gap:4px;
  label { font-size:11px; color:var(--jc-text-secondary); text-transform:uppercase; letter-spacing:.5px; }
}
.row { display:flex; gap:6px; }
</style>
