<script setup lang="ts">
import { ref, nextTick, computed, onMounted, onUnmounted } from 'vue'
import { useProjectStore } from '@/stores/project'
import { open } from '@tauri-apps/plugin-dialog'
import CommandDialog from '@/components/CommandDialog.vue'
import type { Command } from '@/types'

const store = useProjectStore()
const activeTab = ref<'projects' | 'shortcuts' | 'tools'>('projects')
const showAdd = ref(false)
const newName = ref('')
const newDir = ref('')
const expandedProjects = ref<Set<string>>(new Set())
const detectedLang = ref('')
const detectedCmds = ref<{ name: string; command: string; workingDir: string }[]>([])
const dialogProjectId = ref('')
const editingCmd = ref<Command | null>(null)
const cmdDialogRef = ref<InstanceType<typeof CommandDialog>>()

function toggleExpand(id: string) {
  if (expandedProjects.value.has(id)) {
    expandedProjects.value.delete(id)
  } else {
    expandedProjects.value.add(id)
  }
}

async function pickDir() {
  const d = await open({
    directory: true,
    multiple: false,
    title: '选择项目目录'
  })
  if (d && typeof d === 'string') {
    newDir.value = d
    newName.value = d.split(/[\\/]/).pop() || newName.value
    const info = await store.detectProject(d)
    if (info) {
      newName.value = info.name
      detectedLang.value = info.lang
      detectedCmds.value = info.suggestCommands
    }
  }
}

async function handleAdd() {
  const n = newName.value.trim() || '新项目'
  store.addProject(n)
  const pid = store.projects[store.projects.length - 1].id
  expandedProjects.value.add(pid)
  for (const c of detectedCmds.value) {
    store.addCommand(pid, c)
  }
  newName.value = ''
  newDir.value = ''
  detectedLang.value = ''
  detectedCmds.value = []
  showAdd.value = false
}

function addQuickCmd(id: string) {
  editingCmd.value = null
  dialogProjectId.value = id
  cmdDialogRef.value?.openDialog()
}

function editCmd(pid: string, cmd: Command) {
  editingCmd.value = cmd
  dialogProjectId.value = pid
  cmdDialogRef.value?.openDialog()
}

function isRunning(pid: string, cid: string) {
  return store.runningMap[store.cmdKey(pid, cid)] === 'running'
}

// ---- Project context menu ----
const projCtxShow = ref(false)
const projCtxPos = ref({ x: 0, y: 0 })
const projCtxId = ref('')

function openProjCtx(e: MouseEvent, pid: string) {
  e.preventDefault()
  projCtxPos.value = { x: e.clientX, y: e.clientY }
  projCtxId.value = pid
  projCtxShow.value = true
}

function closeProjCtx() {
  projCtxShow.value = false
}

function ctxRenameProj() {
  editingProjId.value = projCtxId.value
  editProjName.value = store.projects.find(p => p.id === projCtxId.value)?.name || ''
  closeProjCtx()
  nextTick(() => {
    const el = document.querySelector<HTMLInputElement>('.proj-edit-input')
    el?.focus()
    el?.select()
  })
}

function confirmRenameProj() {
  const n = editProjName.value.trim()
  if (n) {
    store.updateProjectName(editingProjId.value, n)
  }
  editingProjId.value = ''
}

// 新增快速命令
function ctxAddCmd() {
  dialogProjectId.value = projCtxId.value
  editingCmd.value = null
  cmdDialogRef.value?.openDialog()
  closeProjCtx()
}

function ctxDelProj() {
  store.removeProject(projCtxId.value)
  closeProjCtx()
}

const editingProjId = ref('')
const editProjName = ref('')

// ---- Command context menu ----
const cmdCtxShow = ref(false)
const cmdCtxPos = ref({ x: 0, y: 0 })
const cmdCtxPid = ref('')
const cmdCtxCmd = ref<Command | null>(null)

function openCmdCtx(e: MouseEvent, pid: string, cmd: Command) {
  e.preventDefault()
  e.stopPropagation()
  cmdCtxPos.value = { x: e.clientX, y: e.clientY }
  cmdCtxPid.value = pid
  cmdCtxCmd.value = cmd
  cmdCtxShow.value = true
}

function closeCmdCtx() {
  cmdCtxShow.value = false
}

