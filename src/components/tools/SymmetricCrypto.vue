<script setup lang="ts">
import { ref, watch } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcSegmented from '@/components/ui/JcSegmented.vue'
import {
  utf8ToBytes,
  bytesToUtf8,
  hexToBytes,
  bytesToHex,
  base64ToBytes,
  bytesToBase64,
  aesEcbEncrypt,
  aesEcbDecrypt,
  aesCbcEncrypt,
  aesCbcDecrypt,
  desEcbEncrypt,
  desEcbDecrypt,
  desCbcEncrypt,
  desCbcDecrypt
} from './crypto-helper'

// 配置参数
const algorithm = ref<'AES' | 'DES'>('AES')
const mode = ref<'ECB' | 'CBC'>('ECB')

const algorithmOptions = [
  { label: 'AES', value: 'AES' },
  { label: 'DES', value: 'DES' }
]
const modeOptions = [
  { label: 'ECB (电子密码本)', value: 'ECB' },
  { label: 'CBC (密码块链接)', value: 'CBC' }
]
const textFormatOptions = [
  { label: 'UTF-8 String', value: 'utf8' },
  { label: 'Hex String', value: 'hex' },
  { label: 'Base64', value: 'base64' }
]
const cipherFormatOptions = [
  { label: 'Base64 字符串', value: 'base64' },
  { label: '十六进制 Hex', value: 'hex' }
]

// 密钥与IV
const key = ref('1234567890123456') // 默认 16 字节 AES
const keyFormat = ref<'utf8' | 'hex' | 'base64'>('utf8')
const iv = ref('1234567890123456') // 默认 16 字节 IV
const ivFormat = ref<'utf8' | 'hex' | 'base64'>('utf8')

// 输入输出
const plainText = ref('Hello World! 对称加解密测试。')
const plainFormat = ref<'utf8' | 'hex' | 'base64'>('utf8')

const cipherText = ref('')
const cipherFormat = ref<'base64' | 'hex'>('base64')

const errorMsg = ref('')

// 自动根据算法调整默认 Key & IV 长度
watch(algorithm, (newAlgo) => {
  if (newAlgo === 'DES') {
    key.value = '12345678' // DES 8 字节
    iv.value = '12345678'
  } else {
    key.value = '1234567890123456' // AES 16 字节
    iv.value = '1234567890123456'
  }
})

// 解析 Key & IV 字节集
function parseInputBytes(val: string, format: 'utf8' | 'hex' | 'base64'): Uint8Array {
  if (!val) return new Uint8Array(0)
  if (format === 'hex') return hexToBytes(val)
  if (format === 'base64') return base64ToBytes(val)
  return utf8ToBytes(val)
}

// 1. 加密核心
async function handleEncrypt() {
  errorMsg.value = ''
  cipherText.value = ''
  try {
    const keyBytes = parseInputBytes(key.value, keyFormat.value)
    const ivBytes = parseInputBytes(iv.value, ivFormat.value)
    const dataBytes = parseInputBytes(plainText.value, plainFormat.value)

    if (keyBytes.length === 0) throw new Error('密钥不能为空')
    if (mode.value === 'CBC' && ivBytes.length === 0) throw new Error('CBC 模式下偏移量 (IV) 不能为空')

    let encrypted: Uint8Array

    if (algorithm.value === 'AES') {
      if (mode.value === 'ECB') {
        // AES 密钥需要 16/24/32 字节
        if (![16, 24, 32].includes(keyBytes.length)) {
          throw new Error('AES-ECB 密钥长度必须为 128/192/256 位 (16/24/32 字节)')
        }
        encrypted = aesEcbEncrypt(dataBytes, keyBytes)
      } else {
        encrypted = await aesCbcEncrypt(dataBytes, keyBytes, ivBytes)
      }
    } else {
      // DES 密钥必须是 8 字节 (64位)
      if (keyBytes.length !== 8) {
        throw new Error('DES 密钥长度必须为 8 字节')
      }
      if (mode.value === 'ECB') {
        encrypted = desEcbEncrypt(dataBytes, keyBytes)
      } else {
        encrypted = desCbcEncrypt(dataBytes, keyBytes, ivBytes)
      }
    }

    // 格式化输出
    if (cipherFormat.value === 'hex') {
      cipherText.value = bytesToHex(encrypted)
    } else {
      cipherText.value = bytesToBase64(encrypted)
    }
  } catch (err: any) {
    errorMsg.value = '加密失败: ' + err.message
  }
}

