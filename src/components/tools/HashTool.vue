<script setup lang="ts">
import { ref, watch } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcSegmented from '@/components/ui/JcSegmented.vue'

const activeTab = ref<'text' | 'file'>('text')
const mode = ref<'md5' | 'sha1' | 'sha256' | 'sha512' | 'sm3'>('sha256')
const inputText = ref('')
const outputHash = ref('')
const errorMsg = ref('')

// 文件计算相关状态
const selectedFile = ref<File | null>(null)
const calculateProgress = ref(0)
const isCalculating = ref(false)
const dragOver = ref(false)
const cancelRequested = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

const algorithms: Record<string, string> = {
  md5: 'MD5',
  sha1: 'SHA-1',
  sha256: 'SHA-256',
  sha512: 'SHA-512',
  sm3: 'SM3 (国密)'
}

const algoOptions = Object.entries(algorithms).map(([value, label]) => ({ value, label }))
const tabOptions = [
  { label: '文本哈希计算', value: 'text' },
  { label: '大文件校验和 (防 OOM)', value: 'file' }
]

// ==========================================
// 1. 增量 MD5 纯 JS 实现
// ==========================================
class Md5Hasher {
  private h = new Uint32Array([0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476])
  private buffer = new Uint8Array(64)
  private bufferLen = 0
  private totalLen = 0

  private K = new Uint32Array([
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391
  ])

  private S = [
    7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
    5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
    4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
    6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21
  ]

  private rol(n: number, c: number) {
    return (n << c) | (n >>> (32 - c))
  }

  update(data: Uint8Array) {
    this.totalLen += data.length
    let offset = 0
    while (offset < data.length) {
      const needed = 64 - this.bufferLen
      const copyLen = Math.min(needed, data.length - offset)
      this.buffer.set(data.subarray(offset, offset + copyLen), this.bufferLen)
      this.bufferLen += copyLen
      offset += copyLen

      if (this.bufferLen === 64) {
        this.compress(this.buffer)
        this.bufferLen = 0
      }
    }
  }

  private compress(block: Uint8Array) {
    const W = new Uint32Array(16)
    for (let i = 0; i < 16; i++) {
      W[i] = block[i * 4] | (block[i * 4 + 1] << 8) | (block[i * 4 + 2] << 16) | (block[i * 4 + 3] << 24)
    }

    let a = this.h[0], b = this.h[1], c = this.h[2], d = this.h[3]

    for (let i = 0; i < 64; i++) {
      let f = 0, g = 0
      if (i < 16) {
        f = (b & c) | (~b & d)
        g = i
      } else if (i < 32) {
        f = (d & b) | (~d & c)
        g = (5 * i + 1) % 16
      } else if (i < 48) {
        f = b ^ c ^ d
        g = (3 * i + 5) % 16
      } else {
        f = c ^ (b | ~d)
        g = (7 * i) % 16
      }

      const temp = d
      d = c
      c = b
      b = (b + this.rol(a + f + this.K[i] + W[g], this.S[i])) | 0
      a = temp
    }

    this.h[0] = (this.h[0] + a) | 0
    this.h[1] = (this.h[1] + b) | 0
    this.h[2] = (this.h[2] + c) | 0
    this.h[3] = (this.h[3] + d) | 0
  }

  finalize(): string {
    const totalBits = this.totalLen * 8
    this.update(new Uint8Array([0x80]))

    // 填充 0，留下 8 字节存长度
    const padLen = this.bufferLen <= 56 ? 56 - this.bufferLen : 120 - this.bufferLen
    const pad = new Uint8Array(padLen)
    this.update(pad)

    // 追加 64 位长度 (小端)
    const lenBytes = new Uint8Array(8)
    let temp = totalBits
    for (let i = 0; i < 8; i++) {
      lenBytes[i] = temp & 0xff
      temp = Math.floor(temp / 256)
    }
    this.update(lenBytes)

    return Array.from(this.h)
      .map(val => {
        let s = ''
        for (let i = 0; i < 4; i++) {
          s += ((val >>> (i * 8)) & 0xff).toString(16).padStart(2, '0')
        }
        return s
      })
      .join('')
  }
}