function ctxEditCmd() {
  if (cmdCtxCmd.value) {
    dialogProjectId.value = cmdCtxPid.value
    editingCmd.value = cmdCtxCmd.value
    cmdDialogRef.value?.openDialog()
  }
  closeCmdCtx()
}

function ctxRenameCmd() {
  const c = cmdCtxCmd.value
  if (c) {
    editingCmdId.value = cmdCtxPid.value + '::' + c.id
    editCmdName.value = c.name
  }
  closeCmdCtx()
  nextTick(() => {
    const el = document.querySelector<HTMLInputElement>('.cmd-edit-input')
    el?.focus()
    el?.select()
  })
}

function confirmRenameCmd() {
  const [pid, cid] = editingCmdId.value.split('::')
  const n = editCmdName.value.trim()
  const p = store.projects.find(p => p.id === pid)
  const c = p?.commands.find(c => c.id === cid)
  if (n && c) {
    store.updateCommand(pid, { ...c, name: n })
  }
  editingCmdId.value = ''
}

const editingCmdId = ref('')
const editCmdName = ref('')

function ctxDelCmd() {
  if (cmdCtxCmd.value) {
    store.removeCommand(cmdCtxPid.value, cmdCtxCmd.value.id)
  }
  closeCmdCtx()
}

// ---- Shortcuts ----
const showScDlg = ref(false)
const newScName = ref('')
const newScCmd = ref('')
const newScDesc = ref('')
const newScCat = ref('')

function openScDlg() {
  showScDlg.value = true
  newScName.value = ''
  newScCmd.value = ''
  newScDesc.value = ''
  newScCat.value = ''
}

const expandedCat = ref('')
const scSearch = ref('')
const filteredCats = computed(() => {
  return shortcutCats.value.filter(c => {
    return shortcutsByCat(c).some(s => {
      return s.command.includes(scSearch.value) ||
             s.description.includes(scSearch.value) ||
             s.name.includes(scSearch.value)
    })
  })
})

const filteredFreq = computed(() => {
  return store.frequentShortcuts.filter(s => {
    return s.command.includes(scSearch.value) ||
           s.description.includes(scSearch.value) ||
           s.name.includes(scSearch.value)
  })
})

const filteredFav = computed(() => {
  return store.favShortcuts.filter(s => {
    return s.command.includes(scSearch.value) ||
           s.description.includes(scSearch.value) ||
           s.name.includes(scSearch.value)
  })
})

const shortcutCats = computed(() => {
  return [...new Set(store.shortcuts.map(s => s.category))]
})

function shortcutsByCat(cat: string) {
  return store.shortcuts.filter(s => s.category === cat)
}

function addSc() {
  const n = newScName.value.trim()
  const c = newScCmd.value.trim()
  if (!n || !c) return
  if (editingScId.value) {
    store.updateShortcut(editingScId.value, {
      name: n,
      command: c,
      description: newScDesc.value.trim(),
      category: newScCat.value.trim() || '自定义'
    })
  } else {
    store.addShortcut({
      name: n,
      command: c,
      description: newScDesc.value.trim(),
      category: newScCat.value.trim() || '自定义'
    })
  }
  showScDlg.value = false
  editingScId.value = ''
}

const scTab = ref<'all' | 'freq' | 'fav'>('all')

// Shortcut context menu
const scCtxShow = ref(false)
const scCtxPos = ref({ x: 0, y: 0 })
const scCtxItem = ref<import('@/stores/project').ShortcutItem | null>(null)

function openScCtx(e: MouseEvent, s: import('@/stores/project').ShortcutItem) {
  e.preventDefault()
  scCtxPos.value = { x: e.clientX, y: e.clientY }
  scCtxItem.value = s
  scCtxShow.value = true
}

function closeScCtx() {
  scCtxShow.value = false
}

function scCtxEdit() {
  const s = scCtxItem.value
  if (s) {
    newScName.value = s.name
    newScCmd.value = s.command
    newScDesc.value = s.description
    newScCat.value = s.category
    editingScId.value = s.id
    showScDlg.value = true
  }
  closeScCtx()
}

function scCtxDel() {
  if (scCtxItem.value) {
    store.removeShortcut(scCtxItem.value.id)
  }
  closeScCtx()
}

function scCtxFav() {
  if (scCtxItem.value) {
    store.toggleFav(scCtxItem.value.id)
  }
  closeScCtx()
}

const editingScId = ref('')

