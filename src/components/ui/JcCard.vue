<script setup lang="ts">
import JcSkeleton from './JcSkeleton.vue'

defineOptions({ name: 'JcCard' })

// API 对齐 Ant Design Card：title / extra / bordered / hoverable / size / loading
// 参考: https://ant.design/components/card-cn
withDefaults(
  defineProps<{
    title?: string
    extra?: string
    bordered?: boolean
    hoverable?: boolean
    size?: 'default' | 'small'
    loading?: boolean
  }>(),
  {
    title: '',
    extra: '',
    bordered: true,
    hoverable: false,
    size: 'default',
    loading: false,
  },
)
</script>

<template>
  <div
    :class="[
      'jc-card',
      `jc-card--${size}`,
      {
        'is-bordered': bordered,
        'is-hoverable': hoverable,
        'is-loading': loading,
      },
    ]"
  >
    <div v-if="title || extra || $slots.title || $slots.extra" class="jc-card__head">
      <div class="jc-card__title"><slot name="title">{{ title }}</slot></div>
      <div class="jc-card__extra"><slot name="extra">{{ extra }}</slot></div>
    </div>
    <div class="jc-card__body">
      <JcSkeleton v-if="loading" :paragraph="true" :rows="3" />
      <slot v-else />
    </div>
    <div v-if="$slots.actions" class="jc-card__actions"><slot name="actions" /></div>
  </div>
</template>

<style scoped>
.jc-card {
  display: flex;
  flex-direction: column;
  background: var(--jc-bg-panel, #252526);
  color: var(--jc-text-primary, #ccc);
  overflow: hidden;
  transition: box-shadow 0.2s ease, transform 0.2s ease, border-color 0.2s ease;
}
.jc-card.is-bordered {
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: var(--jc-radius-lg, 8px);
}
.jc-card.is-hoverable:hover {
  box-shadow: var(--jc-shadow-2, 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 6px 16px 0 rgba(0, 0, 0, 0.08));
  transform: translateY(-1px);
  border-color: var(--jc-border-strong, #555);
}

.jc-card__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--jc-space-sm, 12px);
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  flex-shrink: 0;
}
.jc-card__title {
  font-size: var(--jc-font-size-lg, 16px);
  font-weight: var(--jc-font-weight-medium, 500);
  color: var(--jc-text-highlight, #e0e0e0);
}
.jc-card__extra { font-size: var(--jc-font-size, 13px); color: var(--jc-text-secondary, #858585); }

.jc-card--default .jc-card__head { padding: var(--jc-space, 16px) var(--jc-space-lg, 24px); }
.jc-card--default .jc-card__body { padding: var(--jc-space-lg, 24px); }
.jc-card--small .jc-card__head { padding: var(--jc-space-sm, 12px) var(--jc-space, 16px); }
.jc-card--small .jc-card__body { padding: var(--jc-space, 16px); }

.jc-card__body { flex: 1; min-height: 0; }
.jc-card__actions {
  display: flex;
  align-items: center;
  border-top: 1px solid var(--jc-border-default, #3e3e42);
  padding: var(--jc-space-sm, 12px) var(--jc-space-lg, 24px);
  gap: var(--jc-space, 16px);
  flex-shrink: 0;
}
</style>
