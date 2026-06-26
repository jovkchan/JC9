<script setup lang="ts">
import { ref } from 'vue'

// 密钥生成配置
const keySize = ref(2048)
const keyUsage = ref<'encrypt' | 'sign'>('encrypt')

// 密钥 PEM
const publicKeyPem = ref('')
const privateKeyPem = ref('')

// 操作数据
const inputText = ref('Hello, Antigravity RSA!')
const outputText = ref('')
const signatureText = ref('')
const verifyStatus = ref<'idle' | 'success' | 'failed'>('idle')
const errorMsg = ref('')

const activeTab = ref<'enc-dec' | 'sign-verify'>('enc-dec')

// ArrayBuffer & Base64 / String 辅助函数
function arrayBufferToBase64(buffer: ArrayBuffer): string {
  let binary = ''
  const bytes = new Uint8Array(buffer)
  const len = bytes.byteLength
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return window.btoa(binary)
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const binaryString = window.atob(base64)
  const len = binaryString.length
  const bytes = new Uint8Array(len)
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i)
  }
  return bytes.buffer
}

function formatAsPem(base64: string, type: 'public' | 'private'): string {
  const lines = []
  for (let i = 0; i < base64.length; i += 64) {
    lines.push(base64.substring(i, i + 64))
  }
  const body = lines.join('\n')
  if (type === 'public') {
    return `-----BEGIN PUBLIC KEY-----\n${body}\n-----END PUBLIC KEY-----`
  } else {
    return `-----BEGIN PRIVATE KEY-----\n${body}\n-----END PRIVATE KEY-----`
  }
}

function parsePem(pem: string): string {
  return pem
    .replace(/-----BEGIN[^-]*-----/g, '')
    .replace(/-----END[^-]*-----/g, '')
    .replace(/\s+/g, '')
}

// 1. 生成密钥对
async function generateKeyPair() {
  errorMsg.value = ''
  try {
    const algorithm = keyUsage.value === 'encrypt' 
      ? {
          name: 'RSA-OAEP',
          modulusLength: keySize.value,
          publicExponent: new Uint8Array([1, 0, 1]),
          hash: 'SHA-256'
        }
      : {
          name: 'RSASSA-PKCS1-v1_5',
          modulusLength: keySize.value,
          publicExponent: new Uint8Array([1, 0, 1]),
          hash: 'SHA-256'
        }

    const usages: KeyUsage[] = keyUsage.value === 'encrypt' 
      ? ['encrypt', 'decrypt'] 
      : ['sign', 'verify']

    const keyPair = await window.crypto.subtle.generateKey(
      algorithm,
      true, // 是否可导出
      usages
    )

    // 导出公钥 (spki)
    const pubBuffer = await window.crypto.subtle.exportKey('spki', keyPair.publicKey)
    publicKeyPem.value = formatAsPem(arrayBufferToBase64(pubBuffer), 'public')

    // 导出私钥 (pkcs8)
    const privBuffer = await window.crypto.subtle.exportKey('pkcs8', keyPair.privateKey)
    privateKeyPem.value = formatAsPem(arrayBufferToBase64(privBuffer), 'private')
  } catch (err: any) {
    errorMsg.value = '生成密钥对失败: ' + err.message
  }
}

// 2. 导入公钥
async function importPublicKey(usage: 'encrypt' | 'verify'): Promise<CryptoKey> {
  const clean = parsePem(publicKeyPem.value)
  if (!clean) throw new Error('公钥 PEM 不能为空')
  const buffer = base64ToArrayBuffer(clean)

  const algorithm = usage === 'encrypt'
    ? { name: 'RSA-OAEP', hash: 'SHA-256' }
    : { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }

  return await window.crypto.subtle.importKey(
    'spki',
    buffer,
    algorithm,
    true,
    usage === 'encrypt' ? ['encrypt'] : ['verify']
  )
}

// 3. 导入私钥
async function importPrivateKey(usage: 'decrypt' | 'sign'): Promise<CryptoKey> {
  const clean = parsePem(privateKeyPem.value)
  if (!clean) throw new Error('私钥 PEM 不能为空')
  const buffer = base64ToArrayBuffer(clean)

  const algorithm = usage === 'decrypt'
    ? { name: 'RSA-OAEP', hash: 'SHA-256' }
    : { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }

  return await window.crypto.subtle.importKey(
    'pkcs8',
    buffer,
    algorithm,
    true,
    usage === 'decrypt' ? ['decrypt'] : ['sign']
  )
}

// 4. 公钥加密
async function encryptData() {
  errorMsg.value = ''
  outputText.value = ''
  try {
    const key = await importPublicKey('encrypt')
    const encoder = new TextEncoder()
    const data = encoder.encode(inputText.value)
    const encBuffer = await window.crypto.subtle.encrypt(
      { name: 'RSA-OAEP' },
      key,
      data
    )
    outputText.value = arrayBufferToBase64(encBuffer)
  } catch (err: any) {
    errorMsg.value = '加密失败: ' + err.message
  }
}