async function scCtxDoc() {
  const s = scCtxItem.value
  if (!s) {
    closeScCtx()
    return
  }
  closeScCtx()
  store.openDoc(s.command, s.command)
}
const allTools = [
  { type: 'json', name: 'JSON 格式化', desc: 'JSON 美化/压缩与校验', category: 'code', icon: 'json' },
  { type: 'regex', name: '正则测试器', desc: '正则表达式实时高亮测试', category: 'code', icon: 'regex' },
  { type: 'base64', name: 'Base64 转换', desc: 'Base64 字符串编码解码', category: 'code', icon: 'base64' },
  { type: 'uuid', name: 'UUID 生成器', desc: '批量生成 UUID v4', category: 'code', icon: 'uuid' },
  { type: 'url', name: 'URL 编解码', desc: 'URL encode / decode 转换', category: 'code', icon: 'base64' },
  { type: 'unicode', name: 'Unicode 转换', desc: 'Unicode / ASCII 互转查询', category: 'code', icon: 'base64' },
  { type: 'jwt', name: 'JWT 解码器', desc: '解析 JWT Token Header & Payload', category: 'code', icon: 'json' },
  { type: 'hash', name: '哈希计算', desc: 'MD5 / SHA 系列散列值计算', category: 'code', icon: 'uuid' },
  { type: 'html', name: 'HTML 转义', desc: '实体编码 &lt; &gt; &amp; 互转', category: 'code', icon: 'base64' },
  { type: 'sql', name: 'SQL 格式/压缩', desc: 'SQL 语句一键美化缩进与压缩', category: 'code', icon: 'json' },
  { type: 'diff', name: '代码对比 (Diff)', desc: '文本/配置文件双栏差异对比', category: 'code', icon: 'diff' },
  { type: 'color', name: '颜色转换器', desc: 'HEX/RGB/HSL 互转与取色预览', category: 'code', icon: 'color' },
  { type: 'img-base64', name: '图片转 Base64', desc: '本地图片与 Base64 互转及还原', category: 'code', icon: 'base64' },
  { type: 'qr', name: '二维码工具', desc: '指定内容生成与上传图片解析', category: 'code', icon: 'qr' },
  { type: 'port', name: '端口释放器', desc: '精准释放指定端口占用的进程', category: 'network', icon: 'network' },
  { type: 'dns', name: 'DNS 解析查询', desc: '域名 A/CNAME/AAAA/MX/TXT 解析 dig 查询', category: 'network', icon: 'network' },
  { type: 'env', name: '环境变量查看', desc: '查看系统所有环境变量并过滤', category: 'system', icon: 'system' },
  { type: 'timestamp', name: '时间戳转换', desc: 'Unix时间戳与本地日期互转', category: 'system', icon: 'timestamp' },
  { type: 'time-calc', name: '时间计算器', desc: '工作日偏移及日期时间差计算', category: 'system', icon: 'timestamp' },
  { type: 'cron', name: 'Cron 表达式生成', desc: '可视化 Cron 表达式点选生成与直白中文解析', category: 'system', icon: 'timestamp' },
  { type: 'radix', name: '进制转换', desc: '二/八/十/十六进制高精度转换', category: 'code', icon: 'uuid' },
  { type: 'case', name: '命名风格转换', desc: '下划线/驼峰/帕斯卡/烤串/常量命名互转', category: 'code', icon: 'base64' },
  { type: 'lorem', name: '占位假文生成', desc: '一键生成中英文假文段落填充UI', category: 'code', icon: 'json' },
  { type: 'lines', name: '文本行操作器', desc: '多行文本排序、去重、拆分与合并', category: 'code', icon: 'diff' },
  { type: 'aes-des', name: '对称加解密 (AES/DES)', desc: 'AES/DES 在线加解密与编码转换', category: 'code', icon: 'key' },
  { type: 'rsa', name: '非对称加密 (RSA)', desc: 'RSA 密钥对生成、加解密与签名验签', category: 'code', icon: 'cert' },
  { type: 'css', name: 'CSS 单位换算', desc: 'PX、REM、EM、VW、VH 实时联动转换', category: 'code', icon: 'color' },
  { type: 'svg', name: 'SVG 预览与优化', desc: 'SVG 实时图形渲染预览与源码精简压缩', category: 'code', icon: 'color' },
  { type: 'ssh', name: 'SSH 密钥生成', desc: '生成安全多算法 SSH 密钥对', category: 'system', icon: 'key' },
  { type: 'ssl', name: 'SSL 证书生成', desc: '生成开发测试用自签名 SSL 证书对', category: 'system', icon: 'cert' }
]

