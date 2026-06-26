<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const algorithm = ref<'ed25519' | 'rsa' | 'ecdsa'>('ed25519')
const bits = ref<number>(4096)
const curve = ref<number>(256)
const passphrase = ref('')
const comment = ref('')
const loading = ref(false)
const error = ref('')
const privateKey = ref('')
const publicKey = ref('')
const showPassphrase = ref(false)

const copyPrivateSuccess = ref(false)
const copyPublicSuccess = ref(false)

async function generate() {
  loading.value = true
  error.value = ''
  privateKey.value = ''
  publicKey.value = ''
  
  let b = 2048
  if (algorithm.value === 'rsa') {
    b = bits.value
  } else if (algorithm.value === 'ecdsa') {
    b = curve.value
  }
  
  try {
    const res = await invoke<{ privateKey: string, publicKey: string }>('generate_ssh_key', {
      algorithm: algorithm.value,
      bits: b,
      passphrase: passphrase.value,
      comment: comment.value
    })
    privateKey.value = res.privateKey
    publicKey.value = res.publicKey
  } catch (err: any) {
    error.value = err.toString()
  } finally {
    loading.value = false
  }
}

function copyPrivate() {
  if (!privateKey.value) return
  navigator.clipboard.writeText(privateKey.value)
  copyPrivateSuccess.value = true
  setTimeout(() => {
    copyPrivateSuccess.value = false
  }, 2000)
}

function copyPublic() {
  if (!publicKey.value) return
  navigator.clipboard.writeText(publicKey.value)
  copyPublicSuccess.value = true
  setTimeout(() => {
    copyPublicSuccess.value = false
  }, 2000)
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">SSH 密钥生成器</div>
    </div>
    
    <div class="tool-body">
      <!-- 配置面板 -->
      <div class="card config-card">
        <div class="card-title">密钥算法与参数</div>
        
        <div class="config-row flex-wrap">
          <!-- 算法选择 -->
          <div class="config-item flex-2">
            <span class="config-label">算法选择</span>
            <div class="algo-selector">
              <button 
                :class="['algo-btn', { active: algorithm === 'ed25519' }]" 
                @click="algorithm = 'ed25519'"
              >
                Ed25519
                <span class="badge recommend">推荐</span>
              </button>
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
                ECDSA
              </button>
            </div>
          </div>

          <!-- 动态参数选项 (RSA位数/ECDSA曲线) -->
          <div v-if="algorithm === 'rsa'" class="config-item flex-1">
            <span class="config-label">密钥长度 (Bits)</span>
            <select v-model="bits" class="config-select">
              <option :value="2048">2048 位</option>
              <option :value="3072">3072 位</option>
              <option :value="4096">4096 位 (安全)</option>
            </select>
          </div>

          <div v-if="algorithm === 'ecdsa'" class="config-item flex-1">
            <span class="config-label">曲线类型 (Curve)</span>
            <select v-model="curve" class="config-select">
              <option :value="256">nistp256 (256 位)</option>
              <option :value="384">nistp384 (384 位)</option>
              <option :value="521">nistp521 (521 位)</option>
            </select>
          </div>
        </div>

        <div class="config-row">
          <!-- 注释 (Comment) -->
          <div class="config-item flex-1">
            <span class="config-label">注释 (Comment - 比如邮箱)</span>
            <input 
              v-model="comment" 
              placeholder="e.g. your-email@example.com (留空则不加)" 
              class="config-input"
            />
          </div>

          <!-- 保护密码 (Passphrase) -->
          <div class="config-item flex-1">
            <span class="config-label">密钥保护密码 (Passphrase)</span>
            <div class="password-wrapper">
              <input 
                :type="showPassphrase ? 'text' : 'password'" 
                v-model="passphrase" 
                placeholder="留空表示不设置密码保护" 
                class="config-input password-input"
              />
              <button 
                type="button" 
                class="eye-btn" 
                @click="showPassphrase = !showPassphrase"
                :title="showPassphrase ? '隐藏密码' : '显示密码'"
              >
                <svg v-if="showPassphrase" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2"><path d="M9.88 9.88a3 3 0 1 0 4.24 4.24M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68M6.61 6.61A13.52 13.52 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61M2 2l20 20"/></svg>
                <svg v-else viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>
              </button>
            </div>
          </div>
        </div>

        <div class="action-row">
          <button 
            class="tool-btn pri generate-btn" 
            :disabled="loading" 
            @click="generate"
          >
            <span v-if="loading">正在调用系统生成密钥对...</span>
            <span v-else>立即生成密钥对</span>
          </button>
        </div>
      </div>

      <!-- 错误横幅 -->
      <div v-if="error" class="error-banner">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" class="error-icon"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
        <span class="error-text">{{ error }}</span>
      </div>

      <!-- 结果卡片 -->
      <div v-if="privateKey || publicKey" class="results-grid">
        <!-- 私钥 (Private Key) -->
        <div class="card result-card">
          <div class="result-header">
            <span class="result-title">私钥 (Private Key)</span>
            <button 
              class="now-copy-btn" 
              :class="{ success: copyPrivateSuccess }"
              @click="copyPrivate"
            >
              {{ copyPrivateSuccess ? '已复制 ✔' : '复制私钥' }}
            </button>
          </div>
          <textarea 
            readonly 
            class="key-display mono-display" 
            :value="privateKey"
            placeholder="尚未生成私钥"
          ></textarea>
          <span class="security-tip">提示：私钥必须严密保管，绝对不能泄露给任何人。</span>
        </div>

        <!-- 公钥 (Public Key) -->
        <div class="card result-card">
          <div class="result-header">
            <span class="result-title">公钥 (Public Key)</span>
            <button 
              class="now-copy-btn" 
              :class="{ success: copyPublicSuccess }"
              @click="copyPublic"
            >
              {{ copyPublicSuccess ? '已复制 ✔' : '复制公钥' }}
            </button>
          </div>
          <textarea 
            readonly 
            class="key-display mono-display" 
            :value="publicKey"
            placeholder="尚未生成公钥"
          ></textarea>
          <span class="security-tip">提示：公钥可以安全地共享、或配置在服务器的 <code>~/.ssh/authorized_keys</code> 中。</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.tool-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  padding: 16px;
  background: var(--jc-bg-app);
  overflow-y: auto;
}

