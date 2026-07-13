<script setup lang="ts">
import { ref, nextTick, watch } from 'vue'
import { useStatusStore, type StatusMessage } from '@/stores/status'

const s = useStatusStore()
const listEl = ref<HTMLElement | null>(null)

// 点击遮罩关闭
function onOverlayClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains('np-overlay')) {
    s.closeNotificationPanel()
  }
}

// 新消息时自动滚到底部
watch(() => s.messages.length, async () => {
  await nextTick()
  if (listEl.value) {
    listEl.value.scrollTop = listEl.value.scrollHeight
  }
})

function formatTime(ts: number) {
  const d = new Date(ts)
  return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function typeLabel(t: StatusMessage['type']) {
  const map: Record<string, string> = { info: '信息', success: '成功', warn: '警告', error: '错误' }
  return map[t] ?? t
}

function typeClass(t: StatusMessage['type']) {
  return `np-msg-${t}`
}

function copyAll() {
  const text = s.messages.map(m => `[${formatTime(m.timestamp)}] [${typeLabel(m.type)}] ${m.text}`).join('\n')
  navigator.clipboard.writeText(text)
}

function clearAll() {
  s.messages.splice(0)
}
</script>

<template>
  <Teleport to="body">
    <div class="np-overlay" :class="{ show: s.notificationPanelOpen }" @click="onOverlayClick">
      <div class="np-panel" :class="{ show: s.notificationPanelOpen }" @click.stop>
        <!-- Header -->
        <div class="np-header">
          <div class="np-header-left">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M8 1.5A4.5 4.5 0 0 0 3.5 6v2l-1 2.5h11L12.5 8V6A4.5 4.5 0 0 0 8 1.5z"/>
              <path d="M6 12.5a2 2 0 0 0 4 0"/>
            </svg>
            <span class="np-title">通知中心</span>
            <span class="np-count">{{ s.messages.length }}</span>
          </div>
          <div class="np-header-right">
            <button class="np-btn" @click="copyAll" title="复制全部"><svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor" style="vertical-align:-2px;margin-right:3px"><path d="M281.6 32h374.464a70.4 70.4 0 0 1 49.792 20.608l201.536 201.536a70.4 70.4 0 0 1 20.608 49.792V806.4a57.6 57.6 0 0 1-57.6 57.6H281.6a57.6 57.6 0 0 1-57.6-57.6V89.6a57.6 57.6 0 0 1 57.6-57.6z m19.2 768h550.4a12.8 12.8 0 0 0 12.8-12.8V303.936a6.4 6.4 0 0 0-0.512-2.496l-1.344-2.048-201.536-201.536a6.4 6.4 0 0 0-4.48-1.856H300.8a12.8 12.8 0 0 0-12.8 12.8v678.4c0 7.04 5.76 12.8 12.8 12.8z"/><path d="M256 160v64H172.8a12.8 12.8 0 0 0-12.8 12.8v678.4c0 7.04 5.76 12.8 12.8 12.8h550.4a12.8 12.8 0 0 0 12.8-12.8V832h64v102.4a57.6 57.6 0 0 1-57.6 57.6H153.6a57.6 57.6 0 0 1-57.6-57.6V217.6a57.6 57.6 0 0 1 57.6-57.6H256zM672 64v211.2c0 7.04 5.76 12.8 12.8 12.8H896v64h-243.2a44.8 44.8 0 0 1-44.8-44.8V64h64z"/></svg> 复制</button>
            <button class="np-btn np-btn-danger" @click="clearAll" title="清空">🗑 清空</button>
            <button class="np-close" @click="s.closeNotificationPanel()" title="关闭">✕</button>
          </div>
        </div>

        <!-- Message list -->
        <div class="np-list" ref="listEl">
          <div v-if="s.messages.length === 0" class="np-empty">
            <svg viewBox="0 0 16 16" width="32" height="32" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" opacity="0.3">
              <path d="M8 1.5A4.5 4.5 0 0 0 3.5 6v2l-1 2.5h11L12.5 8V6A4.5 4.5 0 0 0 8 1.5z"/>
              <path d="M6 12.5a2 2 0 0 0 4 0"/>
            </svg>
            <span>暂无通知</span>
          </div>
          <div
            v-for="msg in s.messages"
            :key="msg.id"
            class="np-msg"
            :class="typeClass(msg.type)"
          >
            <div class="np-msg-header">
              <span class="np-msg-type">{{ typeLabel(msg.type) }}</span>
              <span class="np-msg-time">{{ formatTime(msg.timestamp) }}</span>
            </div>
            <div class="np-msg-text">{{ msg.text }}</div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped lang="scss">
/* ── Overlay ── */
.np-overlay {
  position: fixed;
  inset: 0;
  z-index: 9000;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s;

  &.show {
    opacity: 1;
    pointer-events: auto;
  }
}

/* ── Panel ── */
.np-panel {
  width: 460px;
  max-width: 90vw;
  max-height: 80vh;
  margin: 40px 20px 20px;
  background: var(--jc-bg-app, #1e1e2e);
  border: 1px solid var(--jc-border-default, #333);
  border-radius: 10px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transform: translateX(30px);
  transition: transform 0.2s ease;

  &.show {
    transform: translateX(0);
  }
}

/* ── Header ── */
.np-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--jc-border-default, #333);
  flex-shrink: 0;
}

.np-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--jc-text-highlight, #cdd6f4);
  font-size: 14px;
  font-weight: 600;
}

.np-title {
  font-size: 14px;
}

.np-count {
  font-size: 11px;
  font-weight: 700;
  color: #fff;
  background: var(--jc-color-accent, #8a58ff);
  min-width: 20px;
  height: 20px;
  padding: 0 6px;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.np-header-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.np-btn {
  background: var(--jc-bg-elevated, #313244);
  border: 1px solid var(--jc-border-default, #444);
  color: var(--jc-text-secondary, #a6adc8);
  font-size: 11px;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  white-space: nowrap;

  &:hover {
    background: var(--jc-bg-hover, #45475a);
    color: var(--jc-text-primary, #cdd6f4);
  }
}

.np-btn-danger:hover {
  background: rgba(243, 139, 168, 0.15);
  color: var(--jc-color-error, #f38ba8);
  border-color: var(--jc-color-error, #f38ba8);
}

.np-close {
  background: none;
  border: none;
  color: var(--jc-text-secondary, #a6adc8);
  font-size: 16px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  line-height: 1;

  &:hover {
    background: var(--jc-bg-hover, #45475a);
    color: var(--jc-text-primary, #cdd6f4);
  }
}

/* ── List ── */
.np-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;

  &::-webkit-scrollbar {
    width: 6px;
  }
  &::-webkit-scrollbar-track {
    background: transparent;
  }
  &::-webkit-scrollbar-thumb {
    background: var(--jc-border-default, #444);
    border-radius: 3px;
  }
}

.np-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 48px 16px;
  color: var(--jc-text-secondary, #a6adc8);
  font-size: 13px;
}

/* ── Message ── */
.np-msg {
  padding: 8px 12px;
  border-radius: 6px;
  border-left: 3px solid var(--jc-border-default, #444);
  background: var(--jc-bg-panel, #181825);
}

.np-msg-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.np-msg-type {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 1px 6px;
  border-radius: 3px;
}

.np-msg-time {
  font-size: 10px;
  color: var(--jc-text-secondary, #a6adc8);
  font-family: monospace;
}

.np-msg-text {
  font-size: 12px;
  line-height: 1.5;
  color: var(--jc-text-primary, #cdd6f4);
  word-break: break-all;
  white-space: pre-wrap;
}

/* ── Type colors ── */
.np-msg-info {
  border-color: var(--jc-color-accent, #8a58ff);
  .np-msg-type {
    color: var(--jc-color-accent, #8a58ff);
    background: rgba(138, 88, 255, 0.12);
  }
}

.np-msg-success {
  border-color: var(--jc-color-success, #a6e3a1);
  .np-msg-type {
    color: var(--jc-color-success, #a6e3a1);
    background: rgba(166, 227, 161, 0.12);
  }
}

.np-msg-warn {
  border-color: var(--jc-color-warning, #f9e2af);
  .np-msg-type {
    color: var(--jc-color-warning, #f9e2af);
    background: rgba(249, 226, 175, 0.12);
  }
}

.np-msg-error {
  border-color: var(--jc-color-error, #f38ba8);
  .np-msg-type {
    color: var(--jc-color-error, #f38ba8);
    background: rgba(243, 139, 168, 0.12);
  }
}
</style>
