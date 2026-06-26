<script setup lang="ts">
import { ref, nextTick, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '@/stores/project'
import { open } from '@tauri-apps/plugin-dialog'
import CommandDialog from '@/components/CommandDialog.vue'
import type { Command } from '@/types'

const store = useProjectStore()
const activeTab = ref<'projects'|'shortcuts'|'tools'>('projects')
const showAdd = ref(false); const newName = ref(''); const newDir = ref('')
const expandedProjects = ref<Set<string>>(new Set()); const detectedLang = ref('')
const detectedCmds = ref<{name:string;command:string;workingDir:string}[]>([])
const dialogProjectId = ref(''); const editingCmd = ref<Command|null>(null)
const cmdDialogRef = ref<InstanceType<typeof CommandDialog>>()

function toggleExpand(id:string){expandedProjects.value.has(id)?expandedProjects.value.delete(id):expandedProjects.value.add(id)}
async function pickDir(){const d=await open({directory:true,multiple:false,title:'选择项目目录'});if(d&&typeof d==='string'){newDir.value=d;newName.value=d.split(/[\\/]/).pop()||newName.value;const info=await store.detectProject(d);if(info){newName.value=info.name;detectedLang.value=info.lang;detectedCmds.value=info.suggestCommands}}}
async function handleAdd(){const n=newName.value.trim()||'新项目';store.addProject(n);const pid=store.projects[store.projects.length-1].id;expandedProjects.value.add(pid);for(const c of detectedCmds.value)store.addCommand(pid,c);newName.value='';newDir.value='';detectedLang.value='';detectedCmds.value=[];showAdd.value=false}
function addQuickCmd(id:string){editingCmd.value=null;dialogProjectId.value=id;cmdDialogRef.value?.openDialog()}
function editCmd(pid:string,cmd:Command){editingCmd.value=cmd;dialogProjectId.value=pid;cmdDialogRef.value?.openDialog()}
function isRunning(pid:string,cid:string){return store.runningMap[store.cmdKey(pid,cid)]==='running'}

// ---- Project context menu ----
const projCtxShow = ref(false); const projCtxPos = ref({x:0,y:0}); const projCtxId = ref('')
function openProjCtx(e:MouseEvent, pid:string){e.preventDefault();projCtxPos.value={x:e.clientX,y:e.clientY};projCtxId.value=pid;projCtxShow.value=true}
function closeProjCtx(){projCtxShow.value=false}
function ctxRenameProj(){editingProjId.value=projCtxId.value;editProjName.value=store.projects.find(p=>p.id===projCtxId.value)?.name||'';closeProjCtx();nextTick(()=>{const el=document.querySelector<HTMLInputElement>('.proj-edit-input');el?.focus();el?.select()})}
function confirmRenameProj(){const n=editProjName.value.trim();if(n){store.updateProjectName(editingProjId.value,n)};editingProjId.value=''}
function ctxAddCmd(){dialogProjectId.value=projCtxId.value;editingCmd.value=null;cmdDialogRef.value?.openDialog();closeProjCtx()}
function ctxDelProj(){store.removeProject(projCtxId.value);closeProjCtx()}
const editingProjId=ref('');const editProjName=ref('')
document.addEventListener('click',()=>{closeProjCtx();closeCmdCtx()})

// ---- Command context menu ----
const cmdCtxShow = ref(false); const cmdCtxPos = ref({x:0,y:0}); const cmdCtxPid = ref(''); const cmdCtxCmd = ref<Command|null>(null)
function openCmdCtx(e:MouseEvent, pid:string, cmd:Command){e.preventDefault();e.stopPropagation();cmdCtxPos.value={x:e.clientX,y:e.clientY};cmdCtxPid.value=pid;cmdCtxCmd.value=cmd;cmdCtxShow.value=true}
function closeCmdCtx(){cmdCtxShow.value=false}
function ctxEditCmd(){if(cmdCtxCmd.value){dialogProjectId.value=cmdCtxPid.value;editingCmd.value=cmdCtxCmd.value;cmdDialogRef.value?.openDialog()};closeCmdCtx()}
function ctxRenameCmd(){const c=cmdCtxCmd.value;if(c){editingCmdId.value=cmdCtxPid.value+'::'+c.id;editCmdName.value=c.name};closeCmdCtx();nextTick(()=>{const el=document.querySelector<HTMLInputElement>('.cmd-edit-input');el?.focus();el?.select()})}
function confirmRenameCmd(){const [pid,cid]=editingCmdId.value.split('::');const n=editCmdName.value.trim();const p=store.projects.find(p=>p.id===pid);const c=p?.commands.find(c=>c.id===cid);if(n&&c)store.updateCommand(pid,{...c,name:n});editingCmdId.value=''}
const editingCmdId=ref('');const editCmdName=ref('')
function ctxDelCmd(){store.removeCommand(cmdCtxPid.value,cmdCtxCmd.value!.id);closeCmdCtx()}

// ── Tools ──
const activeTool = ref('')
const toolPort = ref('')
const toolMsg = ref('')
const toolBusy = ref(false)
async function runTool() {
  if (activeTool.value === 'port') {
    const p = parseInt(toolPort.value)
    if (!p || p < 1) { toolMsg.value = '请输入有效端口号'; return }
    toolBusy.value = true; toolMsg.value = ''
    try { toolMsg.value = await invoke<string>('kill_port', { port: p }) }
    catch (e: any) { toolMsg.value = typeof e === 'string' ? e : (e?.message || String(e)) }
    toolBusy.value = false
  }
}

// ---- Shortcuts ----
const showScDlg=ref(false);const newScName=ref('');const newScCmd=ref('');const newScDesc=ref('');const newScCat=ref('')
function openScDlg(){showScDlg.value=true;newScName.value='';newScCmd.value='';newScDesc.value='';newScCat.value=''}
const expandedCat = ref('')
const scSearch = ref('')
const filteredCats = computed(() => shortcutCats.value.filter(c => shortcutsByCat(c).some(s => s.command.includes(scSearch.value) || s.description.includes(scSearch.value) || s.name.includes(scSearch.value))))
const filteredFreq = computed(() => store.frequentShortcuts.filter(s => s.command.includes(scSearch.value) || s.description.includes(scSearch.value) || s.name.includes(scSearch.value)))
const filteredFav = computed(() => store.favShortcuts.filter(s => s.command.includes(scSearch.value) || s.description.includes(scSearch.value) || s.name.includes(scSearch.value)))
const shortcutCats=computed(()=>[...new Set(store.shortcuts.map(s=>s.category))])
function shortcutsByCat(cat:string){return store.shortcuts.filter(s=>s.category===cat)}
function addSc(){const n=newScName.value.trim();const c=newScCmd.value.trim();if(!n||!c)return
  if(editingScId.value){store.updateShortcut(editingScId.value,{name:n,command:c,description:newScDesc.value.trim(),category:newScCat.value.trim()||'自定义'})}
  else store.addShortcut({name:n,command:c,description:newScDesc.value.trim(),category:newScCat.value.trim()||'自定义'})
  showScDlg.value=false;editingScId.value=''}
const scTab = ref<'all'|'freq'|'fav'>('all')
// Shortcut context menu
const scCtxShow=ref(false);const scCtxPos=ref({x:0,y:0});const scCtxItem=ref<import('@/stores/project').ShortcutItem|null>(null)
function openScCtx(e:MouseEvent,s:import('@/stores/project').ShortcutItem){e.preventDefault();scCtxPos.value={x:e.clientX,y:e.clientY};scCtxItem.value=s;scCtxShow.value=true}
function closeScCtx(){scCtxShow.value=false}
function scCtxEdit(){const s=scCtxItem.value;if(s){newScName.value=s.name;newScCmd.value=s.command;newScDesc.value=s.description;newScCat.value=s.category;editingScId.value=s.id;showScDlg.value=true};closeScCtx()}
function scCtxDel(){if(scCtxItem.value)store.removeShortcut(scCtxItem.value.id);closeScCtx()}
function scCtxFav(){if(scCtxItem.value)store.toggleFav(scCtxItem.value.id);closeScCtx()}
const editingScId=ref('')
async function scCtxDoc(){const s=scCtxItem.value;if(!s){closeScCtx();return};closeScCtx();store.openDoc(s.command,s.command)}
document.addEventListener('click',()=>{closeScCtx()})
</script>

<template>
  <aside class="side">
    <div class="side-head"></div>
    <div class="tabs">
      <div :class="['tab',{on:activeTab==='projects'}]" @click="activeTab='projects'">项目</div>
      <div :class="['tab',{on:activeTab==='shortcuts'}]" @click="activeTab='shortcuts'">快捷</div>
      <div :class="['tab',{on:activeTab==='tools'}]" @click="activeTab='tools'">工具</div>
    </div>

    <!-- Projects -->
    <div v-show="activeTab==='projects'" class="panel">
      <div class="bar"><button class="btn" @click="showAdd=!showAdd">{{ showAdd?'收起':'+ 添加项目' }}</button></div>
      <div v-if="showAdd" class="add-panel">
        <input v-model="newName" placeholder="项目名称" @keyup.enter="handleAdd" />
        <div class="row"><input v-model="newDir" placeholder="项目目录" style="flex:1;min-width:0" /><button class="btn" @click="pickDir">...</button></div>
        <div v-if="detectedLang" style="font-size:11px;color:var(--jc-color-success)">识别: {{ detectedLang }} · {{ detectedCmds.length }} 命令</div>
        <button class="btn pri" @click="handleAdd">添加</button>
      </div>
      <div class="tree">
        <div v-for="p in store.projects" :key="p.id">
          <div class="proj" :class="{sel:store.selectedProjectId===p.id}" @click="toggleExpand(p.id);store.selectedProjectId=p.id" @contextmenu="openProjCtx($event,p.id)">
            <template v-if="editingProjId===p.id">
              <input class="proj-edit-input" v-model="editProjName" @keyup.enter="confirmRenameProj" @keyup.escape="editingProjId=''" @blur="confirmRenameProj" @click.stop />
            </template>
            <template v-else>
            <span class="arrow">{{ expandedProjects.has(p.id)?'▾':'▸' }}</span><span class="pn">{{ p.name }}</span><span class="pc">{{ p.commands.length }}</span>
            <button class="del" @click.stop="store.removeProject(p.id)">✕</button>
            </template>
          </div>
          <div v-if="expandedProjects.has(p.id)" class="cmds">
            <div v-for="cmd in p.commands" :key="cmd.id" class="cmd" :class="{on:isRunning(p.id,cmd.id)}" @contextmenu="openCmdCtx($event,p.id,cmd)">
              <template v-if="editingCmdId===p.id+'::'+cmd.id">
                <input class="cmd-edit-input" v-model="editCmdName" @keyup.enter="confirmRenameCmd" @keyup.escape="editingCmdId=''" @blur="confirmRenameCmd" @click.stop />
              </template>
              <template v-else>
              <span class="dot" :class="{live:isRunning(p.id,cmd.id)}"></span>
              <span class="cn" @click="store.startCommand(p.id,cmd)" @dblclick="editCmd(p.id,cmd)" :title="cmd.command">{{ cmd.name }}</span>
              <button v-if="isRunning(p.id,cmd.id)" class="stop" @click.stop="store.stopCommand(p.id,cmd.id)">■</button>
              <button class="del" @click.stop="store.removeCommand(p.id,cmd.id)">✕</button>
              </template>
            </div>
            <button class="addc" @click="addQuickCmd(p.id)">+ 命令</button>
          </div>
        </div>
        <div v-if="store.projects.length===0&&!showAdd" class="empty">点击 + 添加项目</div>
      </div>
      <CommandDialog ref="cmdDialogRef" :project-id="dialogProjectId" :editing="editingCmd" @close="editingCmd=null" />
    </div>

    <!-- Shortcuts -->
    <div v-show="activeTab==='shortcuts'" class="panel" style="display:flex;flex-direction:column">
      <div class="bar"><button class="btn" @click="openScDlg">+ 添加快捷命令</button></div>
      <div class="tabs">
        <div :class="['tab',{on:scTab==='all'}]" @click="scTab='all'">全部</div>
        <div :class="['tab',{on:scTab==='freq'}]" @click="scTab='freq'">常用</div>
        <div :class="['tab',{on:scTab==='fav'}]" @click="scTab='fav'">收藏</div>
      </div>
      <div style="flex:1;overflow-y:auto">
          <!-- All: accordion single-expand -->
          <div v-if="scTab==='all'" v-for="cat in filteredCats" :key="cat" style="border-bottom:1px solid var(--jc-border-default)">
          <div class="scat" @click="expandedCat = expandedCat===cat?'':cat">{{ expandedCat===cat?'▾':'▸'}} {{ cat }} ({{ shortcutsByCat(cat).length }})</div>
          <div v-if="expandedCat===cat">
            <div v-for="s in shortcutsByCat(cat)" :key="s.id" class="sc" @click="store.useShortcut(s)" @contextmenu="openScCtx($event,s)" :title="s.command + '\n' + s.description">
              <span class="fav-star" v-if="s.favorite">★</span>
              <span class="scc">{{ s.command }}</span>
            </div>
          </div>
        </div>
        <!-- Frequent -->
        <div v-if="scTab==='freq'" v-for="s in filteredFreq" :key="s.id" class="sc" @click="store.useShortcut(s)" @contextmenu="openScCtx($event,s)" :title="s.command + '\n' + s.description">
          <span class="fav-star" v-if="s.favorite">★</span>
          <span class="scc">{{ s.command }}</span><span class="scd">{{ s.useCount }}次</span>
        </div>
        <!-- Favorites -->
        <div v-if="scTab==='fav'" v-for="s in filteredFav" :key="s.id" class="sc" @click="store.useShortcut(s)" @contextmenu="openScCtx($event,s)" :title="s.command + '\n' + s.description">
          <span class="fav-star">★</span>
          <span class="scc">{{ s.command }}</span>
        </div>
      </div>
      <div style="padding:4px 6px;border-top:1px solid var(--jc-border-default);flex-shrink:0">
        <input v-model="scSearch" placeholder="搜索命令..." style="width:100%;font-size:11px;padding:3px 6px" />
      </div>
    </div>

    <!-- Tools -->
    <div v-show="activeTab==='tools'" class="panel" style="padding:8px;display:flex;flex-direction:column;gap:4px">
      <div style="font-size:12px;font-weight:600;color:var(--jc-text-highlight);padding:4px 4px 8px">工具箱</div>
      <div class="tool-grid">
        <button class="tool-card" @click="activeTool='port';toolPort='';toolMsg=''" title="杀掉指定端口的进程">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/>
          </svg>
          <span>端口杀手</span>
        </button>
        <!-- 后续工具在此追加 -->
      </div>
    </div>

    <!-- Tool Dialog -->
    <Teleport to="body">
      <div v-if="activeTool==='port'" class="mbg" @click.self="activeTool=''">
        <div class="mw" style="min-width:360px">
          <div class="mt">端口杀手</div>
          <div class="mb">
            <div class="fld">
              <label>端口号</label>
              <input v-model="toolPort" placeholder="如: 8080" type="number" @keyup.enter="runTool" />
            </div>
            <div v-if="toolMsg" :style="{fontSize:'12px',color:toolMsg.includes('已杀掉')?'var(--jc-color-success)':'var(--jc-color-error)',wordBreak:'break-all',padding:'4px 0'}">{{ toolMsg }}</div>
            <div class="acts">
              <button class="btn" @click="activeTool=''">取消</button>
              <button class="btn pri" @click="runTool" :disabled="toolBusy">{{ toolBusy?'处理中...':'执行' }}</button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div v-if="projCtxShow" class="ctx" :style="{left:projCtxPos.x+'px',top:projCtxPos.y+'px'}" @click.stop>
        <div class="ci" @click="ctxRenameProj">重命名</div>
        <div class="ci" @click="ctxAddCmd">新增命令</div>
        <div class="ci" style="color:var(--jc-color-error)" @click="ctxDelProj">删除项目</div>
      </div>
    </Teleport>
    <Teleport to="body">
      <div v-if="cmdCtxShow" class="ctx" :style="{left:cmdCtxPos.x+'px',top:cmdCtxPos.y+'px'}" @click.stop>
        <div class="ci" @click="ctxEditCmd">编辑</div>
        <div class="ci" @click="ctxRenameCmd">重命名</div>
        <div class="ci" style="color:var(--jc-color-error)" @click="ctxDelCmd">删除</div>
      </div>
    </Teleport>
    <Teleport to="body">
      <div v-if="showScDlg" class="mbg" @click.self="showScDlg=false;editingScId=''">
        <div class="mw">
          <div class="mt">{{ editingScId?'编辑快捷命令':'添加快捷命令' }}</div>
          <div class="mb">
            <div class="fld"><label>名称</label><input v-model="newScName" placeholder="如: Go 编译" @keyup.enter="addSc" autofocus /></div>
            <div class="fld"><label>命令</label><input v-model="newScCmd" placeholder="如: go build -o app.exe ." @keyup.enter="addSc" style="font-family:'Cascadia Code',Consolas,monospace" /></div>
            <div class="fld"><label>分类</label><input v-model="newScCat" placeholder="如: Go / 自定义" /></div>
            <div class="fld"><label>说明</label><input v-model="newScDesc" placeholder="中文用法说明" /></div>
            <div class="acts"><button class="btn" @click="showScDlg=false">取消</button><button class="btn pri" @click="addSc">添加</button></div>
          </div>
        </div>
      </div>
    </Teleport>
    <Teleport to="body">
      <div v-if="scCtxShow" class="ctx" :style="{left:scCtxPos.x+'px',top:scCtxPos.y+'px'}" @click.stop>
        <div class="ci" @click="scCtxEdit">编辑</div>
        <div class="ci" @click="scCtxFav">{{ scCtxItem?.favorite?'取消收藏':'收藏' }}</div>
        <div class="ci" @click="scCtxDoc">查看文档</div>
        <div class="ci" style="color:var(--jc-color-error)" @click="scCtxDel">删除</div>
      </div>
    </Teleport>
  </aside>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;
.side { width:210px; min-width:210px; height:100%; background:var(--jc-bg-panel); display:flex; flex-direction:column; overflow:hidden; user-select:none; }
.side-head { height:2px; background:var(--jc-color-accent); }
.tabs { display:flex; }
.tab { @include tab-base; }
.panel { @include flex-panel; }
.bar { padding:6px 10px; border-bottom:1px solid var(--jc-border-default); }
.btn { @include btn-base; }
.btn.pri { @include btn-primary; }
.btn:disabled { opacity:.5; }
.add-panel { padding:8px 10px; display:flex; flex-direction:column; gap:5px; border-bottom:1px solid var(--jc-border-default);
  input { @include input-base; }
}
.row { display:flex; gap:4px; }
.tree { flex:1; overflow-y:auto; padding:4px 0; }
.proj { display:flex; align-items:center; gap:4px; padding:4px 10px; cursor:pointer; font-size:12px;
  &:hover { background:var(--jc-bg-hover); }
  &.sel { background:var(--jc-bg-selected); }
}
.arrow { font-size:9px; color:var(--jc-text-secondary); width:12px; flex-shrink:0; }
.pn { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.pc { font-size:10px; color:var(--jc-text-secondary); background:var(--jc-bg-btn); padding:0 4px; border-radius:3px; }
.del { display:none; background:none; color:var(--jc-text-secondary); font-size:12px; padding:0 4px; cursor:pointer;
  &:hover { color:var(--jc-color-error); }
}
.proj:hover .del,.cmd:hover .del { display:inline; }
.cmds { padding-left:12px; }
.cmd { display:flex; align-items:center; gap:4px; padding:3px 10px; font-size:12px;
  &:hover { background:var(--jc-bg-hover); }
  &.on { background:var(--jc-bg-selected); }
}
.dot { @include dot; }
.cn { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; cursor:pointer;
  &:hover { color:var(--jc-color-success); }
}
.stop { background:none; color:var(--jc-color-error); font-size:10px; padding:0 3px; cursor:pointer; }
.addc { display:block; width:100%; text-align:left; background:none; border:none; color:var(--jc-text-secondary); font-size:11px; padding:3px 10px; cursor:pointer;
  &:hover { color:var(--jc-color-success); }
}
.empty { padding:20px; text-align:center; font-size:11px; color:var(--jc-text-secondary); }
input { @include input-base; }
.ctx { @include ctx-menu; }
.ci { @include ctx-item; }
.proj-edit-input, .cmd-edit-input { background:var(--jc-bg-input); border:1px solid var(--jc-color-accent); color:var(--jc-text-primary); padding:1px 4px; font-size:12px; width:100%; outline:none; }
.scat { padding:6px 10px; font-size:11px; font-weight:600; color:var(--jc-text-highlight); cursor:pointer; background:var(--jc-bg-elevated);
  &:hover { background:var(--jc-bg-selected); }
}
.sc { padding:4px 10px 4px 20px; font-size:11px; cursor:pointer; color:var(--jc-text-secondary); display:flex; align-items:center;
  &:hover { background:var(--jc-bg-hover); color:var(--jc-color-success); }
}
.scc { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-family:'Cascadia Code',Consolas,monospace; }
.scd { font-size:9px; color:var(--jc-text-secondary); margin-left:4px; white-space:nowrap; }
.fav-star { color:var(--jc-color-favorite); margin-right:2px; font-size:10px; }
.mbg { position:fixed; inset:0; background:var(--jc-bg-overlay); display:flex; align-items:center; justify-content:center; z-index:1000; }
.mw { background:var(--jc-bg-elevated); border:1px solid var(--jc-border-strong); min-width:400px; box-shadow:var(--jc-shadow-modal); }
.mt { background:var(--jc-bg-panel); padding:10px 16px; font-size:14px; font-weight:600; color:var(--jc-text-highlight); border-bottom:1px solid var(--jc-border-default); }
.mb { padding:16px; display:flex; flex-direction:column; gap:12px; }
.fld { display:flex; flex-direction:column; gap:4px;
  label { font-size:11px; color:var(--jc-text-secondary); text-transform:uppercase; letter-spacing:.5px; }
  input { @include input-base; padding:6px 10px; font-size:13px; }
}
.acts { display:flex; justify-content:flex-end; gap:8px; margin-top:4px; }
.tool-grid { display:grid; grid-template-columns:1fr 1fr; gap:6px; }
.tool-card { display:flex; flex-direction:column; align-items:center; justify-content:center; gap:4px; padding:12px 4px; background:var(--jc-bg-elevated); border:1px solid var(--jc-border-default); border-radius:6px; cursor:pointer; color:var(--jc-text-secondary); font-size:11px; transition:all 80ms;
  &:hover { background:var(--jc-bg-hover); color:var(--jc-color-accent); border-color:var(--jc-color-accent); }
}
</style>
