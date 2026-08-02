<script setup lang="ts">
import { computed } from 'vue'

defineOptions({ name: 'JcButton' })

// API 对齐 Ant Design Button：type / danger / ghost / block / loading / htmlType / size / shape
// 参考: https://ant.design/components/button-cn
export type JcButtonType = 'primary' | 'default' | 'dashed' | 'text' | 'link'
export type JcButtonSize = 'large' | 'middle' | 'small'
export type JcButtonShape = 'default' | 'round' | 'circle'

const props = withDefaults(
  defineProps<{
    /** 按钮类型：主/默认/虚线/文本/链接（对应 antd type） */
    type?: JcButtonType
    /** 危险按钮：用于删除/移动等危险操作（对应 antd danger） */
    danger?: boolean
    /** 幽灵按钮：透明背景，用于深色/复杂背景（对应 antd ghost） */
    ghost?: boolean
    /** 宽度撑满父容器（对应 antd block） */
    block?: boolean
    /** 加载状态：禁用 + 转圈，防重复提交（对应 antd loading） */
    loading?: boolean
    disabled?: boolean
    /** 尺寸（对应 antd size：large | middle | small） */
    size?: JcButtonSize
    /** 形状（对应 antd shape：default | round | circle） */
    shape?: JcButtonShape
    /** 原生 button type（对应 antd htmlType） */
    htmlType?: 'button' | 'submit' | 'reset'
    title?: string
  }>(),
  {
    type: 'default',
    danger: false,
    ghost: false,
    block: false,
    loading: false,
    disabled: false,
    size: 'middle',
    shape: 'default',
    htmlType: 'button',
    title: '',
  },
)

const emit = defineEmits<{
  click: [e: MouseEvent]
}>()

const classes = computed(() => [
  'jc-btn',
  `jc-btn--${props.type}`,
  `jc-btn--${props.size}`,
  `jc-btn--${props.shape}`,
  {
    'is-danger': props.danger,
    'is-ghost': props.ghost,
    'is-loading': props.loading,
    'is-block': props.block,
  },
])
</script>

<template>
  <button
    :type="htmlType"
    :class="classes"
    :disabled="disabled || loading"
    :title="title"
    @click="(e) => !loading && emit('click', e)"
  >
    <span v-if="loading" class="jc-btn__spinner" aria-hidden="true" />
    <span class="jc-btn__content"><slot /></span>
  </button>
</template>

<style scoped>
/* 对齐 Ant Design Button 设计 Token 语义 */
.jc-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: var(--jc-radius, 6px);
  font-family: var(--jc-font-family, inherit);
  font-weight: var(--jc-font-weight-regular, 400);
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
  transition: all 0.2s cubic-bezier(0.645, 0.045, 0.355, 1);
}
.jc-btn:focus-visible {
  outline: 2px solid var(--jc-color-accent, #8a58ff);
  outline-offset: 1px;
}
.jc-btn:disabled {
  cursor: default;
  opacity: 0.5;
}

/* 尺寸（对应 antd controlHeightSM / controlHeight / controlHeightLG） */
.jc-btn--small { height: var(--jc-control-height-sm, 24px); padding: 0 10px; font-size: var(--jc-font-size-sm, 12px); }
.jc-btn--middle { height: var(--jc-control-height, 28px); padding: 0 14px; font-size: var(--jc-font-size-control, 12px); }
.jc-btn--large { height: var(--jc-control-height-lg, 36px); padding: 0 18px; font-size: var(--jc-font-size-lg, 14px); }

/* 形状 */
.jc-btn--round { border-radius: 999px; }
.jc-btn--circle { width: var(--jc-control-height, 28px); padding: 0; border-radius: 50%; }

/* primary 主按钮 */
.jc-btn--primary {
  background: var(--jc-color-accent, #8a58ff);
  color: var(--jc-color-white, #fff);
  box-shadow: 0 2px 0 rgba(0, 0, 0, 0.06);
}
.jc-btn--primary:hover:not(:disabled) { background: var(--jc-color-accent-hover, #a070ff); }
.jc-btn--primary.is-danger { background: var(--jc-color-error, #f44747); }
.jc-btn--primary.is-danger:hover:not(:disabled) { filter: brightness(1.1); }
.jc-btn--primary.is-ghost { background: transparent; color: var(--jc-color-accent, #8a58ff); box-shadow: none; }

/* default 默认按钮 */
.jc-btn--default {
  background: var(--jc-bg-btn, #3c3c3c);
  color: var(--jc-text-primary, #ccc);
  border: 1px solid var(--jc-border-strong, #555);
}
.jc-btn--default:hover:not(:disabled) {
  background: var(--jc-bg-btn-hover, #4c4c4c);
  border-color: var(--jc-color-accent, #8a58ff);
  color: var(--jc-color-accent, #8a58ff);
}
.jc-btn--default.is-danger { color: var(--jc-color-error, #f44747); border-color: var(--jc-color-error, #f44747); }
.jc-btn--default.is-danger:hover:not(:disabled) { background: var(--jc-color-error-light-9, rgba(244, 71, 71, 0.1)); }
.jc-btn--default.is-ghost { background: transparent; color: var(--jc-color-white, #fff); border-color: var(--jc-color-white, #fff); }

/* dashed 虚线按钮（常用于添加操作） */
.jc-btn--dashed {
  border: 1px dashed var(--jc-border-strong, #555);
  background: transparent;
  color: var(--jc-text-primary, #ccc);
}
.jc-btn--dashed:hover:not(:disabled) { border-color: var(--jc-color-accent, #8a58ff); color: var(--jc-color-accent, #8a58ff); }

/* text 文本按钮（最次级行动点） */
.jc-btn--text { background: transparent; color: var(--jc-text-primary, #ccc); }
.jc-btn--text:hover:not(:disabled) { background: var(--jc-bg-hover, #2a2d2e); }
.jc-btn--text.is-danger { color: var(--jc-color-error, #f44747); }
.jc-btn--text.is-danger:hover:not(:disabled) { background: var(--jc-color-error-light-9, rgba(244, 71, 71, 0.1)); }

/* link 链接按钮（导航跳转） */
.jc-btn--link { background: transparent; color: var(--jc-color-accent, #8a58ff); padding-inline: 0; }
.jc-btn--link:hover:not(:disabled) { color: var(--jc-color-accent-hover, #a070ff); }
.jc-btn--link.is-danger { color: var(--jc-color-error, #f44747); }

.jc-btn.is-block { display: flex; width: 100%; }
.jc-btn.is-loading { opacity: 0.65; }

.jc-btn__content { display: inline-flex; align-items: center; gap: 6px; }
.jc-btn__spinner {
  width: 12px;
  height: 12px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: jc-btn-spin 0.7s linear infinite;
}
@keyframes jc-btn-spin {
  to { transform: rotate(360deg); }
}
</style>