const toolSearchQuery = ref('')

const filteredTools = computed(() => {
  const q = toolSearchQuery.value.trim().toLowerCase()
  if (!q) return allTools
  return allTools.filter(t => t.name.includes(q) || t.desc.includes(q))
})

const categorizedTools = computed(() => {
  const map: Record<string, typeof allTools> = { code: [], network: [], system: [] }
  filteredTools.value.forEach(t => {
    if (map[t.category]) {
      map[t.category].push(t)
    }
  })
  return map
})

const recentUsedTools = computed(() => {
  return store.recentTools.map(type => allTools.find(t => t.type === type)).filter(Boolean) as typeof allTools
})

function handleShortcutKeys(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return
  if (e.altKey && e.key >= '1' && e.key <= '9') {
    e.preventDefault()
    const idx = parseInt(e.key) - 1
    if (idx >= 0 && idx < allTools.length) {
      const tool = allTools[idx]
      store.openTool(tool.type, tool.name)
    }
  }
}

function handleGlobalClick() {
  closeProjCtx()
  closeCmdCtx()
  closeScCtx()
}

onMounted(() => {
  document.addEventListener('click', handleGlobalClick)
  document.addEventListener('keydown', handleShortcutKeys)
})

onUnmounted(() => {
  document.removeEventListener('click', handleGlobalClick)
  document.removeEventListener('keydown', handleShortcutKeys)
})
</script>