// 5. 私钥解密
async function decryptData() {
  errorMsg.value = ''
  try {
    const key = await importPrivateKey('decrypt')
    const data = base64ToArrayBuffer(outputText.value.trim())
    const decBuffer = await window.crypto.subtle.decrypt(
      { name: 'RSA-OAEP' },
      key,
      data
    )
    const decoder = new TextDecoder()
    inputText.value = decoder.decode(decBuffer)
  } catch (err: any) {
    errorMsg.value = '解密失败: ' + err.message
  }
}

// 6. 私钥签名
async function signData() {
  errorMsg.value = ''
  signatureText.value = ''
  try {
    const key = await importPrivateKey('sign')
    const encoder = new TextEncoder()
    const data = encoder.encode(inputText.value)
    const sigBuffer = await window.crypto.subtle.sign(
      { name: 'RSASSA-PKCS1-v1_5' },
      key,
      data
    )
    signatureText.value = arrayBufferToBase64(sigBuffer)
  } catch (err: any) {
    errorMsg.value = '签名失败: ' + err.message
  }
}

// 7. 公钥验签
async function verifyData() {
  errorMsg.value = ''
  verifyStatus.value = 'idle'
  try {
    const key = await importPublicKey('verify')
    const encoder = new TextEncoder()
    const data = encoder.encode(inputText.value)
    const signature = base64ToArrayBuffer(signatureText.value.trim())
    const isValid = await window.crypto.subtle.verify(
      { name: 'RSASSA-PKCS1-v1_5' },
      key,
      signature,
      data
    )
    verifyStatus.value = isValid ? 'success' : 'failed'
  } catch (err: any) {
    errorMsg.value = '验签异常: ' + err.message
    verifyStatus.value = 'failed'
  }
}

function copyText(text: string) {
  if (!text) return
  navigator.clipboard.writeText(text)
}

function clearAll() {
  publicKeyPem.value = ''
  privateKeyPem.value = ''
  inputText.value = ''
  outputText.value = ''
  signatureText.value = ''
  verifyStatus.value = 'idle'
  errorMsg.value = ''
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">RSA 加解密 &amp; 签名工具 (RSA Crypto Tool)</div>
      <div class="tool-desc-header">RSA 密钥生成、公钥加密、私钥解密、私钥签名与公钥验签 (Web Crypto API)</div>
    </div>

    <div class="tool-body">
      <!-- 左栏：密钥配置与生成 -->
      <div class="left-pane card">
        <div class="card-title">RSA 密钥对管理</div>
        
        <div class="config-row">
          <div class="cfg-item">
            <label>密钥长度 (Key Size)</label>
            <select v-model="keySize" class="tool-select">
              <option :value="1024">1024 bit (较快/低安全)</option>
              <option :value="2048">2048 bit (推荐/标准)</option>
              <option :value="4096">4096 bit (安全/慢)</option>
            </select>
          </div>
          <div class="cfg-item">
            <label>密钥用途 (Usage)</label>
            <select v-model="keyUsage" class="tool-select">
              <option value="encrypt">加解密 (RSA-OAEP)</option>
              <option value="sign">签名验签 (RSASSA-PKCS1-v1_5)</option>
            </select>
          </div>
        </div>

        <div class="btn-group">
          <button class="tool-btn pri" @click="generateKeyPair">生成 RSA 密钥对</button>
          <button class="tool-btn err" @click="clearAll">全部清空</button>
        </div>

        <!-- 公钥 PEM 框 -->
        <div class="pem-wrap">
          <div class="pem-header">
            <span>公钥 PEM (Public Key)</span>
            <button class="btn-copy" @click="copyText(publicKeyPem)">复制</button>
          </div>
          <textarea v-model="publicKeyPem" placeholder="此处显示生成的公钥，或粘贴您的 RSA 公钥 PEM..." spellcheck="false" class="code-font"></textarea>
        </div>

        <!-- 私钥 PEM 框 -->
        <div class="pem-wrap">
          <div class="pem-header">
            <span>私钥 PEM (Private Key)</span>
            <button class="btn-copy" @click="copyText(privateKeyPem)">复制</button>
          </div>
          <textarea v-model="privateKeyPem" placeholder="此处显示生成的私钥，或粘贴您的 RSA 私钥 PEM..." spellcheck="false" class="code-font"></textarea>
        </div>
      </div>

      <!-- 右栏：操作区域 -->
      <div class="right-pane">
        <!-- 切换 Tab -->
        <div class="tab-header card">
          <div class="tabs">
            <button :class="{ active: activeTab === 'enc-dec' }" @click="activeTab = 'enc-dec'">
              公钥加密 / 私钥解密 (Encryption)
            </button>
            <button :class="{ active: activeTab === 'sign-verify' }" @click="activeTab = 'sign-verify'">
              私钥签名 / 公钥验签 (Signature)
            </button>
          </div>
        </div>

        <!-- 操作面板 -->
        <div class="card op-panel">
          <div class="data-field">
            <label>输入数据 / 明文 (Plain Text)</label>
            <textarea v-model="inputText" placeholder="输入要加密或签名的文本内容..." spellcheck="false"></textarea>
          </div>

          <!-- Tab 1: 加解密 -->
          <div v-if="activeTab === 'enc-dec'" class="tab-content">
            <div class="action-bar">
              <button class="tool-btn pri" @click="encryptData">▲ 公钥加密 (Encrypt)</button>
              <button class="tool-btn" @click="decryptData">▼ 私钥解密 (Decrypt)</button>
            </div>
            
            <div class="data-field">
              <label>加密结果 / 密文 (Base64 Encrypted String)</label>
              <textarea v-model="outputText" placeholder="加密后的 Base64 文本..." spellcheck="false" class="code-font"></textarea>
            </div>
          </div>

          <!-- Tab 2: 签名验签 -->
          <div v-else class="tab-content">
            <div class="action-bar">
              <button class="tool-btn pri" @click="signData">🖋 私钥签名 (Sign)</button>
              <button class="tool-btn" @click="verifyData">🔍 公钥验签 (Verify)</button>
              
              <!-- 验签状态指示器 -->
              <span v-if="verifyStatus === 'success'" class="status-tag success">✓ 验签通过 (Signature Valid)</span>
              <span v-else-if="verifyStatus === 'failed'" class="status-tag failed">✗ 验签失败 (Signature Invalid)</span>
            </div>

            <div class="data-field">
              <label>签名值 (Base64 Signature String)</label>
              <textarea v-model="signatureText" placeholder="生成的 Base64 签名串..." spellcheck="false" class="code-font"></textarea>
            </div>
          </div>
        </div>

        <!-- 错误提示 -->
        <div v-if="errorMsg" class="error-panel card">
          <span class="err-title">执行异常:</span>
          <span class="err-desc">{{ errorMsg }}</span>
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
  gap: 16px;
}
.tool-header {
  flex-shrink: 0;
  border-left: 3px solid var(--jc-color-accent);
  padding-left: 10px;
}
.tool-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}
.tool-desc-header {
  font-size: 12px;
  color: var(--jc-text-secondary);
  margin-top: 2px;
}
.tool-body {
  display: grid;
  grid-template-columns: 1fr 1.2fr;
  gap: 16px;
  align-items: start;
  max-width: 1250px;
  @media (max-width: 950px) {
    grid-template-columns: 1fr;
  }
}
.card {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.card-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-primary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--jc-border-strong);
  padding-bottom: 6px;
}

