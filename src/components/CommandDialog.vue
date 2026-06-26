<script setup lang="ts">
import { ref, watch } from 'vue'
import { useProjectStore } from '@/stores/project'
import { open } from '@tauri-apps/plugin-dialog'
import type { Command } from '@/types'

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
  <Teleport to="body">
    <div v-if="visible" class="mbg" @click.self="close">
      <div class="mw">
        <div class="mt">{{ editing ? '编辑命令' : '添加命令' }}</div>
        <div class="mb">
          <div class="fld"><label>命令名称</label><input v-model="name" placeholder="如: 启动前端" @keyup.enter="save" autofocus /></div>
          <div class="fld"><label>工作目录</label><div class="row"><input v-model="workingDir" placeholder="如: D:\code\my-project" style="flex:1;min-width:0" @keyup.enter="save" /><button class="btn" @click="pickDir">...</button></div></div>
          <div class="fld"><label>启动命令</label><input v-model="command" placeholder="如: npm run dev" @keyup.enter="save" style="font-family:'Cascadia Code',Consolas,monospace" /></div>
          <div class="acts"><button class="btn" @click="close">取消</button><button class="btn pri" @click="save">{{ editing?'保存':'添加' }}</button></div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.mbg { position:fixed; inset:0; background:rgba(0,0,0,.5); display:flex; align-items:center; justify-content:center; z-index:1000; }
.mw { background:#2d2d30; border:1px solid #555; min-width:440px; box-shadow:0 8px 24px rgba(0,0,0,.6); }
.mt { background:#252526; padding:10px 16px; font-size:14px; font-weight:600; color:#e0e0e0; border-bottom:1px solid #3e3e42; }
.mb { padding:16px; display:flex; flex-direction:column; gap:12px; }
.fld { display:flex; flex-direction:column; gap:4px; }
.fld label { font-size:11px; color:#858585; text-transform:uppercase; letter-spacing:.5px; }
.fld input { background:#3c3c3c; border:1px solid #555; padding:6px 10px; color:#ccc; font-size:13px; }
.fld input:focus { border-color:#007acc; }
.row { display:flex; gap:6px; }
.btn { background:#3c3c3c; color:#ccc; padding:5px 16px; font-size:12px; }
.btn:hover { background:#4c4c4c; }
.btn.pri { background:#007acc; color:#fff; }
.btn.pri:hover { background:#1a8ad4; }
.acts { display:flex; justify-content:flex-end; gap:8px; margin-top:4px; }
</style>