<template>
  <aside class="side">
    <div class="side-head"></div>
    <div class="tabs" role="tablist">
      <div :class="['tab',{on:activeTab==='projects'}]" role="tab" :aria-selected="activeTab==='projects'" tabindex="0" @click="activeTab='projects'" @keyup.enter="activeTab='projects'">项目</div>
      <div :class="['tab',{on:activeTab==='shortcuts'}]" role="tab" :aria-selected="activeTab==='shortcuts'" tabindex="0" @click="activeTab='shortcuts'" @keyup.enter="activeTab='shortcuts'">快捷</div>
      <div :class="['tab',{on:activeTab==='tools'}]" role="tab" :aria-selected="activeTab==='tools'" tabindex="0" @click="activeTab='tools'" @keyup.enter="activeTab='tools'">工具</div>
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
          <template v-if="scTab==='all'">
            <div v-for="cat in filteredCats" :key="cat" style="border-bottom:1px solid var(--jc-border-default)">
              <div class="scat" @click="expandedCat = expandedCat===cat?'':cat">{{ expandedCat===cat?'▾':'▸'}} {{ cat }} ({{ shortcutsByCat(cat).length }})</div>
              <div v-if="expandedCat===cat">
                <div v-for="s in shortcutsByCat(cat)" :key="s.id" class="sc" @click="store.useShortcut(s)" @contextmenu="openScCtx($event,s)" :title="s.command + '\n' + s.description">
                  <span class="fav-star" v-if="s.favorite">★</span>
                  <span class="scc">{{ s.command }}</span>
                </div>
              </div>
            </div>
          </template>
          <!-- Frequent -->
          <template v-if="scTab==='freq'">
            <div v-for="s in filteredFreq" :key="s.id" class="sc" @click="store.useShortcut(s)" @contextmenu="openScCtx($event,s)" :title="s.command + '\n' + s.description">
              <span class="fav-star" v-if="s.favorite">★</span>
              <span class="scc">{{ s.command }}</span><span class="scd">{{ s.useCount }}次</span>
            </div>
          </template>
          <!-- Favorites -->
          <template v-if="scTab==='fav'">
            <div v-for="s in filteredFav" :key="s.id" class="sc" @click="store.useShortcut(s)" @contextmenu="openScCtx($event,s)" :title="s.command + '\n' + s.description">
              <span class="fav-star">★</span>
              <span class="scc">{{ s.command }}</span>
            </div>
          </template>
      </div>
      <div style="padding:4px 6px;border-top:1px solid var(--jc-border-default);flex-shrink:0">
        <input v-model="scSearch" placeholder="搜索命令..." style="width:100%;font-size:11px;padding:3px 6px" />
      </div>
    </div>

    <!-- Tools -->
    <div v-show="activeTab==='tools'" class="panel" style="display:flex;flex-direction:column">
      <!-- 搜索过滤 -->
      <div class="search-bar">
        <input v-model="toolSearchQuery" placeholder="搜索实用工具..." class="tool-search-input" />
      </div>

      <div class="tools-list-container">
        <!-- 最近使用 -->
        <div v-if="recentUsedTools.length > 0 && !toolSearchQuery" class="tools-section">
          <div class="section-title">最近使用</div>
          <div class="tools-row-grid">
            <button v-for="t in recentUsedTools" :key="'rec-'+t.type" class="tool-item-card" @click="store.openTool(t.type, t.name)" :title="t.desc">
              <span class="tool-icon">
                <svg v-if="t.icon === 'json'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 19a1 1 0 0 1-1 1H7a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2a1 1 0 0 1 1 1"/><path d="M14 19a1 1 0 0 0 1 1h2a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2a1 1 0 0 0-1 1"/></svg>
                <svg v-else-if="t.icon === 'regex'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m5 5 14 14"/></svg>
                <svg v-else-if="t.icon === 'base64'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m17 3 4 4-4 4M21 7H3M7 21l-4-4 4-4M3 17h18"/></svg>
                <svg v-else-if="t.icon === 'uuid'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M7 21V3M17 21V3M3 12h18"/></svg>
                <svg v-else-if="t.icon === 'diff'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 3H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2z"/><path d="M9 7v10M5 12h8"/></svg>
                <svg v-else-if="t.icon === 'color'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
                <svg v-else-if="t.icon === 'qr'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="8" height="8" x="3" y="3" rx="1"/><rect width="8" height="8" x="13" y="3" rx="1"/><rect width="8" height="8" x="3" y="13" rx="1"/><path d="M13 13h1v1h-1zM18 13h3v3h-3zM13 18h3v3h-3zM18 18h1v1h-1z"/></svg>
                <svg v-else-if="t.icon === 'network'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/></svg>
                <svg v-else-if="t.icon === 'system'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="14" x="2" y="3" rx="2"/><path d="M8 21h8M12 17v4M6 8l4 4-4 4"/></svg>
                <svg v-else-if="t.icon === 'timestamp'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
                <svg v-else-if="t.icon === 'key'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 1.5 1.5M15.5 7.5 14 6"/></svg>
                <svg v-else-if="t.icon === 'cert'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
              </span>
              <div class="tool-info">
                <div class="tool-name">{{ t.name }}</div>
              </div>
            </button>
          </div>
        </div>

        <!-- 编码工具 -->
        <div v-if="categorizedTools.code.length > 0" class="tools-section">
          <div class="section-title">编码工具 (CODE)</div>
          <div class="tools-flex-list">
            <button v-for="t in categorizedTools.code" :key="t.type" class="tool-item-line" @click="store.openTool(t.type, t.name)">
              <div class="tool-meta-left">
                <span class="tool-icon">
                  <svg v-if="t.icon === 'json'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 19a1 1 0 0 1-1 1H7a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2a1 1 0 0 1 1 1"/><path d="M14 19a1 1 0 0 0 1 1h2a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2a1 1 0 0 0-1 1"/></svg>
                  <svg v-else-if="t.icon === 'regex'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m5 5 14 14"/></svg>
                  <svg v-else-if="t.icon === 'base64'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m17 3 4 4-4 4M21 7H3M7 21l-4-4 4-4M3 17h18"/></svg>
                  <svg v-else-if="t.icon === 'uuid'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M7 21V3M17 21V3M3 12h18"/></svg>
                  <svg v-else-if="t.icon === 'diff'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 3H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2z"/><path d="M9 7v10M5 12h8"/></svg>
                  <svg v-else-if="t.icon === 'color'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
                  <svg v-else-if="t.icon === 'qr'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="8" height="8" x="3" y="3" rx="1"/><rect width="8" height="8" x="13" y="3" rx="1"/><rect width="8" height="8" x="3" y="13" rx="1"/><path d="M13 13h1v1h-1zM18 13h3v3h-3zM13 18h3v3h-3zM18 18h1v1h-1z"/></svg>
                </span>
                <div class="tool-text-wrap">
                  <div class="tool-name">{{ t.name }}</div>
                  <div class="tool-desc">{{ t.desc }}</div>
                </div>
              </div>
              <span class="tool-shortcut-tag">Alt+{{ allTools.findIndex(x => x.type === t.type) + 1 }}</span>
            </button>
          </div>
        </div>

        <!-- 网络工具 -->
        <div v-if="categorizedTools.network.length > 0" class="tools-section">
          <div class="section-title">网络工具 (NETWORK)</div>
          <div class="tools-flex-list">
            <button v-for="t in categorizedTools.network" :key="t.type" class="tool-item-line" @click="store.openTool(t.type, t.name)">
              <div class="tool-meta-left">
                <span class="tool-icon">
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/></svg>
                </span>
                <div class="tool-text-wrap">
                  <div class="tool-name">{{ t.name }}</div>
                  <div class="tool-desc">{{ t.desc }}</div>
                </div>
              </div>
              <span class="tool-shortcut-tag">Alt+{{ allTools.findIndex(x => x.type === t.type) + 1 }}</span>
            </button>
          </div>
        </div>

        <!-- 系统工具 -->
        <div v-if="categorizedTools.system.length > 0" class="tools-section">
          <div class="section-title">系统工具 (SYSTEM)</div>
          <div class="tools-flex-list">
            <button v-for="t in categorizedTools.system" :key="t.type" class="tool-item-line" @click="store.openTool(t.type, t.name)">
              <div class="tool-meta-left">
                <span class="tool-icon">
                  <svg v-if="t.icon === 'system'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="14" x="2" y="3" rx="2"/><path d="M8 21h8M12 17v4M6 8l4 4-4 4"/></svg>
                  <svg v-else-if="t.icon === 'timestamp'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
                  <svg v-else-if="t.icon === 'key'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 1.5 1.5M15.5 7.5 14 6"/></svg>
                  <svg v-else-if="t.icon === 'cert'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                </span>
                <div class="tool-text-wrap">
                  <div class="tool-name">{{ t.name }}</div>
                  <div class="tool-desc">{{ t.desc }}</div>
                </div>
              </div>
              <span class="tool-shortcut-tag">Alt+{{ allTools.findIndex(x => x.type === t.type) + 1 }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>

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
.search-bar {
  padding: 6px 10px;
  border-bottom: 1px solid var(--jc-border-default);
  flex-shrink: 0;
}
.tool-search-input {
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 4px 8px;
  font-size: 11px;
  outline: none;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.tools-list-container {
  flex: 1;
  overflow-y: auto;
  padding: 6px 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.tools-section {
  display: flex;
  flex-direction: column;
}
.section-title {
  font-size: 9px;
  font-weight: 600;
  color: var(--jc-text-secondary);
  padding: 0 10px 4px 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.tools-row-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 6px;
  padding: 0 10px;
}
.tool-item-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 4px;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  cursor: pointer;
  color: var(--jc-text-primary);
  width: 100%;
  &:hover {
    background: var(--jc-bg-hover);
    border-color: var(--jc-color-accent);
    .tool-icon {
      color: var(--jc-color-accent-hover);
    }
  }
  .tool-icon {
    color: var(--jc-color-accent);
    display: flex;
    align-items: center;
  }
  .tool-name {
    font-size: 10px;
    font-weight: 600;
    text-align: center;
  }
}
.tools-flex-list {
  display: flex;
  flex-direction: column;
}
.tool-item-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: none;
  border: none;
  width: 100%;
  cursor: pointer;
  color: var(--jc-text-primary);
  text-align: left;
  &:hover {
    background: var(--jc-bg-hover);
    .tool-icon {
      color: var(--jc-color-accent-hover);
    }
    .tool-name {
      color: var(--jc-text-highlight);
    }
  }
}
.tool-meta-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  width: 80%;
}
.tool-icon {
  color: var(--jc-text-secondary);
  display: flex;
  align-items: center;
  flex-shrink: 0;
}
.tool-text-wrap {
  display: flex;
  flex-direction: column;
  min-width: 0;
  width: 100%;
}
.tool-name {
  font-size: 11px;
  font-weight: 600;
  color: var(--jc-text-primary);
}
.tool-desc {
  font-size: 9px;
  color: var(--jc-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  width: 100%;
}
.tool-shortcut-tag {
  font-size: 8px;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-secondary);
  padding: 1px 3px;
  border-radius: 3px;
  font-family: Consolas, monospace;
}
</style>
