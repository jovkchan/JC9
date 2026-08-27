<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useProjectStore } from '@/stores/project'
import JcContextMenu from '@/components/ui/JcContextMenu.vue'
import JcInput from '@/components/ui/JcInput.vue'
import type { JcContextMenuItem } from '@/components/ui'
import '@xterm/xterm/css/xterm.css'

const props = defineProps<{ processId: string; active: boolean }>()
const store = useProjectStore()
const container = ref<HTMLDivElement>()
const input = ref(''); const inputRef = ref<InstanceType<typeof JcInput>>()
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
const ctxItems: JcContextMenuItem[] = [
  { label: '复制', icon: '📋', value: 'copy' },
  { label: '粘贴', icon: '📥', value: 'paste' },
]
function onCtxSelect(item: JcContextMenuItem) {
  if (item.value === 'copy') doCopy()
  else if (item.value === 'paste') doPaste()
}

function doFit(){setTimeout(()=>{if(!container.value||container.value.clientWidth===0||container.value.clientHeight===0)return;try{fit?.fit();if(fit && term){const r=term.rows??24;const c=term.cols??80;invoke('pty_resize',{processId:props.processId,rows:r,cols:c}).catch(()=>{})}}catch(e){}},30)}
function send(d:string){invoke('pty_write',{processId:props.processId,data:Array.from(new TextEncoder().encode(d))}).catch(()=>{})}
function sendLine(){const t=input.value;if(!t)return;send(t+'\r\n');if(history.length===0||history[history.length-1]!==t)history.push(t);hi=history.length;input.value=''}
function onKd(e:KeyboardEvent){
  if(e.key==='ArrowUp'){e.preventDefault();if(!history.length)return;if(hi<=0)hi=0;else hi--;input.value=history[hi]}
  else if(e.key==='ArrowDown'){e.preventDefault();if(hi<history.length-1){hi++;input.value=history[hi]}else{hi=history.length;input.value=''}}
}

// ── 明暗主题适配：读取当前 CSS 变量作为 xterm 主题，随 data-theme 实时切换 ──
function readTermTheme() {
  const cs = getComputedStyle(document.documentElement)
  const pick = (name: string, fallback: string) => cs.getPropertyValue(name).trim() || fallback
  const bg = pick('--jc-term-bg', '#1e1e1e')
  return {
    background: bg,
    foreground: pick('--jc-term-fg', '#ccc'),
    cursor: pick('--jc-term-cursor', '#ccc'),
    cursorAccent: bg,
    selectionBackground: pick('--jc-term-selection', '#264f78'),
  }
}
let themeObs: MutationObserver | null = null
function applyTermTheme() {
  if (!term) return
  term.options.theme = readTermTheme()
  term.refresh(0, term.rows - 1)
}

onMounted(async()=>{
  if(!container.value)return
  term=new Terminal({cursorBlink:true,convertEol:true,fontSize:13,disableStdin:false,rightClickSelectsWord:false,fontFamily:"'Microsoft YaHei Mono','Cascadia Code','Consolas',monospace",theme:readTermTheme()})
  fit=new FitAddon();term.loadAddon(fit);term.open(container.value)
  if(props.active)doFit()
  const buf=store.getOutput(props.processId);if(buf.length>0){term.write(new Uint8Array(buf));term.scrollToBottom()}
  // 监听明暗主题切换（html data-theme），实时更新 xterm 主题（与 PTY 无关，独立优先建立）
  themeObs=new MutationObserver((muts)=>{if(muts.some((m)=>m.attributeName==='data-theme'))applyTermTheme()})
  themeObs.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']})
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
onUnmounted(()=>{ul?.();ro?.disconnect();themeObs?.disconnect();term?.dispose();})
</script>

<template>
  <div class="tw">
    <div ref="container" class="to" @click="focusTerm" @contextmenu.prevent="onCtx" />
    <div class="tb">
      <span class="p">>_</span>
      <JcInput ref="inputRef" v-model="input" beam glow style="flex:1;min-width:0" placeholder="回车发送 | 点终端区域交互选择 ↑↓" @keyup.enter="sendLine" @keydown="onKd" />
    </div>
    <!-- 右键菜单 -->
    <JcContextMenu :show="ctxShow" :x="ctxX" :y="ctxY" :items="ctxItems" @select="onCtxSelect" @update:show="ctxShow = $event" />
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
/* 终端命令输入框（JcInput）等宽字体覆盖 */
.tb :deep(.jc-input__inner) { font-family:'Cascadia Code',Consolas,monospace; font-size:13px; }
</style>

