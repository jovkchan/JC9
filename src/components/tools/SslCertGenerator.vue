<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcInputNumber from '@/components/ui/JcInputNumber.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const mode = ref<'single' | 'chain'>('single')
const commonName = ref('localhost')
const sans = ref<string[]>(['localhost', '127.0.0.1'])
const newSan = ref('')
const days = ref<number>(365)
const algorithm = ref<'rsa' | 'ecdsa'>('rsa')
const bits = ref<number>(2048)
const curve = ref<string>('prime256v1')

const bitsOptions = [
  { label: '2048 位 (标准)', value: 2048 },
  { label: '3072 位', value: 3072 },
  { label: '4096 位 (高度安全)', value: 4096 }
]
const curveOptions = [
  { label: 'prime256v1 (P-256)', value: 'prime256v1' },
  { label: 'secp384r1 (P-384)', value: 'secp384r1' }
]

const loading = ref(false)
const error = ref('')

// 证书生成结果
const serverKey = ref('')
const serverCert = ref('')
const caCert = ref('')
const clientKey = ref('')
const clientCert = ref('')

// 当前查看的文件（用于证书链模式下的分栏展示）
const activeFile = ref<'ca' | 'server_key' | 'server_cert' | 'client_key' | 'client_cert'>('ca')
const copySuccessMap = ref<Record<string, boolean>>({})

function addSanTag() {
  const val = newSan.value.trim()
  if (val && !sans.value.includes(val)) {
    sans.value.push(val)
  }
  newSan.value = ''
}

function removeSanTag(index: number) {
  sans.value.splice(index, 1)
}

async function generate() {
  loading.value = true
  error.value = ''
  
  // 重置结果
  serverKey.value = ''
  serverCert.value = ''
  caCert.value = ''
  clientKey.value = ''
  clientCert.value = ''

  try {
    const res = await invoke<{
      serverKey: string
      serverCert: string
      caCert?: string
      clientKey?: string
      clientCert?: string
    }>('generate_ssl_cert', {
      mode: mode.value,
      commonName: commonName.value,
      sans: sans.value,
      days: days.value,
      algo: algorithm.value,
      bits: bits.value,
      curve: curve.value
    })

    serverKey.value = res.serverKey
    serverCert.value = res.serverCert
    if (mode.value === 'chain') {
      caCert.value = res.caCert || ''
      clientKey.value = res.clientKey || ''
      clientCert.value = res.clientCert || ''
      activeFile.value = 'ca' // 默认选中 CA 证书
    }
  } catch (err: any) {
    error.value = err.toString()
  } finally {
    loading.value = false
  }
}

function copyText(key: string, text: string) {
  if (!text) return
  navigator.clipboard.writeText(text)
  copySuccessMap.value[key] = true
  setTimeout(() => {
    copySuccessMap.value[key] = false
  }, 2000)
}

