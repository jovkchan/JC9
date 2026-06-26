<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useProjectStore } from '@/stores/project'
import ProjectSidebar from '@/components/ProjectSidebar.vue'
import MainPanel from '@/components/MainPanel.vue'
import TitleBar from '@/components/TitleBar.vue'
const store = useProjectStore()
const sidebarCollapsed = ref(false)
onMounted(async () => { await store.loadProjects(); await store.initListeners(); setTimeout(() => store.startDefaultTerminal(), 200) })
onUnmounted(() => store.destroyListeners())
</script>
<template>
  <div class="app">
    <TitleBar />
    <div class="app-body">
      <div class="sidebar-wrap" :class="{fold:sidebarCollapsed}">
        <ProjectSidebar v-show="!sidebarCollapsed" />
      </div>
      <div class="splitter" @click="sidebarCollapsed=!sidebarCollapsed" :title="sidebarCollapsed?'展开侧栏':'折叠侧栏'">
        <span class="splitter-arrow">{{ sidebarCollapsed?'▶':'◀' }}</span>
      </div>
      <MainPanel />
    </div>
  </div>
</template>
<style scoped lang="scss">
.app { display:flex; flex-direction:column; height:100vh; background:var(--jc-bg-app); }
.app-body { display:flex; flex:1; overflow:hidden; }
.sidebar-wrap { width:210px; min-width:210px; transition:width .15s; overflow:hidden; &.fold { width:0; min-width:0; } }
.splitter { width:8px; min-width:8px; background:var(--jc-bg-elevated); cursor:pointer; display:flex; align-items:center; justify-content:center; border-left:1px solid var(--jc-border-default); border-right:1px solid var(--jc-border-default);
  &:hover { background:var(--jc-color-purple); }
  &-arrow { font-size:8px; color:var(--jc-text-secondary); }
  &:hover &-arrow { color:var(--jc-color-white); }
}
</style>