// ==========================================
// 2. 增量 SHA-1 纯 JS 实现
// ==========================================
class Sha1Hasher {
  private h = new Uint32Array([0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0])
  private buffer = new Uint8Array(64)
  private bufferLen = 0
  private totalLen = 0

  private rol(n: number, c: number) {
    return (n << c) | (n >>> (32 - c))
  }

  update(data: Uint8Array) {
    this.totalLen += data.length
    let offset = 0
    while (offset < data.length) {
      const needed = 64 - this.bufferLen
      const copyLen = Math.min(needed, data.length - offset)
      this.buffer.set(data.subarray(offset, offset + copyLen), this.bufferLen)
      this.bufferLen += copyLen
      offset += copyLen

      if (this.bufferLen === 64) {
        this.compress(this.buffer)
        this.bufferLen = 0
      }
    }
  }

  private compress(block: Uint8Array) {
    const W = new Uint32Array(80)
    for (let i = 0; i < 16; i++) {
      W[i] = (block[i * 4] << 24) | (block[i * 4 + 1] << 16) | (block[i * 4 + 2] << 8) | block[i * 4 + 3]
    }
    for (let i = 16; i < 80; i++) {
      W[i] = this.rol(W[i - 3] ^ W[i - 8] ^ W[i - 14] ^ W[i - 16], 1)
    }

    let a = this.h[0], b = this.h[1], c = this.h[2], d = this.h[3], e = this.h[4]

    for (let i = 0; i < 80; i++) {
      let f = 0, k = 0
      if (i < 20) {
        f = (b & c) | (~b & d)
        k = 0x5a827999
      } else if (i < 40) {
        f = b ^ c ^ d
        k = 0x6ed9eba1
      } else if (i < 60) {
        f = (b & c) | (b & d) | (c & d)
        k = 0x8f1bbcdc
      } else {
        f = b ^ c ^ d
        k = 0xca62c1d6
      }

      const temp = (this.rol(a, 5) + f + e + k + W[i]) | 0
      e = d
      d = c
      c = this.rol(b, 30)
      b = a
      a = temp
    }

    this.h[0] = (this.h[0] + a) | 0
    this.h[1] = (this.h[1] + b) | 0
    this.h[2] = (this.h[2] + c) | 0
    this.h[3] = (this.h[3] + d) | 0
    this.h[4] = (this.h[4] + e) | 0
  }

  finalize(): string {
    const totalBits = this.totalLen * 8
    this.update(new Uint8Array([0x80]))

    const padLen = this.bufferLen <= 56 ? 56 - this.bufferLen : 120 - this.bufferLen
    const pad = new Uint8Array(padLen)
    this.update(pad)

    const lenBytes = new Uint8Array(8)
    let temp = totalBits
    for (let i = 7; i >= 0; i--) {
      lenBytes[i] = temp & 0xff
      temp = Math.floor(temp / 256)
    }
    this.update(lenBytes)

    return Array.from(this.h)
      .map(val => val.toString(16).padStart(8, '0'))
      .join('')
  }
}

// ==========================================
// 3. 增量 SHA-256 纯 JS 实现
// ==========================================
class Sha256Hasher {
  private h = new Uint32Array([
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
  ])
  private buffer = new Uint8Array(64)
  private bufferLen = 0
  private totalLen = 0

  private K = new Uint32Array([
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664d, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
  ])

  private rotr(n: number, c: number) {
    return (n >>> c) | (n << (32 - c))
  }

  update(data: Uint8Array) {
    this.totalLen += data.length
    let offset = 0
    while (offset < data.length) {
      const needed = 64 - this.bufferLen
      const copyLen = Math.min(needed, data.length - offset)
      this.buffer.set(data.subarray(offset, offset + copyLen), this.bufferLen)
      this.bufferLen += copyLen
      offset += copyLen

      if (this.bufferLen === 64) {
        this.compress(this.buffer)
        this.bufferLen = 0
      }
    }
  }

