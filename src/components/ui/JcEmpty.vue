<script setup lang="ts">
defineOptions({ name: 'JcEmpty' })

// API 对齐 Ant Design Empty：image / description + 自定义插槽
// 参考: https://ant.design/components/empty-cn
withDefaults(
  defineProps<{
    /** 自定义图形/emoji；留空用默认占位 */
    image?: string
    description?: string
  }>(),
  {
    image: '',
    description: '',
  },
)
</script>

<template>
  <div class="jc-empty">
    <div class="jc-empty__image">
      <slot name="image">
        <span class="jc-empty__img" aria-hidden="true">{{ image || '📄' }}</span>
      </slot>
    </div>
    <div v-if="description || $slots.description" class="jc-empty__desc">
      <slot name="description">{{ description }}</slot>
    </div>
    <div v-if="$slots.default" class="jc-empty__footer">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.jc-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--jc-space-sm, 12px);
  padding: var(--jc-space-lg, 24px);
  color: var(--jc-text-secondary, #858585);
  text-align: center;
}
.jc-empty__image {
  font-size: 48px;
  line-height: 1;
  opacity: 0.7;
  user-select: none;
}
.jc-empty__desc {
  font-size: var(--jc-font-size, 13px);
  color: var(--jc-text-secondary, #858585);
}
.jc-empty__footer {
  margin-top: var(--jc-space-xxs, 4px);
}
</style>
