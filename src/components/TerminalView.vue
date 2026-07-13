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

// ── 右键菜单 ──
const ctxShow = ref(false)
const ctxX = ref(0)
const ctxY = ref(0)
function closeCtx() { ctxShow.value = false }
function doCopy() { closeCtx(); if (term?.hasSelection()) navigator.clipboard.writeText(term.getSelection()) }
function doPaste() { closeCtx(); navigator.clipboard.readText().then(t => send(t)) }
function onCtx(e: MouseEvent) { e.preventDefault(); ctxX.value = e.clientX; ctxY.value = e.clientY; ctxShow.value = true }

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
  term=new Terminal({cursorBlink:true,convertEol:true,fontSize:13,disableStdin:false,rightClickSelectsWord:false,fontFamily:"'Microsoft YaHei Mono','Cascadia Code','Consolas',monospace",theme:{
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
  // 右键菜单（xterm 内部 canvas 会拦截 contextmenu，直接绑定到 terminal 元素）
  setTimeout(() => {
    if (term && term.element) term.element.addEventListener('contextmenu', (e: Event) => { e.preventDefault(); onCtx(e as MouseEvent) })
  }, 100)
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
    <div ref="container" class="to" @click="focusTerm" @contextmenu.prevent="onCtx" />
    <div class="tb">
      <span class="p">>_</span>
      <input ref="inputRef" v-model="input" class="ti" placeholder="回车发送 | 点终端区域交互选择 ↑↓" @keyup.enter="sendLine" @keydown="onKd" spellcheck="false" />
    </div>
    <!-- 右键菜单 -->
    <Teleport to="body">
      <div v-if="ctxShow" class="ctx-overlay" @mousedown="closeCtx" @contextmenu.prevent="closeCtx">
        <div class="ctx-menu" :style="{ left: ctxX + 'px', top: ctxY + 'px' }" @mousedown.stop>
          <div class="ctx-item" @click="doCopy"><span class="ctx-icon"><svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M768 832a128 128 0 0 1-128 128H256a128 128 0 0 1-128-128V384a128 128 0 0 1 128-128v576h512z m-128-704c35.346 0 64 28.654 64 64v576a64 64 0 0 1-64 64H320a64 64 0 0 1-64-64V192a64 64 0 0 1 64-64h320z m-64 64H384v512h192V192z"/></svg></span>复制</div>
          <div class="ctx-item" @click="doPaste"><span class="ctx-icon"><svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M704 128h160a32 32 0 0 1 32 32v736a32 32 0 0 1-32 32H160a32 32 0 0 1-32-32V160a32 32 0 0 1 32-32h160a192 192 0 0 1 384 0z m-64 0a128 128 0 0 0-256 0h256zM192 192v640h640V192H704v128H320V192H192z"/></svg></span>粘贴</div>
        </div>
      </div>
    </Teleport>
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

<!-- 右键菜单样式（Teleport 到 body，不能用 scoped） -->
<style lang="scss">
.ctx-overlay { position:fixed; inset:0; z-index:99999; }
.ctx-menu { position:fixed; z-index:100000; min-width:120px; background:var(--jc-bg-elevated); border:1px solid var(--jc-border-default); border-radius:6px; padding:4px; box-shadow:0 4px 16px rgba(0,0,0,.3); }
.ctx-item { display:flex; align-items:center; gap:8px; padding:6px 12px; cursor:pointer; font-size:12px; color:var(--jc-text-primary); border-radius:4px; white-space:nowrap; user-select:none; &:hover { background:var(--jc-bg-hover); } }
.ctx-icon { display:flex; align-items:center; width:16px; justify-content:center; color:var(--jc-text-secondary); }
</style>