  private compress(block: Uint8Array) {
    const W = new Uint32Array(64)
    for (let i = 0; i < 16; i++) {
      W[i] = (block[i * 4] << 24) | (block[i * 4 + 1] << 16) | (block[i * 4 + 2] << 8) | block[i * 4 + 3]
    }
    for (let i = 16; i < 64; i++) {
      const s0 = this.rotr(W[i - 15], 7) ^ this.rotr(W[i - 15], 18) ^ (W[i - 15] >>> 3)
      const s1 = this.rotr(W[i - 2], 17) ^ this.rotr(W[i - 2], 19) ^ (W[i - 2] >>> 10)
      W[i] = (W[i - 16] + s0 + W[i - 7] + s1) | 0
    }

    let a = this.h[0], b = this.h[1], c = this.h[2], d = this.h[3],
        e = this.h[4], f = this.h[5], g = this.h[6], h = this.h[7]

    for (let i = 0; i < 64; i++) {
      const S1 = this.rotr(e, 6) ^ this.rotr(e, 11) ^ this.rotr(e, 25)
      const ch = (e & f) ^ (~e & g)
      const temp1 = (h + S1 + ch + this.K[i] + W[i]) | 0
      const S0 = this.rotr(a, 2) ^ this.rotr(a, 13) ^ this.rotr(a, 22)
      const maj = (a & b) ^ (a & c) ^ (b & c)
      const temp2 = (S0 + maj) | 0

      h = g
      g = f
      f = e
      e = (d + temp1) | 0
      d = c
      c = b
      b = a
      a = (temp1 + temp2) | 0
    }

    this.h[0] = (this.h[0] + a) | 0
    this.h[1] = (this.h[1] + b) | 0
    this.h[2] = (this.h[2] + c) | 0
    this.h[3] = (this.h[3] + d) | 0
    this.h[4] = (this.h[4] + e) | 0
    this.h[5] = (this.h[5] + f) | 0
    this.h[6] = (this.h[6] + g) | 0
    this.h[7] = (this.h[7] + h) | 0
  }

  finalize(): string {
    const totalBits = this.totalLen * 8
    this.update(new Uint8Array([0x80]))

    const padLen = this.bufferLen <= 56 ? 56 - this.bufferLen : 120 - this.bufferLen
    const pad = new Uint8Array(padLen)
    this.update(pad)

    const lenBytes = new Uint8Array(8)
    let temp = totalBits
    for (let i = 7; i >= 0; i--) {
      lenBytes[i] = temp & 0xff
      temp = Math.floor(temp / 256)
    }
    this.update(lenBytes)

    return Array.from(this.h)
      .map(val => val.toString(16).padStart(8, '0'))
      .join('')
  }
}

// ==========================================
// 4. 增量 SM3 (国密) 纯 JS 实现
// ==========================================
class Sm3Hasher {
  private h = new Uint32Array([
    0x7380166f, 0x4914b2b9, 0x172442d7, 0xda8a0600,
    0xa96f30bc, 0x163138aa, 0xe38dee4d, 0xb0fb0e4e
  ])
  private buffer = new Uint8Array(64)
  private bufferLen = 0
  private totalLen = 0

  private rol(n: number, c: number) {
    return (n << c) | (n >>> (32 - c))
  }

  private P0(x: number): number {
    return x ^ this.rol(x, 9) ^ this.rol(x, 17)
  }

  private P1(x: number): number {
    return x ^ this.rol(x, 15) ^ this.rol(x, 23)
  }

  update(data: Uint8Array) {
    this.totalLen += data.length
    let offset = 0
    while (offset < data.length) {
      const needed = 64 - this.bufferLen
      const copyLen = Math.min(needed, data.length - offset)
      this.buffer.set(data.subarray(offset, offset + copyLen), this.bufferLen)
      this.bufferLen += copyLen
      offset += copyLen

      if (this.bufferLen === 64) {
        this.compress(this.buffer)
        this.bufferLen = 0
      }
    }
  }