function downloadFile(filename: string, content: string) {
  if (!content) return
  const blob = new Blob([content], { type: 'text/plain;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}

function downloadAll() {
  if (mode.value === 'single') {
    downloadFile('server.key', serverKey.value)
    downloadFile('server.crt', serverCert.value)
  } else {
    downloadFile('ca.crt', caCert.value)
    downloadFile('server.key', serverKey.value)
    downloadFile('server.crt', serverCert.value)
    downloadFile('client.key', clientKey.value)
    downloadFile('client.crt', clientCert.value)
  }
}
</script>

<template>
  <ToolShell title="SSL 自签名证书生成器">
    <div class="tool-body">
      <!-- 配置面板 -->
      <div class="card config-card">
        <div class="card-title">证书配置参数</div>

        <!-- 模式选择 -->
        <div class="config-row">
          <div class="config-item flex-1">
            <span class="config-label">生成模式</span>
            <div class="mode-selector">
              <button 
                :class="['mode-btn', { active: mode === 'single' }]" 
                @click="mode = 'single'"
              >
                单张服务器证书
                <span class="mode-desc">适用于 Web 服务 HTTPS</span>
              </button>
              <button 
                :class="['mode-btn', { active: mode === 'chain' }]" 
                @click="mode = 'chain'"
              >
                完整双向证书链 (CA + Server + Client)
                <span class="mode-desc">适用于 MySQL SSL / gRPC mTLS</span>
              </button>
            </div>
          </div>
        </div>

        <div class="config-row">
          <!-- Common Name -->
          <div class="config-item flex-2">
            <span class="config-label">常用名称 (Common Name / CN)</span>
            <JcInput beam glow v-model="commonName" placeholder="e.g. localhost 或 127.0.0.1" />
          </div>

          <!-- 有效天数 -->
          <div class="config-item flex-1">
            <span class="config-label">证书有效期 (Days)</span>
            <JcInputNumber :model-value="days" :min="1" suffix="天" placeholder="e.g. 365" size="small" beam glow @update:model-value="days = $event ?? 365" />
          </div>
        </div>

        <!-- SAN 域名与IP配置 -->
        <div class="config-row">
          <div class="config-item flex-1">
            <span class="config-label">Subject Alternative Names (SAN) - 决定浏览器信任与否的关键</span>
            <div class="san-input-wrapper">
              <JcInput beam glow v-model="newSan" placeholder="添加域名或 IP (如 test.local 或 192.168.1.100)，回车确认" @keyup.enter="addSanTag" style="flex: 1; min-width: 0" />
              <JcButton size="small" @click="addSanTag">添加</JcButton>
            </div>
            <div class="san-tags-container">
              <div v-for="(tag, index) in sans" :key="tag" class="san-tag">
                <span>{{ tag }}</span>
                <button class="remove-tag" @click="removeSanTag(index)">✕</button>
              </div>
              <div v-if="sans.length === 0" class="san-empty-tip">未添加 SAN (证书签署时会默认使用 localhost 和 127.0.0.1 兜底)</div>
            </div>
          </div>
        </div>

        <div class="config-row flex-wrap">
          <!-- 算法选择 -->
          <div class="config-item flex-2">
            <span class="config-label">加密算法</span>
            <div class="algo-selector">
              <button 
                :class="['algo-btn', { active: algorithm === 'rsa' }]" 
                @click="algorithm = 'rsa'"
              >
                RSA
              </button>
              <button 
                :class="['algo-btn', { active: algorithm === 'ecdsa' }]" 
                @click="algorithm = 'ecdsa'"
              >
                ECDSA (椭圆曲线)
              </button>
            </div>
          </div>

          <!-- 算法特定参数 -->
          <div v-if="algorithm === 'rsa'" class="config-item flex-1">
            <span class="config-label">密钥长度 (Bits)</span>
            <JcSelect beam glow :model-value="bits" :options="bitsOptions" style="width: 100%" @update:model-value="(v) => bits = Number(v)" />
          </div>

          <div v-if="algorithm === 'ecdsa'" class="config-item flex-1">
            <span class="config-label">曲线类型 (Curve)</span>
            <JcSelect beam glow v-model="curve" :options="curveOptions" style="width: 100%" />
          </div>
        </div>

        <div class="action-row">
          <JcButton type="primary" size="large" :loading="loading" @click="generate">
            {{ loading ? '正在调用系统 OpenSSL 生成证书...' : '立即生成自签名证书' }}
          </JcButton>
        </div>
      </div>

      <!-- 错误横幅 -->
      <div v-if="error" class="error-banner">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" class="error-icon"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
        <span class="error-text">{{ error }}</span>
      </div>

      <!-- 结果卡片 -->
      <div v-if="serverCert" class="results-container">
        <!-- 头部一键操作栏 -->
        <div class="results-actions-bar">
          <span class="status-success-tip">✔ 证书对生成成功！</span>
          <JcButton @click="downloadAll">一键下载全部 PEM 证书文件 (适用于 MySQL / Nginx 配置)</JcButton>
        </div>

        <!-- 1. 单证书模式下的布局 (双栏并排) -->
        <div v-if="mode === 'single'" class="results-grid">
          <!-- 证书 (Certificate) -->
          <div class="card result-card">
            <div class="result-header">
              <span class="result-title">服务器证书 (server.crt)</span>
              <JcButton size="small" @click="copyText('serverCert', serverCert)">{{ copySuccessMap['serverCert'] ? '已复制 ✔' : '复制证书' }}</JcButton>
            </div>
            <JcTextarea :model-value="serverCert" mono readonly beam glow :beam-size-ratio="0.6" class="jc-fill key-display" />
          </div>

          <!-- 私钥 (PrivateKey) -->
          <div class="card result-card">
            <div class="result-header">
              <span class="result-title">服务器私钥 (server.key)</span>
              <JcButton size="small" @click="copyText('serverKey', serverKey)">{{ copySuccessMap['serverKey'] ? '已复制 ✔' : '复制私钥' }}</JcButton>
            </div>
            <JcTextarea :model-value="serverKey" mono readonly beam glow :beam-size-ratio="0.6" class="jc-fill key-display" />
          </div>
        </div>

        <!-- 2. 证书链模式下的布局 (经典分栏导航器) -->
        <div v-else class="chain-workspace">
          <!-- 左侧文件导航 -->
          <div class="chain-sidebar">
            <div class="chain-sidebar-title">证书链文件</div>
            <div class="chain-file-list">
              <button 
                :class="['file-item-btn', { active: activeFile === 'ca' }]" 
                @click="activeFile = 'ca'"
              >
                <span class="file-icon">📄</span>
                <div class="file-meta">
                  <span class="file-name">ca.crt</span>
                  <span class="file-desc">Root CA 根证书</span>
                </div>
              </button>
              
              <button 
                :class="['file-item-btn', { active: activeFile === 'server_cert' }]" 
                @click="activeFile = 'server_cert'"
              >
                <span class="file-icon">📄</span>
                <div class="file-meta">
                  <span class="file-name">server.crt</span>
                  <span class="file-desc">MySQL/Web 服务端证书</span>
                </div>
              </button>

              <button 
                :class="['file-item-btn', { active: activeFile === 'server_key' }]" 
                @click="activeFile = 'server_key'"
              >
                <span class="file-icon">🔑</span>
                <div class="file-meta">
                  <span class="file-name">server.key</span>
                  <span class="file-desc">服务端私钥</span>
                </div>
              </button>

              <button 
                :class="['file-item-btn', { active: activeFile === 'client_cert' }]" 
                @click="activeFile = 'client_cert'"
              >
                <span class="file-icon">📄</span>
                <div class="file-meta">
                  <span class="file-name">client.crt</span>
                  <span class="file-desc">客户端连接证书</span>
                </div>
              </button>

              <button 
                :class="['file-item-btn', { active: activeFile === 'client_key' }]" 
                @click="activeFile = 'client_key'"
              >
                <span class="file-icon">🔑</span>
                <div class="file-meta">
                  <span class="file-name">client.key</span>
                  <span class="file-desc">客户端私钥</span>
                </div>
              </button>
            </div>
          </div>

          <!-- 右侧 PEM 文本展示 -->
          <div class="chain-viewer">
            <div class="viewer-header" v-if="activeFile === 'ca'">
              <span class="viewer-title">CA 根证书 (ca.crt) <span class="tip-inline">需要配置在 MySQL 的 <code>ssl-ca</code> 或客户端 CA 校验项</span></span>
              <JcButton size="small" @click="copyText('ca', caCert)">{{ copySuccessMap['ca'] ? '已复制 ✔' : '复制 CA 证书' }}</JcButton>
            </div>
            
            <div class="viewer-header" v-if="activeFile === 'server_cert'">
              <span class="viewer-title">服务端证书 (server.crt) <span class="tip-inline">需要配置在 MySQL 的 <code>ssl-cert</code></span></span>
              <JcButton size="small" @click="copyText('server_cert', serverCert)">{{ copySuccessMap['server_cert'] ? '已复制 ✔' : '复制服务端证书' }}</JcButton>
            </div>

            <div class="viewer-header" v-if="activeFile === 'server_key'">
              <span class="viewer-title">服务端私钥 (server.key) <span class="tip-inline">需要配置在 MySQL 的 <code>ssl-key</code></span></span>
              <JcButton size="small" @click="copyText('server_key', serverKey)">{{ copySuccessMap['server_key'] ? '已复制 ✔' : '复制服务端私钥' }}</JcButton>
            </div>

            <div class="viewer-header" v-if="activeFile === 'client_cert'">
              <span class="viewer-title">客户端证书 (client.crt) <span class="tip-inline">MySQL 双向认证时供客户端连接校验</span></span>
              <JcButton size="small" @click="copyText('client_cert', clientCert)">{{ copySuccessMap['client_cert'] ? '已复制 ✔' : '复制客户端证书' }}</JcButton>
            </div>

            <div class="viewer-header" v-if="activeFile === 'client_key'">
              <span class="viewer-title">客户端私钥 (client.key) <span class="tip-inline">MySQL 双向认证时客户端持有私钥</span></span>
              <JcButton size="small" @click="copyText('client_key', clientKey)">{{ copySuccessMap['client_key'] ? '已复制 ✔' : '复制客户端私钥' }}</JcButton>
            </div>

            <!-- 展示窗口 -->
            <JcTextarea v-if="activeFile === 'ca'" :model-value="caCert" mono readonly beam glow :beam-size-ratio="0.6" class="jc-fill key-display" />
            <JcTextarea v-else-if="activeFile === 'server_cert'" :model-value="serverCert" mono readonly beam glow :beam-size-ratio="0.6" class="jc-fill key-display" />
            <JcTextarea v-else-if="activeFile === 'server_key'" :model-value="serverKey" mono readonly beam glow :beam-size-ratio="0.6" class="jc-fill key-display" />
            <JcTextarea v-else-if="activeFile === 'client_cert'" :model-value="clientCert" mono readonly beam glow :beam-size-ratio="0.6" class="jc-fill key-display" />
            <JcTextarea v-else-if="activeFile === 'client_key'" :model-value="clientKey" mono readonly beam glow :beam-size-ratio="0.6" class="jc-fill key-display" />
          </div>
        </div>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped lang="scss">
.tool-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.card {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.config-card {
  border-left: 3px solid var(--jc-color-accent);
}

.card-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--jc-border-default);
  padding-bottom: 8px;
  margin-bottom: 4px;
}

.config-row {
  display: flex;
  gap: 16px;
  width: 100%;
  
  &.flex-wrap {
    flex-wrap: wrap;
  }
}

.config-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 200px;
}

.flex-1 { flex: 1; }
.flex-2 { flex: 2; }

.config-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  font-weight: 600;
}

