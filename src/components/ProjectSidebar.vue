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

const portInput=ref('');const portResult=ref('');const portWorking=ref(false)
async function killPort(){const p=parseInt(portInput.value);if(!p||p<1){portResult.value='请输入有效端口号';return};portWorking.value=true;portResult.value='';try{portResult.value=await invoke<string>('kill_port',{port:p})}catch(e:any){portResult.value=typeof e==='string'?e:(e?.message||String(e))};portWorking.value=false}

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
        <div v-if="detectedLang" style="font-size:11px;color:#4ec9b0">识别: {{ detectedLang }} · {{ detectedCmds.length }} 命令</div>
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
          <div v-if="scTab==='all'" v-for="cat in filteredCats" :key="cat" style="border-bottom:1px solid #3e3e42">
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
      <div style="padding:4px 6px;border-top:1px solid #3e3e42;flex-shrink:0">
        <input v-model="scSearch" placeholder="搜索命令..." style="width:100%;font-size:11px;padding:3px 6px" />
      </div>
    </div>

    <!-- Tools -->
    <div v-show="activeTab==='tools'" class="panel" style="padding:10px;display:flex;flex-direction:column;gap:10px">
      <div style="font-size:13px;font-weight:600">端口杀手</div>
      <input v-model="portInput" placeholder="端口号" @keyup.enter="killPort" type="number" />
      <button class="btn pri" @click="killPort" :disabled="portWorking">{{ portWorking?'查找中...':'杀掉进程' }}</button>
      <div v-if="portResult" :style="{fontSize:'11px',color:portResult.includes('已杀掉')?'#4ec9b0':'#f44747',wordBreak:'break-all'}">{{ portResult }}</div>
    </div>

    <Teleport to="body">
      <div v-if="projCtxShow" class="ctx" :style="{left:projCtxPos.x+'px',top:projCtxPos.y+'px'}" @click.stop>
        <div class="ci" @click="ctxRenameProj">重命名</div>
        <div class="ci" @click="ctxAddCmd">新增命令</div>
        <div class="ci" style="color:#f44747" @click="ctxDelProj">删除项目</div>
      </div>
    </Teleport>
    <Teleport to="body">
      <div v-if="cmdCtxShow" class="ctx" :style="{left:cmdCtxPos.x+'px',top:cmdCtxPos.y+'px'}" @click.stop>
        <div class="ci" @click="ctxEditCmd">编辑</div>
        <div class="ci" @click="ctxRenameCmd">重命名</div>
        <div class="ci" style="color:#f44747" @click="ctxDelCmd">删除</div>
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
        <div class="ci" style="color:#f44747" @click="scCtxDel">删除</div>
      </div>
    </Teleport>
  </aside>
</template>