  private compress(block: Uint8Array) {
    const W = new Uint32Array(68)
    const WPrime = new Uint32Array(64)

    for (let i = 0; i < 16; i++) {
      W[i] = (block[i * 4] << 24) | (block[i * 4 + 1] << 16) | (block[i * 4 + 2] << 8) | block[i * 4 + 3]
    }
    for (let i = 16; i < 68; i++) {
      W[i] = this.P1(W[i - 16] ^ W[i - 9] ^ this.rol(W[i - 3], 15)) ^ this.rol(W[i - 13], 7) ^ W[i - 6]
    }
    for (let i = 0; i < 64; i++) {
      WPrime[i] = W[i] ^ W[i + 4]
    }

    let a = this.h[0], b = this.h[1], c = this.h[2], d = this.h[3],
        e = this.h[4], f = this.h[5], g = this.h[6], h = this.h[7]

    for (let j = 0; j < 64; j++) {
      const Tj = j < 16 ? 0x79cc4519 : 0x7a879d8a
      const ss1 = this.rol((this.rol(a, 12) + e + Tj) | 0, 7)
      const ss2 = ss1 ^ this.rol(a, 12)
      
      let ff = 0, gg = 0
      if (j < 16) {
        ff = a ^ b ^ c
        gg = e ^ f ^ g
      } else {
        ff = (a & b) | (a & c) | (b & c)
        gg = (e & f) | (~e & g)
      }

      const tt1 = (ff + d + ss2 + WPrime[j]) | 0
      const tt2 = (gg + h + ss1 + W[j]) | 0

      d = c
      c = this.rol(b, 9)
      b = a
      a = tt1
      h = g
      g = this.rol(f, 19)
      f = e
      e = this.P0(tt2)
    }

    this.h[0] ^= a
    this.h[1] ^= b
    this.h[2] ^= c
    this.h[3] ^= d
    this.h[4] ^= e
    this.h[5] ^= f
    this.h[6] ^= g
    this.h[7] ^= h
  }

  finalize(): string {
    const totalBits = this.totalLen * 8
    this.update(new Uint8Array([0x80]))

    const padLen = this.bufferLen <= 56 ? 56 - this.bufferLen : 120 - this.bufferLen
    const pad = new Uint8Array(padLen)
    this.update(pad)

    const lenBytes = new Uint8Array(8)
    let temp = totalBits
    for (let i = 7; i >= 0; i--) {
      lenBytes[i] = temp & 0xff
      temp = Math.floor(temp / 256)
    }
    this.update(lenBytes)

    return Array.from(this.h)
      .map(val => val.toString(16).padStart(8, '0'))
      .join('')
  }
}

// ==========================================
// 5. 统一计算管理器
// ==========================================
async function calculateTextHash() {
  errorMsg.value = ''
  if (!inputText.value) {
    outputHash.value = ''
    return
  }

  try {
    const enc = new TextEncoder().encode(inputText.value)

    if (mode.value === 'md5') {
      const h = new Md5Hasher()
      h.update(enc)
      outputHash.value = h.finalize()
    } else if (mode.value === 'sha1') {
      const h = new Sha1Hasher()
      h.update(enc)
      outputHash.value = h.finalize()
    } else if (mode.value === 'sha256') {
      const h = new Sha256Hasher()
      h.update(enc)
      outputHash.value = h.finalize()
    } else if (mode.value === 'sm3') {
      const h = new Sm3Hasher()
      h.update(enc)
      outputHash.value = h.finalize()
    } else if (mode.value === 'sha512') {
      // SHA-512 继续使用原生极快计算
      const buffer = await crypto.subtle.digest('SHA-512', enc)
      outputHash.value = Array.from(new Uint8Array(buffer))
        .map(b => b.toString(16).padStart(2, '0'))
        .join('')
    }
  } catch (err: any) {
    errorMsg.value = '哈希值计算失败: ' + err.message
  }
}

