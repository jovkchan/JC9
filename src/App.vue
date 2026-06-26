<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useProjectStore } from '@/stores/project'
import ProjectSidebar from '@/components/ProjectSidebar.vue'
import MainPanel from '@/components/MainPanel.vue'
const store = useProjectStore()
const sidebarCollapsed = ref(false)
onMounted(async () => { await store.loadProjects(); await store.initListeners(); setTimeout(() => store.startDefaultTerminal(), 200) })
onUnmounted(() => store.destroyListeners())
</script>
<template>
  <div class="app">
    <div class="sidebar-wrap" :class="{fold:sidebarCollapsed}">
      <ProjectSidebar v-show="!sidebarCollapsed" />
    </div>
    <div class="splitter" @click="sidebarCollapsed=!sidebarCollapsed" :title="sidebarCollapsed?'展开侧栏':'折叠侧栏'">
      <span class="splitter-arrow">{{ sidebarCollapsed?'▶':'◀' }}</span>
    </div>
    <MainPanel />
  </div>
</template>
<style scoped>
.app { display:flex; height:100vh; background:#1e1e1e; }
.sidebar-wrap { width:210px; min-width:210px; transition:width .15s; overflow:hidden; }
.sidebar-wrap.fold { width:0; min-width:0; }
.splitter { width:8px; min-width:8px; background:#2d2d30; cursor:pointer; display:flex; align-items:center; justify-content:center; border-left:1px solid #3e3e42; border-right:1px solid #3e3e42; }
.splitter:hover { background:#8a58ff; }
.splitter-arrow { font-size:8px; color:#858585; }
.splitter:hover .splitter-arrow { color:#fff; }
</style>
