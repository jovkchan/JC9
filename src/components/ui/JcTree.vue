<script lang="ts">
import { computed, defineComponent, h, ref } from 'vue'
import type { VNode } from 'vue'
import JcCheckbox from './JcCheckbox.vue'

// API 对齐 Ant Design Tree：treeData / defaultExpandAll / expandedKeys / selectedKeys / checkable / checkedKeys
// 参考: https://ant.design/components/tree-cn
export interface JcTreeNode {
  key: string | number
  title: string
  children?: JcTreeNode[]
  disabled?: boolean
  icon?: string
}

type Key = string | number

function collectKeys(nodes: JcTreeNode[]): Key[] {
  const out: Key[] = []
  const walk = (list: JcTreeNode[]) => {
    list.forEach((n) => {
      out.push(n.key)
      if (n.children?.length) walk(n.children)
    })
  }
  walk(nodes)
  return out
}

export default defineComponent({
  name: 'JcTree',
  props: {
    treeData: { type: Array, default: () => [] as JcTreeNode[] },
    defaultExpandAll: { type: Boolean, default: false },
    expandedKeys: { type: Array, default: null },
    selectedKeys: { type: Array, default: null },
    checkable: { type: Boolean, default: false },
    checkedKeys: { type: Array, default: null },
    selectable: { type: Boolean, default: true },
  },
  emits: ['update:expandedKeys', 'update:selectedKeys', 'update:checkedKeys', 'select', 'check', 'expand'],
  setup(props, { emit }) {
    const keys = computed(() => collectKeys((props.treeData as JcTreeNode[]) || []))

    // 展开状态（受控优先，否则内部维护）
    const internalExpanded = ref<Set<Key>>(new Set(props.defaultExpandAll ? keys.value : []))
    const expanded = computed<Set<Key>>(() =>
      props.expandedKeys ? new Set(props.expandedKeys as Key[]) : internalExpanded.value,
    )

    // 选中状态
    const internalSelected = ref<Set<Key>>(new Set())
    const selected = computed<Set<Key>>(() =>
      props.selectedKeys ? new Set(props.selectedKeys as Key[]) : internalSelected.value,
    )

    // 勾选状态
    const internalChecked = ref<Set<Key>>(new Set())
    const checked = computed<Set<Key>>(() =>
      props.checkedKeys ? new Set(props.checkedKeys as Key[]) : internalChecked.value,
    )

    function toggleExpand(key: Key) {
      const next = new Set(expanded.value)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      internalExpanded.value = next
      emit('update:expandedKeys', [...next])
      emit('expand', key, !next.has(key))
    }

    function select(key: Key) {
      if (!props.selectable) return
      const next = new Set(selected.value)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      internalSelected.value = next
      emit('update:selectedKeys', [...next])
      emit('select', key)
    }

    function check(node: JcTreeNode, value: boolean) {
      const next = new Set(checked.value)
      const walk = (n: JcTreeNode) => {
        if (value) next.add(n.key)
        else next.delete(n.key)
        n.children?.forEach(walk)
      }
      walk(node)
      internalChecked.value = next
      emit('update:checkedKeys', [...next])
      emit('check', node.key)
    }

    function renderNode(node: JcTreeNode, level: number): VNode {
      const hasChildren = !!(node.children && node.children.length)
      const isExpanded = expanded.value.has(node.key)
      const isSelected = selected.value.has(node.key)
      const isChecked = checked.value.has(node.key)

      const row = h(
        'div',
        {
          class: ['jc-tree__row', { 'is-selected': isSelected, 'is-disabled': node.disabled }],
          onClick: (e: MouseEvent) => {
            if (node.disabled) return
            e.stopPropagation()
            if (hasChildren) toggleExpand(node.key)
            select(node.key)
          },
        },
        [
          h(
            'span',
            {
              class: ['jc-tree__switcher', { 'is-open': isExpanded, 'is-leaf': !hasChildren }],
              onClick: (e: MouseEvent) => {
                e.stopPropagation()
                if (hasChildren) toggleExpand(node.key)
              },
            },
            hasChildren ? '▸' : '',
          ),
          props.checkable
            ? h(
                JcCheckbox,
                {
                  checked: isChecked,
                  disabled: node.disabled,
                  onClick: (e: MouseEvent) => e.stopPropagation(),
                  onChange: (v: boolean) => check(node, v),
                },
                { default: () => '' },
              )
            : null,
          node.icon ? h('span', { class: 'jc-tree__icon' }, node.icon) : null,
          h('span', { class: 'jc-tree__title' }, node.title),
        ],
      )

      const children = hasChildren && isExpanded
        ? h('div', { class: 'jc-tree__children' }, (node.children as JcTreeNode[]).map((c) => renderNode(c, level + 1)))
        : null

      return h('div', { class: ['jc-tree__node'], style: { paddingLeft: `${level * 18}px` } }, [row, children])
    }

    return () => {
      const list = (props.treeData as JcTreeNode[]) || []
      return h('div', { class: 'jc-tree', role: 'tree' }, list.map((n) => renderNode(n, 0)))
    }
  },
})
</script>

<style scoped>
.jc-tree {
  font-size: var(--jc-font-size, 13px);
  color: var(--jc-text-primary, #ccc);
  user-select: none;
}
.jc-tree__row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: var(--jc-radius-sm, 4px);
  cursor: pointer;
  white-space: nowrap;
}
.jc-tree__row:hover { background: var(--jc-bg-hover, #2a2d2e); }
.jc-tree__row.is-selected { background: var(--jc-color-accent-light-9, rgba(138, 88, 255, 0.15)); color: var(--jc-text-highlight, #e0e0e0); }
.jc-tree__row.is-disabled { opacity: 0.5; cursor: not-allowed; }

.jc-tree__switcher {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--jc-text-secondary, #858585);
  transition: transform 0.2s var(--jc-motion-ease, cubic-bezier(0.645, 0.045, 0.355, 1));
  flex-shrink: 0;
}
.jc-tree__switcher.is-open { transform: rotate(90deg); }
.jc-tree__switcher.is-leaf { visibility: hidden; }
.jc-tree__icon { margin-right: 4px; }
.jc-tree__title { overflow: hidden; text-overflow: ellipsis; }
.jc-tree__children { margin-left: 14px; border-left: 1px dashed var(--jc-border-default, #3e3e42); }
</style>
