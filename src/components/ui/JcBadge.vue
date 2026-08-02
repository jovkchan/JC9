<script setup lang="ts">
import { computed } from 'vue'

defineOptions({ name: 'JcBadge' })

// API 对齐 Ant Design Badge：count / max / dot / status / text / showZero
// 参考: https://ant.design/components/badge-cn
export type JcBadgeStatus = 'success' | 'error' | 'warning' | 'processing' | 'default'

const props = withDefaults(
  defineProps<{
    /** 展示的数字/文本；不传则不显示角标 */
    count?: number | string
    /** 封顶数值，超出显示 N+（对应 antd max） */
    max?: number
    /** 仅显示小圆点 */
    dot?: boolean
    /** 状态点（对应 antd status） */
    status?: JcBadgeStatus
    /** 状态点旁的文案（对应 antd text） */
    text?: string
    /** 为 0 时也显示（对应 antd showZero） */
    showZero?: boolean
    title?: string
  }>(),
  {
    count: undefined,
    max: 99,
    dot: false,
    status: 'default',
    text: '',
    showZero: false,
    title: '',
  },
)

const emit = defineEmits<{ click: [e: MouseEvent] }>()

const showCount = computed(() => {
  if (props.count === undefined) return false
  if (props.count === 0 && !props.showZero) return false
  return true
})

const display = computed(() => {
  if (props.dot) return ''
  const c = props.count as string | number
  return typeof c === 'number' && c > props.max ? `${props.max}+` : String(c)
})
</script>

<template>
  <span class="jc-badge" :title="title" @click="emit('click', $event)">
    <slot />
    <sup v-if="showCount" :class="['jc-badge__count', { 'is-dot': dot }]">
      {{ display }}
    </sup>
    <span
      v-if="status !== 'default'"
      :class="['jc-badge__status', `is-${status}`]"
      aria-hidden="true"
    />
    <span v-if="text" class="jc-badge__text">{{ text }}</span>
  </span>
</template>

<style scoped>
.jc-badge {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  vertical-align: middle;
}

.jc-badge__count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  padding: 0 5px;
  border-radius: 999px;
  background: var(--jc-color-error, #f44747);
  color: var(--jc-color-white, #fff);
  font-size: var(--jc-font-size-xs, 11px);
  font-weight: var(--jc-font-weight-medium, 500);
  line-height: 1;
  font-variant-numeric: var(--jc-font-variant-numeric, tabular-nums);
}
.jc-badge__count.is-dot {
  min-width: 8px;
  width: 8px;
  height: 8px;
  padding: 0;
}

.jc-badge__status {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.jc-badge__status.is-success { background: var(--jc-color-success, #4ec9b0); }
.jc-badge__status.is-error { background: var(--jc-color-error, #f44747); }
.jc-badge__status.is-warning { background: var(--jc-color-warning, #d7ba7d); }
.jc-badge__status.is-processing { background: var(--jc-color-accent, #8a58ff); }
.jc-badge__status.is-processing { animation: jc-badge-pulse 1.4s ease-in-out infinite; }

.jc-badge__text {
  color: var(--jc-text-secondary, #858585);
  font-size: var(--jc-font-size-sm, 12px);
}

@keyframes jc-badge-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(138, 88, 255, 0.4); }
  50% { box-shadow: 0 0 0 4px rgba(138, 88, 255, 0); }
}
</style>
