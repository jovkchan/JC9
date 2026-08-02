<script setup lang="ts">
import { computed } from 'vue'

defineOptions({ name: 'JcSwitch' })

// API 对齐 Ant Design Switch：checked / disabled / loading / size / checkedChildren / unCheckedChildren
// 参考: https://ant.design/components/switch-cn
const props = withDefaults(
  defineProps<{
    checked?: boolean
    disabled?: boolean
    loading?: boolean
    size?: 'default' | 'small'
    /** 选中时显示的文本 */
    checkedChildren?: string
    /** 未选中时显示的文本 */
    unCheckedChildren?: string
    title?: string
  }>(),
  {
    checked: false,
    disabled: false,
    loading: false,
    size: 'default',
    checkedChildren: '',
    unCheckedChildren: '',
    title: '',
  },
)

const emit = defineEmits<{
  'update:checked': [value: boolean]
  change: [value: boolean, e: Event]
}>()

const classes = computed(() => [
  'jc-switch',
  `jc-switch--${props.size}`,
  {
    'is-checked': props.checked,
    'is-disabled': props.disabled || props.loading,
  },
])

function toggle(e: Event) {
  if (props.disabled || props.loading) return
  const v = !props.checked
  emit('update:checked', v)
  emit('change', v, e)
}
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="checked"
    :class="classes"
    :disabled="disabled || loading"
    :title="title"
    @click="toggle"
  >
    <span v-if="loading" class="jc-switch__loader" aria-hidden="true" />
    <span class="jc-switch__inner">{{ checked ? checkedChildren : unCheckedChildren }}</span>
  </button>
</template>

<style scoped>
.jc-switch {
  position: relative;
  display: inline-flex;
  align-items: center;
  border: none;
  border-radius: 999px;
  background: var(--jc-bg-btn-hover, #4c4c4c);
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.2s var(--jc-motion-ease, cubic-bezier(0.645, 0.045, 0.355, 1));
}
.jc-switch::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  border-radius: 50%;
  background: var(--jc-color-white, #fff);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  transition: transform 0.2s var(--jc-motion-ease, cubic-bezier(0.645, 0.045, 0.355, 1));
}
.jc-switch--default { width: 44px; height: 22px; }
.jc-switch--default::after { width: 18px; height: 18px; }
.jc-switch--default.is-checked::after { transform: translateX(22px); }
.jc-switch--small { width: 32px; height: 16px; }
.jc-switch--small::after { width: 12px; height: 12px; }
.jc-switch--small.is-checked::after { transform: translateX(16px); }

.jc-switch.is-checked { background: var(--jc-color-accent, #8a58ff); }
.jc-switch.is-disabled { opacity: 0.5; cursor: not-allowed; }

.jc-switch__inner {
  font-size: 10px;
  color: var(--jc-color-white, #fff);
  line-height: 1;
  padding: 0 20px 0 22px;
  white-space: nowrap;
}
.jc-switch--small .jc-switch__inner { padding: 0 14px 0 16px; }
.jc-switch.is-checked .jc-switch__inner { padding: 0 22px 0 20px; }
.jc-switch--small.is-checked .jc-switch__inner { padding: 0 16px 0 14px; }

.jc-switch__loader {
  position: absolute;
  left: 8px;
  width: 10px;
  height: 10px;
  border: 2px solid var(--jc-color-white, #fff);
  border-top-color: transparent;
  border-radius: 50%;
  animation: jc-switch-spin 0.7s linear infinite;
}
.jc-switch.is-checked .jc-switch__loader { left: auto; right: 8px; }
@keyframes jc-switch-spin {
  to { transform: rotate(360deg); }
}
</style>
