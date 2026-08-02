<script setup lang="ts">
import { defineComponent } from 'vue'
import JcEmpty from './JcEmpty.vue'

defineOptions({ name: 'JcTable' })

// API 对齐 Ant Design Table：columns / dataSource / loading / rowKey / size / stripe / border
// 参考: https://ant.design/components/table-cn
// 规范：数值列建议 align="right"（对齐原则）；空数据显示 `--`（数据格式规范）
export interface JcTableColumn<T = Record<string, any>> {
  key: string
  title: string
  dataIndex?: string
  width?: number | string
  align?: 'left' | 'center' | 'right'
  ellipsis?: boolean
  /** 自定义渲染：返回 string / number / VNode 均可 */
  render?: (row: T, index: number) => unknown
}

/** 把单元格值渲染为任意内容（字符串或 VNode），空值显示 -- */
const RenderCell = defineComponent({
  name: 'RenderCell',
  props: { value: { default: null } },
  render() {
    const v = (this as any).value
    return v === undefined || v === null ? '--' : v
  },
})

const props = withDefaults(
  defineProps<{
    columns?: JcTableColumn[]
    dataSource?: Record<string, any>[]
    loading?: boolean
    rowKey?: string | ((row: Record<string, any>) => string)
    emptyText?: string
    size?: 'small' | 'middle' | 'large'
    stripe?: boolean
    border?: boolean
  }>(),
  {
    columns: () => [],
    dataSource: () => [],
    loading: false,
    rowKey: 'key',
    emptyText: '暂无数据',
    size: 'middle',
    stripe: false,
    border: false,
  },
)

const emit = defineEmits<{ 'row-click': [row: Record<string, any>, index: number, e: Event] }>()

function rowKeyOf(row: Record<string, any>, index: number): string {
  if (typeof props.rowKey === 'function') return (props.rowKey as (row: Record<string, any>) => string)(row)
  return String((row as any)[props.rowKey] ?? index)
}
function cellValue(col: JcTableColumn, row: Record<string, any>, index: number) {
  if (col.render) return col.render(row, index)
  if (col.dataIndex) return (row as any)[col.dataIndex]
  return ''
}
</script>

<template>
  <div :class="['jc-table', `jc-table--${size}`, { 'is-border': border, 'is-stripe': stripe }]">
    <table class="jc-table__table">
      <thead class="jc-table__head">
        <tr>
          <th
            v-for="col in columns"
            :key="col.key"
            :class="col.align ? `is-${col.align}` : ''"
            :style="col.width ? { width: typeof col.width === 'number' ? col.width + 'px' : col.width } : {}"
          >
            {{ col.title }}
          </th>
        </tr>
      </thead>
      <tbody v-if="loading" class="jc-table__body">
        <tr v-for="r in 3" :key="r">
          <td v-for="col in columns" :key="col.key">
            <div class="jc-table__skeleton-line" />
          </td>
        </tr>
      </tbody>
      <tbody v-else-if="dataSource.length === 0" class="jc-table__body">
        <tr>
          <td :colspan="columns.length || 1">
            <JcEmpty :description="emptyText" />
          </td>
        </tr>
      </tbody>
      <tbody v-else class="jc-table__body">
        <tr
          v-for="(row, index) in dataSource"
          :key="rowKeyOf(row, index)"
          @click="emit('row-click', row, index, $event)"
        >
          <td
            v-for="col in columns"
            :key="col.key"
            :class="[col.align ? `is-${col.align}` : '', { 'is-ellipsis': col.ellipsis }]"
          >
            <slot name="cell" :row="row" :index="index" :column="col">
              <RenderCell :value="cellValue(col, row, index)" />
            </slot>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.jc-table {
  width: 100%;
  overflow: auto;
  background: var(--jc-bg-panel, #252526);
  border-radius: var(--jc-radius, 6px);
}
.jc-table.is-border {
  border: 1px solid var(--jc-border-default, #3e3e42);
}
.jc-table.is-border td,
.jc-table.is-border th {
  border-right: 1px solid var(--jc-border-default, #3e3e42);
}
.jc-table.is-border td:last-child,
.jc-table.is-border th:last-child {
  border-right: none;
}

.jc-table__table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--jc-font-size, 13px);
}
.jc-table__head th {
  text-align: left;
  font-weight: var(--jc-font-weight-medium, 500);
  color: var(--jc-text-secondary, #858585);
  background: var(--jc-bg-elevated, #2d2d30);
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  white-space: nowrap;
  position: sticky;
  top: 0;
}
.jc-table--small { font-size: var(--jc-font-size-sm, 12px); }
.jc-table--small th, .jc-table--small td { padding: 4px 8px; }
.jc-table--middle th, .jc-table--middle td { padding: 8px 12px; }
.jc-table--large th, .jc-table--large td { padding: 12px 16px; }

.jc-table__body td {
  color: var(--jc-text-primary, #ccc);
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  vertical-align: middle;
}
.jc-table__body tr:last-child td { border-bottom: none; }
.jc-table__body tr.is-hoverable { cursor: default; }
.jc-table__body tbody tr:hover { background: var(--jc-bg-hover, #2a2d2e); }
.jc-table.is-stripe tbody tr:nth-child(even) { background: var(--jc-bg-hover, #2a2d2e); }

.jc-table th.is-right, .jc-table td.is-right { text-align: right; }
.jc-table th.is-center, .jc-table td.is-center { text-align: center; }
.jc-table td.is-ellipsis {
  max-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.jc-table__skeleton-line {
  height: 12px;
  border-radius: var(--jc-radius-sm, 4px);
  background: var(--jc-bg-hover, #2a2d2e);
  animation: jc-table-shimmer 1.4s ease infinite;
  background-image: linear-gradient(
    90deg,
    var(--jc-bg-hover, #2a2d2e) 25%,
    var(--jc-bg-selected, #37373d) 37%,
    var(--jc-bg-hover, #2a2d2e) 63%
  );
  background-size: 400% 100%;
}
@keyframes jc-table-shimmer {
  0% { background-position: 100% 50%; }
  100% { background-position: 0 50%; }
}
</style>