// 2. 解密核心
async function handleDecrypt() {
  errorMsg.value = ''
  try {
    const keyBytes = parseInputBytes(key.value, keyFormat.value)
    const ivBytes = parseInputBytes(iv.value, ivFormat.value)
    
    if (keyBytes.length === 0) throw new Error('密钥不能为空')
    if (mode.value === 'CBC' && ivBytes.length === 0) throw new Error('CBC 模式下偏移量 (IV) 不能为空')
    if (!cipherText.value.trim()) throw new Error('待解密密文不能为空')

    // 解析密文
    let cipherBytes: Uint8Array
    if (cipherFormat.value === 'hex') {
      cipherBytes = hexToBytes(cipherText.value)
    } else {
      cipherBytes = base64ToBytes(cipherText.value)
    }

    let decrypted: Uint8Array

    if (algorithm.value === 'AES') {
      if (mode.value === 'ECB') {
        if (![16, 24, 32].includes(keyBytes.length)) {
          throw new Error('AES-ECB 密钥长度必须为 128/192/256 位 (16/24/32 字节)')
        }
        decrypted = aesEcbDecrypt(cipherBytes, keyBytes)
      } else {
        decrypted = await aesCbcDecrypt(cipherBytes, keyBytes, ivBytes)
      }
    } else {
      if (keyBytes.length !== 8) {
        throw new Error('DES 密钥长度必须为 8 字节')
      }
      if (mode.value === 'ECB') {
        decrypted = desEcbDecrypt(cipherBytes, keyBytes)
      } else {
        decrypted = desCbcDecrypt(cipherBytes, keyBytes, ivBytes)
      }
    }

    // 格式化明文输出
    if (plainFormat.value === 'hex') {
      plainText.value = bytesToHex(decrypted)
    } else if (plainFormat.value === 'base64') {
      plainText.value = bytesToBase64(decrypted)
    } else {
      plainText.value = bytesToUtf8(decrypted)
    }
  } catch (err: any) {
    errorMsg.value = '解密失败: ' + err.message
  }
}

function clearAll() {
  plainText.value = ''
  cipherText.value = ''
  errorMsg.value = ''
}

function copyResult(text: string) {
  if (!text) return
  navigator.clipboard.writeText(text)
}
</script>

<template>
  <ToolShell title="AES / DES 对称加解密" subtitle="支持 AES(128/192/256) 与 DES 的 ECB/CBC 模式加解密">
    <div class="tool-body">
      <!-- 左侧：加解密算法配置栏 -->
      <div class="left-pane card">
        <div class="card-title">算法配置 (Config)</div>

        <div class="field-item">
          <label>算法 (Algorithm)</label>
          <JcSegmented :model-value="algorithm" :options="algorithmOptions" @update:model-value="(v) => algorithm = v as 'AES' | 'DES'" />
        </div>

        <div class="field-item">
          <label>模式 (Mode)</label>
          <JcSegmented :model-value="mode" :options="modeOptions" @update:model-value="(v) => mode = v as 'ECB' | 'CBC'" />
        </div>

        <div class="field-item">
          <div class="label-with-select">
            <label>密钥 (Key)</label>
            <JcSelect beam v-model="keyFormat" :options="textFormatOptions" size="small" />
          </div>
          <JcInput beam v-model="key" placeholder="输入加密密钥..." style="font-family: 'Cascadia Code', Consolas, monospace" />
          <div class="field-desc">
            {{ algorithm === 'AES' ? 'AES 密钥长度支持: 16字节(128位)/24字节(192位)/32字节(256位)' : 'DES 密钥长度固定为 8字节(64位)' }}
          </div>
        </div>

        <div class="field-item" v-show="mode === 'CBC'">
          <div class="label-with-select">
            <label>偏移量 (IV)</label>
            <JcSelect beam v-model="ivFormat" :options="textFormatOptions" size="small" />
          </div>
          <JcInput beam v-model="iv" placeholder="输入偏移量..." style="font-family: 'Cascadia Code', Consolas, monospace" />
          <div class="field-desc">
            {{ algorithm === 'AES' ? 'AES CBC 模式 IV 长度固定为 16 字节' : 'DES CBC 模式 IV 长度固定为 8 字节' }}
          </div>
        </div>

        <div class="act-buttons">
          <JcButton type="primary" block @click="handleEncrypt">▲ 文本加密 (Encrypt)</JcButton>
          <JcButton block @click="handleDecrypt">▼ 密文解密 (Decrypt)</JcButton>
          <JcButton danger block @click="clearAll">清空数据</JcButton>
        </div>
      </div>

      <!-- 右侧：数据输入输出区 -->
      <div class="right-pane">
        <!-- 明文框 -->
        <div class="card val-panel">
          <div class="panel-header">
            <span>明文数据 (Plaintext)</span>
            <div class="panel-acts">
              <JcSelect beam v-model="plainFormat" :options="textFormatOptions" size="small" />
              <JcButton size="small" @click="copyResult(plainText)">复制</JcButton>
            </div>
          </div>
          <JcTextarea v-model="plainText" beam :beam-size-ratio="0.6" :rows="7" :spellcheck="false" placeholder="在此输入需要加密的明文，或解密出的结果..." />
        </div>

        <!-- 密文框 -->
        <div class="card val-panel">
          <div class="panel-header">
            <span>密文数据 (Ciphertext)</span>
            <div class="panel-acts">
              <JcSelect beam v-model="cipherFormat" :options="cipherFormatOptions" size="small" />
              <JcButton size="small" @click="copyResult(cipherText)">复制</JcButton>
            </div>
          </div>
          <JcTextarea v-model="cipherText" mono beam :beam-size-ratio="0.6" :rows="7" :spellcheck="false" placeholder="在此输入需要解密的密文，或加密出的结果..." />
        </div>

        <!-- 错误提示 -->
        <div v-if="errorMsg" class="error-panel card">
          <span class="err-title">错误:</span>
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

// 属性配置
.field-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}
.label-with-select {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.field-desc {
  font-size: 10px;
  color: var(--jc-text-secondary);
  opacity: 0.85;
}
.act-buttons {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
}

// 右侧面板
.right-pane {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.val-panel {
  gap: 8px;
}
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
}
.panel-acts {
  display: flex;
  align-items: center;
  gap: 8px;
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
