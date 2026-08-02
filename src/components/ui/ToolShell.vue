<script setup lang="ts">
defineOptions({ name: 'ToolShell' })

// 工具页统一外壳（收编 32 个工具的 tool-container/tool-header/tool-body-split/editor-pane 重复结构）
// 用法：
//   <ToolShell title="Base64 转换器" split>
//     <template #actions> <JcButton type="primary">转换</JcButton> </template>
//     <template #left-label>原始内容</template>
//     <template #left> <JcTextarea v-model="input" class="jc-fill" mono /> </template>
//     <template #right-label>结果</template>
//     <template #right> <JcTextarea v-model="output" class="jc-fill" mono readonly /> </template>
//   </ToolShell>
withDefaults(
  defineProps<{
    title: string
    subtitle?: string
    /** 左右分栏布局 */
    split?: boolean
  }>(),
  {
    subtitle: '',
    split: false,
  },
)
</script>

<template>
  <div class="tool-shell">
    <header class="tool-shell__header">
      <div class="tool-shell__heading">
        <h2 class="tool-shell__title">{{ title }}</h2>
        <span v-if="subtitle" class="tool-shell__subtitle">{{ subtitle }}</span>
      </div>
      <div class="tool-shell__actions">
        <slot name="actions" />
      </div>
    </header>

    <div v-if="split" class="tool-shell__body tool-shell__body--split">
      <section class="tool-shell__pane">
        <div v-if="$slots['left-label']" class="tool-shell__pane-label">
          <slot name="left-label" />
        </div>
        <slot name="left" />
      </section>
      <section class="tool-shell__pane">
        <div v-if="$slots['right-label']" class="tool-shell__pane-label">
          <slot name="right-label" />
        </div>
        <slot name="right" />
      </section>
    </div>
    <div v-else class="tool-shell__body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.tool-shell {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--jc-bg-panel, #252526);
}
.tool-shell__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--jc-space-sm, 12px);
  padding: var(--jc-space-sm, 12px) var(--jc-space, 16px);
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  background: var(--jc-bg-elevated, #2d2d30);
  flex-shrink: 0;
}
.tool-shell__heading {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}
.tool-shell__title {
  font-size: var(--jc-font-size-lg, 16px);
  font-weight: var(--jc-font-weight-medium, 500);
  color: var(--jc-text-highlight, #e0e0e0);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tool-shell__subtitle {
  font-size: var(--jc-font-size-sm, 12px);
  color: var(--jc-text-secondary, #858585);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tool-shell__actions {
  display: flex;
  align-items: center;
  gap: var(--jc-space-xs, 8px);
  flex-shrink: 0;
}

.tool-shell__body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: var(--jc-space, 16px);
}
.tool-shell__body--split {
  display: flex;
  gap: var(--jc-space, 16px);
}
.tool-shell__pane {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--jc-space-xs, 8px);
}
.tool-shell__pane-label {
  font-size: var(--jc-font-size-sm, 12px);
  color: var(--jc-text-secondary, #858585);
}

/* 面板内撑满辅助类：给 textarea/div 加 class="jc-fill" 即填满所在面板 */
:global(.jc-fill) {
  width: 100%;
  height: 100%;
  min-height: 0;
  flex: 1;
  resize: none;
}
</style>