.tool-header {
  margin-bottom: 16px;
  flex-shrink: 0;
}

.tool-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}

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

.algo-selector {
  display: flex;
  gap: 8px;
}

.algo-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  background: var(--jc-bg-btn);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 8px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
  border-radius: 4px;
  position: relative;
  
  &:hover {
    background: var(--jc-bg-btn-hover);
    border-color: var(--jc-color-accent);
  }
  
  &.active {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    border-color: var(--jc-color-accent);
    
    .badge {
      background: rgba(255, 255, 255, 0.2);
      color: #fff;
    }
  }
}

.badge {
  font-size: 9px;
  padding: 1px 4px;
  border-radius: 3px;
  
  &.recommend {
    background: var(--jc-bg-selected);
    color: var(--jc-color-success);
    font-weight: 700;
  }
}

.config-select {
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 7px 10px;
  font-size: 12px;
  outline: none;
  border-radius: 4px;
  
  &:focus {
    border-color: var(--jc-color-accent);
  }
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

.password-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

.password-input {
  padding-right: 36px;
}

.eye-btn {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  
  &:hover {
    color: var(--jc-text-primary);
  }
}

.action-row {
  margin-top: 8px;
  display: flex;
  justify-content: flex-end;
}

.generate-btn {
  min-width: 180px;
  padding: 10px 20px;
  font-size: 12px;
  font-weight: 600;
  border-radius: 4px;
}

.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.2s;
  
  &:hover:not(:disabled) {
    background: var(--jc-bg-btn-hover);
  }
  
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  &.pri {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    
    &:hover:not(:disabled) {
      background: var(--jc-color-accent-hover, #007acc);
    }
  }
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

.now-copy-btn {
  background: var(--jc-bg-btn);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 4px 10px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.2s;
  
  &:hover {
    background: var(--jc-bg-hover);
    border-color: var(--jc-color-accent);
  }
  
  &.success {
    background: var(--jc-color-success);
    color: var(--jc-color-white);
    border-color: var(--jc-color-success);
  }
}

.key-display {
  flex: 1;
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 10px;
  font-size: 11px;
  outline: none;
  resize: none;
  border-radius: 4px;
  
  &:focus {
    border-color: var(--jc-color-accent);
  }
}

.mono-display {
  font-family: 'Cascadia Code', Consolas, monospace;
  line-height: 1.4;
}

.security-tip {
  font-size: 10px;
  color: var(--jc-text-secondary);
  code {
    background: var(--jc-bg-elevated);
    padding: 1px 3px;
    border-radius: 3px;
  }
}
</style>