// 密钥配置
.config-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.cfg-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}
.tool-select {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 6px 10px;
  font-size: 12px;
  outline: none;
  border-radius: 4px;
  cursor: pointer;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.btn-group {
  display: flex;
  gap: 8px;
}
.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 8px 16px;
  font-size: 12px;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.2s;
  &:hover {
    background: var(--jc-bg-btn-hover);
  }
  &.pri {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    &:hover {
      background: var(--jc-color-accent-hover);
    }
  }
  &.err {
    &:hover {
      background: var(--jc-color-error);
      color: var(--jc-color-white);
    }
  }
}

// PEM 框
.pem-wrap {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.pem-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
}
.btn-copy {
  background: none;
  border: none;
  color: var(--jc-color-accent);
  cursor: pointer;
  font-size: 11px;
  &:hover {
    text-decoration: underline;
  }
}
textarea {
  width: 100%;
  height: 120px;
  resize: vertical;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  color: var(--jc-text-primary);
  padding: 8px;
  font-size: 12px;
  line-height: 1.4;
  outline: none;
  &:focus {
    border-color: var(--jc-color-accent);
  }
  &.code-font {
    font-family: 'Cascadia Code', Consolas, monospace;
  }
}

// 右栏操作
.right-pane {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.tab-header {
  padding: 0;
  overflow: hidden;
}
.tabs {
  display: flex;
  background: var(--jc-bg-elevated);
  button {
    flex: 1;
    background: none;
    border: none;
    padding: 10px 16px;
    font-size: 12px;
    color: var(--jc-text-secondary);
    cursor: pointer;
    transition: all 0.2s;
    &:hover {
      background: var(--jc-bg-hover);
      color: var(--jc-text-primary);
    }
    &.active {
      background: var(--jc-bg-panel);
      color: var(--jc-color-accent);
      font-weight: 600;
    }
  }
}
.op-panel {
  gap: 16px;
}
.data-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
  textarea {
    height: 140px;
  }
}
.tab-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.action-bar {
  display: flex;
  align-items: center;
  gap: 12px;
}
.status-tag {
  font-size: 11px;
  font-weight: 600;
  padding: 4px 10px;
  border-radius: 12px;
  &.success {
    background: rgba(16, 185, 129, 0.15);
    color: var(--jc-color-success);
  }
  &.failed {
    background: rgba(239, 68, 68, 0.15);
    color: var(--jc-color-error);
  }
}

// 错误提示
.error-panel {
  background: rgba(239, 68, 68, 0.1);
  border-color: rgba(239, 68, 68, 0.2);
  color: var(--jc-color-error);
  font-size: 11px;
  padding: 10px 14px;
  flex-direction: row;
  gap: 6px;
  .err-title {
    font-weight: bold;
  }
}
</style>