// 分块读取大文件并做增量 Hash
function startFileHash() {
  if (!selectedFile.value) return
  isCalculating.value = true
  calculateProgress.value = 0
  errorMsg.value = ''
  outputHash.value = ''
  cancelRequested.value = false

  const file = selectedFile.value
  const chunkSize = 2 * 1024 * 1024 // 2MB
  let offset = 0
  const reader = new FileReader()

  // 初始化 Hasher
  let hasher: any
  if (mode.value === 'md5') hasher = new Md5Hasher()
  else if (mode.value === 'sha1') hasher = new Sha1Hasher()
  else if (mode.value === 'sha256') hasher = new Sha256Hasher()
  else if (mode.value === 'sm3') hasher = new Sm3Hasher()
  else {
    errorMsg.value = '大文件仅支持 MD5, SHA-1, SHA-256, SM3 分块计算'
    isCalculating.value = false
    return
  }

  reader.onload = function (e: any) {
    if (cancelRequested.value) {
      isCalculating.value = false
      calculateProgress.value = 0
      return
    }

    const view = new Uint8Array(e.target.result)
    hasher.update(view)

    offset += chunkSize
    calculateProgress.value = Math.min(100, Math.round((offset / file.size) * 100))

    if (offset < file.size) {
      // 使用 setTimeout 释放主线程，防止 UI 卡死并确保进度条能渲染
      setTimeout(readNextChunk, 0)
    } else {
      outputHash.value = hasher.finalize()
      isCalculating.value = false
    }
  }

  reader.onerror = function () {
    errorMsg.value = '文件读取失败'
    isCalculating.value = false
  }

  function readNextChunk() {
    const slice = file.slice(offset, offset + chunkSize)
    reader.readAsArrayBuffer(slice)
  }

  readNextChunk()
}

// 拖拽上传
function handleDrop(e: DragEvent) {
  dragOver.value = false
  if (isCalculating.value) return
  if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
    selectedFile.value = e.dataTransfer.files[0]
    inputText.value = ''
    startFileHash()
  }
}

function handleFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    selectedFile.value = target.files[0]
    inputText.value = ''
    startFileHash()
  }
}

watch([inputText, mode, activeTab], () => {
  if (activeTab.value === 'text') {
    calculateTextHash()
  } else if (activeTab.value === 'file' && selectedFile.value && !isCalculating.value) {
    startFileHash()
  }
})

function copyResult() {
  if (!outputHash.value) return
  navigator.clipboard.writeText(outputHash.value)
}

function clearAll() {
  inputText.value = ''
  outputHash.value = ''
  errorMsg.value = ''
  selectedFile.value = null
  calculateProgress.value = 0
  isCalculating.value = false
}

// 格式化文件大小
function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}
</script>

