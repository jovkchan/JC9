<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useProjectStore } from '@/stores/project'
import '@xterm/xterm/css/xterm.css'

const props = defineProps<{ processId: string; active: boolean }>()
const store = useProjectStore()
const container = ref<HTMLDivElement>()
const input = ref(''); const inputRef = ref<HTMLInputElement>()
function focusTerm(){ term?.focus() }
const history: string[] = []; let hi = -1
let term: Terminal|null=null; let fit: FitAddon|null=null; let ul: (()=>void)|null=null; let ro: ResizeObserver|null=null

function doFit(){setTimeout(()=>{if(!container.value||container.value.clientWidth===0||container.value.clientHeight===0)return;try{fit?.fit();if(fit && term){const r=term.rows??24;const c=term.cols??80;invoke('pty_resize',{processId:props.processId,rows:r,cols:c}).catch(()=>{})}}catch(e){}},30)}
function send(d:string){invoke('pty_write',{processId:props.processId,data:Array.from(new TextEncoder().encode(d))}).catch(()=>{})}
function sendLine(){const t=input.value;if(!t)return;send(t+'\r\n');if(history.length===0||history[history.length-1]!==t)history.push(t);hi=history.length;input.value=''}
function onKd(e:KeyboardEvent){
  if(e.key==='ArrowUp'){e.preventDefault();if(!history.length)return;if(hi<=0)hi=0;else hi--;input.value=history[hi]}
  else if(e.key==='ArrowDown'){e.preventDefault();if(hi<history.length-1){hi++;input.value=history[hi]}else{hi=history.length;input.value=''}}
}

onMounted(async()=>{
  if(!container.value)return
  const cs = getComputedStyle(document.documentElement)
  term=new Terminal({cursorBlink:true,fontSize:13,disableStdin:false,fontFamily:"'Microsoft YaHei Mono','Cascadia Code','Consolas',monospace",theme:{
    background: cs.getPropertyValue('--jc-term-bg').trim() || '#1e1e1e',
    foreground: cs.getPropertyValue('--jc-term-fg').trim() || '#ccc',
    cursor: cs.getPropertyValue('--jc-color-accent').trim() || '#8a58ff',
    selectionBackground: cs.getPropertyValue('--jc-term-selection').trim() || '#264f78',
  }})
  fit=new FitAddon();term.loadAddon(fit);term.open(container.value)
  if(props.active)doFit()
  const buf=store.getOutput(props.processId);if(buf.length>0){term.write(new Uint8Array(buf));term.scrollToBottom()}
  ul=await listen<{processId:string;data:number[]}>('pty-output',e=>{if(e.payload.processId!==props.processId)return;if(e.payload.data.length>0){term?.write(new Uint8Array(e.payload.data));term?.scrollToBottom()}})
  ro=new ResizeObserver(()=>doFit());ro.observe(container.value)
  // Direct keyboard -> PTY
  term.onData(data => send(data))
  // Focus the terminal directly
  setTimeout(()=>{
    term?.focus()
  }, 200)
})
watch(()=>props.active,v=>{if(v){doFit();setTimeout(()=>{term?.focus()},100)}})
watch(()=>store.clearTermSignal,()=>{term?.scrollToBottom();term?.write('\u001b[2J\u001b[3J\u001b[H');term?.scrollToBottom()})
watch(()=>store.pendingInput,v=>{if(v){input.value=v;store.pendingInput='';nextTick(()=>inputRef.value?.focus())}})
onUnmounted(()=>{ul?.();ro?.disconnect();term?.dispose();})
</script>

<template>
  <div class="tw">
    <div ref="container" class="to" @click="focusTerm" />
    <div class="tb">
      <span class="p">>_</span>
      <input ref="inputRef" v-model="input" class="ti" placeholder="回车发送 | 点终端区域交互选择 ↑↓" @keyup.enter="sendLine" @keydown="onKd" spellcheck="false" />
    </div>
  </div>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;
.tw { flex:1; display:flex; flex-direction:column; overflow:hidden; }
.to { flex:1; overflow:hidden; padding:4px; margin:4px; border:1px solid var(--jc-border-default); background:var(--jc-term-bg);
  &:deep(.xterm) { height:100%; }
  &:deep(.xterm-viewport) { scrollbar-width:thin; }
}
.tb { display:flex; align-items:center; gap:6px; padding:6px 12px; background:var(--jc-bg-elevated); border-top:1px solid var(--jc-border-default); }
.p { color:var(--jc-color-success); font-family:'Cascadia Code',Consolas,monospace; font-size:13px; font-weight:700; }
.ti { flex:1; @include input-base; font-family:'Cascadia Code',Consolas,monospace; font-size:13px; padding:3px 8px;
  &:focus { border-color:var(--jc-color-accent); outline:none; }
}
</style>