.mode-selector {
  display: flex;
  gap: 12px;
  width: 100%;
}

.mode-btn {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
  background: var(--jc-bg-btn);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 10px 14px;
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.2s ease;
  text-align: left;
  
  &:hover {
    background: var(--jc-bg-btn-hover);
    border-color: var(--jc-color-accent);
  }
  
  &.active {
    background: var(--jc-bg-selected);
    border-color: var(--jc-color-accent);
    box-shadow: 0 0 0 1px var(--jc-color-accent);
    
    .mode-desc {
      color: var(--jc-text-primary);
    }
  }
}

.mode-desc {
  font-size: 10px;
  color: var(--jc-text-secondary);
}

.config-input {
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 7px 10px;
  font-size: 12px;
  outline: none;
  border-radius: 4px;
  font-family: inherit;
  
  &:focus {
    border-color: var(--jc-color-accent);
  }
}

.san-input-wrapper {
  display: flex;
  gap: 8px;
}

.san-tags-container {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  padding: 6px 10px;
  border-radius: 4px;
  min-height: 38px;
  align-items: center;
}

.san-tag {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  color: var(--jc-text-primary);
  
  .remove-tag {
    background: none;
    border: none;
    color: var(--jc-text-secondary);
    cursor: pointer;
    font-size: 10px;
    padding: 0 2px;
    
    &:hover {
      color: var(--jc-color-error);
    }
  }
}

