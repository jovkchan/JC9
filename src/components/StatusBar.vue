<script setup lang="ts">
import { useStatusStore } from '@/stores/status'

const s = useStatusStore()
</script>

<template>
  <footer class="statusbar">
    <!-- Left group: connection + messages -->
    <div class="sb-left">
      <span class="sb-item conn" :class="s.connectionStatus" :title="'连接状态: ' + s.connectionLabel">
        <span class="sb-dot" :class="s.connectionStatus"></span>
        <span class="sb-label">{{ s.connectionLabel }}</span>
      </span>
      <span v-if="s.currentMessage" class="sb-item msg" :class="s.currentMessage.type" @click="s.openNotificationPanel()" title="点击查看全部通知">
        {{ s.currentMessage.text }}
      </span>
    </div>

    <!-- Right group: stats + notifications + user -->
    <div class="sb-right">
      <span class="sb-item stat" title="笔记数量">
        <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
          <path d="M4 2h8a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1z"/>
          <path d="M6 5h4M6 8h4M6 11h2"/>
        </svg>
        {{ s.noteCount }}
      </span>
      <span class="sb-item stat" title="项目数量">
        <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
          <path d="M2 4.5L8 2l6 2.5v7L8 14l-6-2.5z"/>
          <path d="M8 2v12"/>
          <path d="M2 7l6 2.5L14 7"/>
        </svg>
        {{ s.projectCount }}
      </span>
      <span class="sb-item notif" :class="{ has: s.notificationCount > 0, active: s.notificationPanelOpen }" title="通知中心" @click="s.toggleNotificationPanel()">
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
          <path d="M8 1.5A4.5 4.5 0 0 0 3.5 6v2l-1 2.5h11L12.5 8V6A4.5 4.5 0 0 0 8 1.5z"/>
          <path d="M6 12.5a2 2 0 0 0 4 0"/>
        </svg>
        <span v-if="s.messages.length > 0" class="sb-badge">{{ s.messages.length > 99 ? '99+' : s.messages.length }}</span>
      </span>
      <span class="sb-item user" :class="{ logged: s.isLoggedIn }" :title="s.isLoggedIn ? s.userName : '未登录'">
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="8" cy="5.5" r="3"/>
          <path d="M2 14c0-3.3 2.7-6 6-6s6 2.7 6 6"/>
        </svg>
        <span class="sb-label">{{ s.isLoggedIn ? s.userName : '未登录' }}</span>
      </span>
    </div>
  </footer>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;

.statusbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 24px;
  padding: 0 8px;
  background: var(--jc-bg-elevated);
  border-top: 1px solid var(--jc-border-default);
  flex-shrink: 0;
  user-select: none;
  font-size: 11px;
  color: var(--jc-text-secondary);
  gap: 4px;
}

// ── Item base ──
.sb-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 0 6px;
  height: 22px;
  white-space: nowrap;
  cursor: default;
  border-radius: 2px;

  &:hover {
    background: var(--jc-bg-hover);
    color: var(--jc-text-primary);
  }
}

// ── Left / Right groups ──
.sb-left {
  display: flex;
  align-items: center;
  gap: 2px;
  overflow: hidden;
  min-width: 0;
}

.sb-right {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

// ── Connection dot ──
.sb-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;

  &.local {
    background: var(--jc-color-success);
  }
  &.online {
    background: var(--jc-color-success);
  }
  &.offline {
    background: var(--jc-color-warning);
  }
  &.syncing {
    background: var(--jc-color-accent);
    animation: pulse 1s ease-in-out infinite;
  }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

// ── Connection label ──
.conn {
  cursor: pointer;
}

// ── Message ──
.msg {
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;

  &.success { color: var(--jc-color-success); }
  &.warn { color: var(--jc-color-warning); }
  &.error { color: var(--jc-color-error); }
}

// ── Stats ──
.stat {
  svg { opacity: 0.6; }
}

// ── Notification bell ──
.notif {
  position: relative;
  cursor: pointer;

  &.has svg { color: var(--jc-color-warning); }
  &.active { background: var(--jc-bg-selected); color: var(--jc-color-accent); }
}

.sb-badge {
  position: absolute;
  top: 1px;
  right: 2px;
  min-width: 14px;
  height: 14px;
  padding: 0 3px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 9px;
  font-weight: 700;
  color: #fff;
  background: var(--jc-color-error);
  border-radius: 7px;
  line-height: 1;
}

// ── User ──
.user {
  cursor: pointer;

  &.logged {
    color: var(--jc-color-success);
  }
}
</style>
