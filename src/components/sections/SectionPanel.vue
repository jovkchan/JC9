<script setup lang="ts">
import { useProjectStore } from '@/stores/project'
import { useAutomationStore } from '@/stores/automation'
import ProjectSidebar from '@/components/ProjectSidebar.vue'
import AutomationList from '@/components/automation/AutomationList.vue'
import BlockPalette from '@/components/automation/editor/BlockPalette.vue'
import NoteHome from '@/components/sections/NoteHome.vue'
import MemoryHome from '@/components/sections/MemoryHome.vue'

const store = useProjectStore()
const autoStore = useAutomationStore()
</script>

<template>
  <section class="section-panel">
    <!-- 项目/快捷/工具 复用 ProjectSidebar 的 panel -->
    <ProjectSidebar
      v-if="store.sidebarTab === 'projects' || store.sidebarTab === 'workflows' || store.sidebarTab === 'tools'"
      :forced-tab="store.sidebarTab"
    />
    <!-- 自动化：列表视图显示自动化列表；编辑视图隐藏列表、显示积木面板 -->
    <AutomationList v-else-if="store.sidebarTab === 'automation' && !autoStore.editing" />
    <BlockPalette v-else-if="store.sidebarTab === 'automation' && autoStore.editing" />
    <NoteHome v-else-if="store.sidebarTab === 'notes'" />
    <MemoryHome v-else-if="store.sidebarTab === 'memories'" />
  </section>
</template>

<style scoped>
.section-panel {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--jc-bg-panel);
  border-right: 1px solid var(--jc-border-default);
}
</style>