.san-empty-tip {
  font-size: 10px;
  color: var(--jc-text-secondary);
  font-style: italic;
}

.algo-selector {
  display: flex;
  gap: 8px;
}

.algo-btn {
  flex: 1;
  background: var(--jc-bg-btn);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 8px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
  border-radius: 4px;
  
  &:hover {
    background: var(--jc-bg-btn-hover);
    border-color: var(--jc-color-accent);
  }
  
  &.active {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    border-color: var(--jc-color-accent);
  }
}

.action-row {
  margin-top: 8px;
  display: flex;
  justify-content: flex-end;
}

.error-banner {
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid var(--jc-color-error);
  color: var(--jc-color-error);
  padding: 10px 14px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.error-icon {
  flex-shrink: 0;
}

.results-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.results-actions-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 10px 16px;
  border-radius: 4px;
  
  .status-success-tip {
    font-size: 12px;
    font-weight: 600;
    color: var(--jc-color-success);
  }
}

.results-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  width: 100%;
  
  @media (max-width: 768px) {
    grid-template-columns: 1fr;
  }
}

.result-card {
  height: 380px;
  display: flex;
  flex-direction: column;
}

.result-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--jc-border-default);
  padding-bottom: 8px;
}

.result-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}

.key-display {
  flex: 1;
  width: 100%;
  resize: none;
}

