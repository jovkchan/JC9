<script setup lang="ts">
import { ref } from 'vue'
import { useProjectStore } from '@/stores/project'
import TerminalView from '@/components/TerminalView.vue'
const store = useProjectStore()
const ctxShow=ref(false);const ctxPos=ref({x:0,y:0});const ctxIdx=ref(-1)
function openCtx(e:MouseEvent,i:number){e.preventDefault();ctxPos.value={x:e.clientX,y:e.clientY};ctxIdx.value=i;ctxShow.value=true}
function closeCtx(){ctxShow.value=false}
function ctxRestart(){const t=store.runningTabs[ctxIdx.value];const c=store.projects.find(p=>p.id===t.projectId)?.commands.find(c=>c.id===t.commandId);if(t&&c)store.restartCommand(t.projectId,c);closeCtx()}
function ctxStop(){const t=store.runningTabs[ctxIdx.value];if(t){store.stopCommand(t.projectId,t.commandId);store.closeTab(ctxIdx.value)};closeCtx()}
function ctxRename(){const t=store.runningTabs[ctxIdx.value];const c=store.projects.find(p=>p.id===t.projectId)?.commands.find(c=>c.id===t.commandId);if(c){const n=prompt('重命名:',c.name);if(n&&n.trim()){store.updateCommand(t.projectId,{...c,name:n.trim()});store.runningTabs[ctxIdx.value].commandName=n.trim()}};closeCtx()}
document.addEventListener('click',closeCtx)
</script>

<template>
  <div class="panel">
    <!-- Tab Bar: terminals + docs -->
    <div class="tabs" v-if="store.runningTabs.length>0||store.docTabs.length>0">
      <div v-for="(t,i) in store.runningTabs" :key="'t'+t.projectId+t.commandId"
        :class="['tab',{on:store.activeTabType==='term'&&i===store.activeTabIndex}]"
        @click="store.activeTabType='term';store.activeTabIndex=i" @contextmenu="openCtx($event,i)">
        <span class="tdot" :class="{live:store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'}"></span>
        <span class="tl">{{ t.commandName }}</span>
        <button class="tx" @click.stop="store.closeTab(i)">✕</button>
      </div>
      <div v-for="(t,i) in store.docTabs" :key="'d'+t.id"
        :class="['tab',{on:store.activeTabType==='doc'&&i===store.activeDocIndex}]"
        @click="store.activeTabType='doc';store.activeDocIndex=i">
        <span class="tl">📖 {{ t.title }}</span>
        <button class="tx" @click.stop="store.closeDocTab(i)">✕</button>
      </div>
    </div>

    <!-- Terminal content -->
    <div v-for="(t,i) in store.runningTabs" :key="'tc'+t.projectId+t.commandId" class="content" v-show="store.activeTabType==='term'&&i===store.activeTabIndex">
      <div class="bar">
        <code class="cmdtext">{{ t.command }}</code>
        <div class="acts">
          <button v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]!=='running'" class="btn pri" @click="()=>{const c=store.projects.find(p=>p.id===t.projectId)?.commands.find(c=>c.id===t.commandId);if(c)store.startCommand(t.projectId,c)}">启动</button>
          <button v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'" class="btn" @click="store.stopCommand(t.projectId,t.commandId)">停止</button>
          <button v-if="store.runningMap[store.cmdKey(t.projectId,t.commandId)]==='running'" class="btn" @click="()=>{const c=store.projects.find(p=>p.id===t.projectId)?.commands.find(c=>c.id===t.commandId);if(c)store.restartCommand(t.projectId,c)}">重启</button>
          <button class="btn" @click="store.clearOutput(t.projectId,t.commandId)">清屏</button>
        </div>
      </div>
      <TerminalView :process-id="store.cmdKey(t.projectId,t.commandId)" :active="store.activeTabType==='term'&&i===store.activeTabIndex" />
    </div>

    <!-- Doc content -->
    <div v-for="(t,i) in store.docTabs" :key="'dc'+t.id" class="content" v-show="store.activeTabType==='doc'&&i===store.activeDocIndex">
      <div class="bar"><code class="cmdtext">{{ t.command }}</code></div>
      <div class="doc-body" v-if="t.loading">加载中...</div>
      <div class="doc-body" v-else>{{ t.content }}</div>
    </div>

    <div v-if="store.runningTabs.length===0&&store.docTabs.length===0" class="empty">从左侧项目列表点击命令启动</div>

    <Teleport to="body">
      <div v-if="ctxShow" class="ctx" :style="{left:ctxPos.x+'px',top:ctxPos.y+'px'}" @click.stop>
        <div class="ci" @click="ctxRestart">重启</div>
        <div class="ci" @click="ctxRename">重命名</div>
        <div class="ci" @click="ctxStop">停止并关闭</div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.panel { flex:1; display:flex; flex-direction:column; overflow:hidden; min-width:0; }
.tabs { display:flex; background:#2d2d30; overflow-x:auto; flex-shrink:0; }
.tab { display:flex; align-items:center; gap:4px; padding:6px 12px; font-size:12px; cursor:pointer; color:#858585; border-right:1px solid #3e3e42; white-space:nowrap; }
.tab:hover { color:#ccc; background:#2a2d2e; }
.tab.on { color:#e0e0e0; background:#1e1e1e; }
.tdot { width:6px; height:6px; border-radius:50%; background:#555; flex-shrink:0; }
.tdot.live { background:#4ec9b0; }
.tl { max-width:160px; overflow:hidden; text-overflow:ellipsis; }
.tx { background:none; color:#858585; font-size:14px; padding:0 4px; cursor:pointer; }
.tx:hover { color:#f44747; }
.content { flex:1; display:flex; flex-direction:column; overflow:hidden; }
.bar { display:flex; justify-content:space-between; align-items:center; padding:6px 12px; background:#2d2d30; border-bottom:1px solid #3e3e42; flex-shrink:0; gap:8px; }
.cmdtext { font-size:11px; color:#4ec9b0; font-family:'Cascadia Code',Consolas,monospace; }
.acts { display:flex; gap:6px; }
.btn { background:#3c3c3c; color:#ccc; padding:3px 12px; font-size:11px; }
.btn:hover { background:#4c4c4c; }
.btn.pri { background:#007acc; color:#fff; }
.btn.pri:hover { background:#1a8ad4; }
.empty { flex:1; display:flex; align-items:center; justify-content:center; color:#858585; font-size:13px; }
.doc-body { flex:1; overflow-y:auto; padding:12px; font-family:'Cascadia Code',Consolas,monospace; font-size:12px; color:#ccc; white-space:pre-wrap; background:#1e1e1e; }
.ctx { position:fixed; z-index:9999; background:#2d2d30; border:1px solid #555; padding:4px 0; min-width:130px; box-shadow:0 4px 12px rgba(0,0,0,.5); }
.ci { padding:5px 14px; font-size:12px; cursor:pointer; color:#ccc; }
.ci:hover { background:#094771; }
</style>