<style scoped>
.side { width:210px; min-width:210px; height:100%; background:#252526; display:flex; flex-direction:column; overflow:hidden; user-select:none; }
.side-head { padding:1 10px; height: 2px; letter-spacing:.5px; color:#e0e0e0; border-bottom:1px solid #3e3e42; background: #7a7af7;}
.tabs { display:flex; }
.tab { flex:1; text-align:center; padding:6px 0; font-size:12px; cursor:pointer; color:#858585; border-bottom:2px solid transparent; }
.tab:hover { color:#ccc; }
.tab.on { color:#e0e0e0; border-bottom-color:#007acc; }
.panel { flex:1; display:flex; flex-direction:column; overflow:hidden; }
.bar { padding:6px 10px; border-bottom:1px solid #3e3e42; }
.btn { background:#3c3c3c; color:#ccc; padding:3px 12px; font-size:12px; }
.btn:hover { background:#4c4c4c; }
.btn.pri { background:#007acc; color:#fff; }
.btn.pri:hover { background:#1a8ad4; }
.btn:disabled { opacity:.5; }
.add-panel { padding:8px 10px; display:flex; flex-direction:column; gap:5px; border-bottom:1px solid #3e3e42; }
.add-panel input { background:#3c3c3c; border:1px solid #555; padding:4px 8px; color:#ccc; }
.add-panel input:focus { border-color:#007acc; }
.row { display:flex; gap:4px; }
.tree { flex:1; overflow-y:auto; padding:4px 0; }
.proj { display:flex; align-items:center; gap:4px; padding:4px 10px; cursor:pointer; font-size:12px; }
.proj:hover { background:#2a2d2e; }
.proj.sel { background:#37373d; }
.arrow { font-size:9px; color:#858585; width:12px; flex-shrink:0; }
.pn { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.pc { font-size:10px; color:#858585; background:#3c3c3c; padding:0 4px; border-radius:3px; }
.del { display:none; background:none; color:#858585; font-size:12px; padding:0 4px; cursor:pointer; }
.del:hover { color:#f44747; }
.proj:hover .del,.cmd:hover .del { display:inline; }
.cmds { padding-left:12px; }
.cmd { display:flex; align-items:center; gap:4px; padding:3px 10px; font-size:12px; }
.cmd:hover { background:#2a2d2e; }
.cmd.on { background:#37373d; }
.dot { width:6px; height:6px; border-radius:50%; background:#555; flex-shrink:0; }
.dot.live { background:#4ec9b0; }
.cn { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; cursor:pointer; }
.cn:hover { color:#4ec9b0; }
.stop { background:none; color:#f44747; font-size:10px; padding:0 3px; cursor:pointer; }
.addc { display:block; width:100%; text-align:left; background:none; border:none; color:#858585; font-size:11px; padding:3px 10px; cursor:pointer; }
.addc:hover { color:#4ec9b0; }
.empty { padding:20px; text-align:center; font-size:11px; color:#858585; }
input { background:#3c3c3c; border:1px solid #555; padding:4px 8px; color:#ccc; }
input:focus { border-color:#007acc; }
.ctx { position:fixed; z-index:9999; background:#2d2d30; border:1px solid #555; padding:4px 0; min-width:120px; box-shadow:0 4px 12px rgba(0,0,0,.5); }
.ci { padding:5px 14px; font-size:12px; cursor:pointer; color:#ccc; }
.ci:hover { background:#094771; }
.proj-edit-input, .cmd-edit-input { background:#3c3c3c; border:1px solid #007acc; color:#ccc; padding:1px 4px; font-size:12px; width:100%; outline:none; }
.scat { padding:6px 10px; font-size:11px; font-weight:600; color:#e0e0e0; cursor:pointer; background:#2d2d30; }
.scat:hover { background:#37373d; }
.sc { padding:4px 10px 4px 20px; font-size:11px; cursor:pointer; color:#858585; display:flex; align-items:center; }
.sc:hover { background:#2a2d2e; color:#4ec9b0; }
.scc { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-family:'Cascadia Code',Consolas,monospace; }
.scd { font-size:9px; color:#858585; margin-left:4px; white-space:nowrap; }
.fav-star { color:#f0c040; margin-right:2px; font-size:10px; }
.mbg { position:fixed; inset:0; background:rgba(0,0,0,.5); display:flex; align-items:center; justify-content:center; z-index:1000; }
.mw { background:#2d2d30; border:1px solid #555; min-width:400px; box-shadow:0 8px 24px rgba(0,0,0,.6); }
.mt { background:#252526; padding:10px 16px; font-size:14px; font-weight:600; color:#e0e0e0; border-bottom:1px solid #3e3e42; }
.mb { padding:16px; display:flex; flex-direction:column; gap:12px; }
.fld { display:flex; flex-direction:column; gap:4px; }
.fld label { font-size:11px; color:#858585; text-transform:uppercase; letter-spacing:.5px; }
.fld input { background:#3c3c3c; border:1px solid #555; padding:6px 10px; color:#ccc; font-size:13px; }
.fld input:focus { border-color:#007acc; }
.acts { display:flex; justify-content:flex-end; gap:8px; margin-top:4px; }
</style>
