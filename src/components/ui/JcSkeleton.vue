<script setup lang="ts">
defineOptions({ name: 'JcSkeleton' })

// API 对齐 Ant Design Skeleton：loading / active / avatar / title / paragraph / rows
// 参考: https://ant.design/components/skeleton-cn
withDefaults(
  defineProps<{
    /** 是否显示骨架（false 时渲染默认插槽内容） */
    loading?: boolean
    /** 是否启用流光动画 */
    active?: boolean
    avatar?: boolean
    title?: boolean
    paragraph?: boolean
    rows?: number
    /** 标题宽度 */
    width?: string | number
    avatarSize?: number
    avatarShape?: 'circle' | 'square'
  }>(),
  {
    loading: true,
    active: true,
    avatar: false,
    title: true,
    paragraph: true,
    rows: 3,
    width: undefined,
    avatarSize: 40,
    avatarShape: 'circle',
  },
)
</script>

<template>
  <slot v-if="!loading" />
  <div v-else :class="['jc-skeleton', { 'is-active': active }]">
    <div
      v-if="avatar"
      class="jc-skeleton__avatar"
      :class="`is-${avatarShape}`"
      :style="{ width: avatarSize + 'px', height: avatarSize + 'px' }"
    />
    <div class="jc-skeleton__content">
      <div
        v-if="title"
        class="jc-skeleton__title"
        :style="width !== undefined ? { width: typeof width === 'number' ? width + 'px' : width } : {}"
      />
      <template v-if="paragraph">
        <div
          v-for="i in rows"
          :key="i"
          class="jc-skeleton__paragraph"
          :style="i === rows ? { width: '60%' } : {}"
        />
      </template>
    </div>
  </div>
</template>

<style scoped>
.jc-skeleton {
  display: flex;
  gap: var(--jc-space, 16px);
  width: 100%;
}
.jc-skeleton.is-active .jc-skeleton__title,
.jc-skeleton.is-active .jc-skeleton__paragraph,
.jc-skeleton.is-active .jc-skeleton__avatar {
  background: linear-gradient(
    90deg,
    var(--jc-bg-hover, #2a2d2e) 25%,
    var(--jc-bg-selected, #37373d) 37%,
    var(--jc-bg-hover, #2a2d2e) 63%
  );
  background-size: 400% 100%;
  animation: jc-skeleton-loading 1.4s ease infinite;
}
.jc-skeleton__avatar {
  flex-shrink: 0;
  background: var(--jc-bg-hover, #2a2d2e);
}
.jc-skeleton__avatar.is-circle { border-radius: 50%; }
.jc-skeleton__avatar.is-square { border-radius: var(--jc-radius, 6px); }
.jc-skeleton__content { flex: 1; display: flex; flex-direction: column; gap: var(--jc-space-sm, 12px); }
.jc-skeleton__title,
.jc-skeleton__paragraph {
  height: 14px;
  border-radius: var(--jc-radius-sm, 4px);
  background: var(--jc-bg-hover, #2a2d2e);
}
.jc-skeleton__title { width: 38%; }
.jc-skeleton__paragraph { width: 100%; }

@keyframes jc-skeleton-loading {
  0% { background-position: 100% 50%; }
  100% { background-position: 0 50%; }
}
</style>