// 证书链模式的工作区样式
.chain-workspace {
  display: flex;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  height: 480px;
  border-radius: 4px;
  overflow: hidden;
}

.chain-sidebar {
  width: 240px;
  border-right: 1px solid var(--jc-border-default);
  display: flex;
  flex-direction: column;
  background: var(--jc-bg-panel);
  flex-shrink: 0;
}

.chain-sidebar-title {
  padding: 12px 16px;
  font-size: 11px;
  font-weight: 600;
  color: var(--jc-text-secondary);
  text-transform: uppercase;
  border-bottom: 1px solid var(--jc-border-default);
  letter-spacing: 0.5px;
}

.chain-file-list {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow-y: auto;
}

.file-item-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  background: none;
  border: none;
  border-bottom: 1px solid var(--jc-border-default);
  cursor: pointer;
  text-align: left;
  color: var(--jc-text-primary);
  width: 100%;
  transition: background 0.2s;
  
  &:hover {
    background: var(--jc-bg-hover);
  }
  
  &.active {
    background: var(--jc-bg-selected);
    border-left: 3px solid var(--jc-color-accent);
    padding-left: 13px; // 抵消 border 宽度
    
    .file-name {
      color: var(--jc-text-highlight);
    }
  }
}

.file-icon {
  font-size: 16px;
}

.file-meta {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.file-name {
  font-size: 12px;
  font-weight: 600;
  font-family: 'Cascadia Code', Consolas, monospace;
}

.file-desc {
  font-size: 10px;
  color: var(--jc-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chain-viewer {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 16px;
  background: var(--jc-bg-app);
}

.viewer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  flex-shrink: 0;
}

.viewer-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-highlight);
  display: flex;
  align-items: center;
  gap: 8px;
}

.tip-inline {
  font-size: 10px;
  font-weight: normal;
  color: var(--jc-text-secondary);
  
  code {
    background: var(--jc-bg-elevated);
    padding: 1px 3px;
    border-radius: 3px;
  }
}

.chain-display {
  flex: 1;
}
</style>
