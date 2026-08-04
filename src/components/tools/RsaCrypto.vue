<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcSegmented from '@/components/ui/JcSegmented.vue'

// 密钥生成配置
const keySize = ref(2048)
const keyUsage = ref<'encrypt' | 'sign'>('encrypt')

const keySizeOptions = [
  { label: '1024 bit (较快/低安全)', value: 1024 },
  { label: '2048 bit (推荐/标准)', value: 2048 },
  { label: '4096 bit (安全/慢)', value: 4096 }
]
const keyUsageOptions = [
  { label: '加解密 (RSA-OAEP)', value: 'encrypt' },
  { label: '签名验签 (RSASSA-PKCS1-v1_5)', value: 'sign' }
]
const tabOptions = [
  { label: '公钥加密 / 私钥解密 (Encryption)', value: 'enc-dec' },
  { label: '私钥签名 / 公钥验签 (Signature)', value: 'sign-verify' }
]

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
  <ToolShell title="RSA 加解密 &amp; 签名工具" subtitle="RSA 密钥生成、公钥加密、私钥解密、私钥签名与公钥验签 (Web Crypto API)">
    <div class="tool-body">
      <!-- 左栏：密钥配置与生成 -->
      <div class="left-pane card">
        <div class="card-title">RSA 密钥对管理</div>
        
        <div class="config-row">
          <div class="cfg-item">
            <label>密钥长度 (Key Size)</label>
            <JcSelect beam glow :model-value="keySize" :options="keySizeOptions" style="width: 100%" @update:model-value="(v) => keySize = Number(v)" />
          </div>
          <div class="cfg-item">
            <label>密钥用途 (Usage)</label>
            <JcSelect beam glow v-model="keyUsage" :options="keyUsageOptions" style="width: 100%" />
          </div>
        </div>

        <div class="btn-group">
          <JcButton type="primary" @click="generateKeyPair">生成 RSA 密钥对</JcButton>
          <JcButton danger @click="clearAll">全部清空</JcButton>
        </div>

        <!-- 公钥 PEM 框 -->
        <div class="pem-wrap">
          <div class="pem-header">
            <span>公钥 PEM (Public Key)</span>
            <JcButton size="small" @click="copyText(publicKeyPem)">复制</JcButton>
          </div>
          <JcTextarea v-model="publicKeyPem" mono beam glow :beam-size-ratio="0.6" :rows="5" :spellcheck="false" placeholder="此处显示生成的公钥，或粘贴您的 RSA 公钥 PEM..." />
        </div>

        <!-- 私钥 PEM 框 -->
        <div class="pem-wrap">
          <div class="pem-header">
            <span>私钥 PEM (Private Key)</span>
            <JcButton size="small" @click="copyText(privateKeyPem)">复制</JcButton>
          </div>
          <JcTextarea v-model="privateKeyPem" mono beam glow :beam-size-ratio="0.6" :rows="5" :spellcheck="false" placeholder="此处显示生成的私钥，或粘贴您的 RSA 私钥 PEM..." />
        </div>
      </div>

      <!-- 右栏：操作区域 -->
      <div class="right-pane">
        <!-- 切换 Tab -->
        <div class="card">
          <JcSegmented
            :model-value="activeTab"
            :options="tabOptions"
            @update:model-value="(v) => activeTab = v as 'enc-dec' | 'sign-verify'"
          />
        </div>

        <!-- 操作面板 -->
        <div class="card op-panel">
          <div class="data-field">
            <label>输入数据 / 明文 (Plain Text)</label>
            <JcTextarea v-model="inputText" beam glow :beam-size-ratio="0.6" :spellcheck="false" :rows="6" placeholder="输入要加密或签名的文本内容..." />
          </div>

          <!-- Tab 1: 加解密 -->
          <div v-if="activeTab === 'enc-dec'" class="tab-content">
            <div class="action-bar">
              <JcButton type="primary" @click="encryptData">▲ 公钥加密 (Encrypt)</JcButton>
              <JcButton @click="decryptData">▼ 私钥解密 (Decrypt)</JcButton>
            </div>
            
            <div class="data-field">
              <label>加密结果 / 密文 (Base64 Encrypted String)</label>
              <JcTextarea v-model="outputText" mono beam glow :beam-size-ratio="0.6" :spellcheck="false" :rows="6" placeholder="加密后的 Base64 文本..." />
            </div>
          </div>

          <!-- Tab 2: 签名验签 -->
          <div v-else class="tab-content">
            <div class="action-bar">
              <JcButton type="primary" @click="signData">🖋 私钥签名 (Sign)</JcButton>
              <JcButton @click="verifyData">🔍 公钥验签 (Verify)</JcButton>
              
              <!-- 验签状态指示器 -->
              <span v-if="verifyStatus === 'success'" class="status-tag success">✓ 验签通过 (Signature Valid)</span>
              <span v-else-if="verifyStatus === 'failed'" class="status-tag failed">✗ 验签失败 (Signature Invalid)</span>
            </div>

            <div class="data-field">
              <label>签名值 (Base64 Signature String)</label>
              <JcTextarea v-model="signatureText" mono beam glow :beam-size-ratio="0.6" :spellcheck="false" :rows="6" placeholder="生成的 Base64 签名串..." />
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
  </ToolShell>
</template>

<style scoped lang="scss">
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
.btn-group {
  display: flex;
  gap: 8px;
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

// 右栏操作
.right-pane {
  display: flex;
  flex-direction: column;
  gap: 16px;
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