<template>
  <ToolShell title="哈希/校验和计算器">
    <template #actions>
      <label style="font-size: 11px; color: var(--jc-text-secondary)">计算算法：</label>
      <JcSelect v-model="mode" :options="algoOptions" size="small" :disabled="isCalculating" style="width: 150px" />
      <JcButton type="primary" size="small" @click="copyResult" :disabled="!outputHash">复制哈希</JcButton>
      <JcButton size="small" danger ghost @click="clearAll" :disabled="isCalculating">清空</JcButton>
    </template>

    <JcSegmented
      :model-value="activeTab"
      :options="tabOptions"
      size="small"
      :disabled="isCalculating"
      @update:model-value="(v) => { if (!isCalculating) activeTab = v as 'text' | 'file' }"
    />

    <!-- 文本哈希主内容 -->
    <div v-show="activeTab === 'text'" class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label">输入文本内容</div>
        <JcTextarea v-model="inputText" mono :spellcheck="false" class="jc-fill" placeholder="在此输入需要计算哈希的字符串文本..." />
      </div>
      <div class="editor-pane">
        <div class="pane-label">哈希输出 (HEX 结果)</div>
        <JcTextarea :model-value="outputHash" mono readonly :spellcheck="false" class="jc-fill hash-output" placeholder="计算出的哈希值会在这里显示..." />
      </div>
    </div>

    <!-- 文件哈希主内容 -->
    <div v-show="activeTab === 'file'" class="file-hash-pane">
      <!-- 拖拽上传区 -->
      <div 
        class="drag-drop-area"
        :class="{ 'drag-over': dragOver, 'has-file': selectedFile }"
        @dragover.prevent="dragOver = true"
        @dragleave.prevent="dragOver = false"
        @drop.prevent="handleDrop"
        @click="fileInput?.click()"
      >
        <input 
          type="file" 
          ref="fileInput" 
          style="display: none;" 
          @change="handleFileSelect" 
        />
        
        <div class="drag-icon">
          <svg viewBox="0 0 24 24" width="36" height="36" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
            <polyline points="14 2 14 8 20 8"></polyline>
            <line x1="12" y1="18" x2="12" y2="12"></line>
            <polyline points="9 15 12 12 15 15"></polyline>
          </svg>
        </div>

        <div v-if="!selectedFile" class="drag-text">
          <span>拖拽大文件到此处，或点击浏览上传</span>
          <span class="drag-sub">支持 GB 级大文件，分块流式读取计算，不占浏览器内存</span>
        </div>
        <div v-else class="file-info-wrap">
          <div class="file-name">{{ selectedFile.name }}</div>
          <div class="file-size">{{ formatFileSize(selectedFile.size) }}</div>
        </div>
      </div>

      <!-- 计算进度与结果 -->
      <div class="file-acts" v-if="selectedFile">
        <div class="progress-wrap" v-if="isCalculating">
          <div class="progress-header">
            <span>哈希计算中...</span>
            <span>{{ calculateProgress }}%</span>
          </div>
          <div class="progress-bar-bg">
            <div class="progress-bar-fill" :style="{ width: calculateProgress + '%' }"></div>
          </div>
          <JcButton size="small" @click="cancelRequested = true">取消计算</JcButton>
        </div>

        <div class="result-wrap" v-if="outputHash && !isCalculating">
          <div class="pane-label">计算出的文件哈希值 ({{ mode.toUpperCase() }}):</div>
          <div class="hash-result-box code-font">
            <span>{{ outputHash }}</span>
            <JcButton size="small" @click="copyResult">复制</JcButton>
          </div>
        </div>
      </div>
    </div>

    <!-- 错误输出 -->
    <div v-if="errorMsg" class="tool-footer-error">{{ errorMsg }}</div>
  </ToolShell>
</template>

<style scoped lang="scss">
// 文本哈希
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 12px;
  min-height: 0;
}
.editor-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  padding: 8px;
  border-radius: 4px;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 6px;
  text-transform: uppercase;
}
.code-font {
  font-family: 'Cascadia Code', Consolas, monospace;
}
.hash-output {
  color: var(--jc-color-success);
}

// 文件哈希
.file-hash-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 12px 0;
  overflow-y: auto;
}
.drag-drop-area {
  border: 2px dashed var(--jc-border-strong);
  background: var(--jc-bg-panel);
  border-radius: 6px;
  padding: 30px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  cursor: pointer;
  text-align: center;
  transition: all 0.2s;
  &:hover, &.drag-over {
    border-color: var(--jc-color-accent);
    background: var(--jc-bg-hover);
  }
  &.has-file {
    border-style: solid;
    border-color: var(--jc-color-success);
  }
}
.drag-icon {
  color: var(--jc-text-secondary);
}
.drag-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--jc-text-primary);
  .drag-sub {
    font-size: 10px;
    color: var(--jc-text-secondary);
  }
}
.file-info-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  .file-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--jc-text-highlight);
  }
  .file-size {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}

// 进度与结果
.file-acts {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 16px;
  border-radius: 4px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.progress-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.progress-header {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--jc-text-secondary);
}
.progress-bar-bg {
  width: 100%;
  height: 8px;
  background: var(--jc-bg-input);
  border-radius: 4px;
  overflow: hidden;
}
.progress-bar-fill {
  height: 100%;
  background: var(--jc-color-accent);
  transition: width 0.1s ease-out;
}
.hash-result-box {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  padding: 10px 12px;
  border-radius: 4px;
  color: var(--jc-color-success);
  font-size: 12px;
  word-break: break-all;
  gap: 10px;
}

.tool-footer-error {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>
