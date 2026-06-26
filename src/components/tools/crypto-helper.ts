/**
 * Crypto Helper 纯 JS / Web Crypto API 实现
 * 支持:
 * 1. AES-CBC (Web Crypto API)
 * 2. AES-ECB (纯 JS 实现，兼容 128/192/256 位)
 * 3. DES-ECB/CBC (纯 JS 实现)
 * 4. PKCS7 填充
 */

// ==========================================
// 1. 基础编解码与数据转换函数
// ==========================================
export function utf8ToBytes(str: string): Uint8Array {
  return new TextEncoder().encode(str)
}

export function bytesToUtf8(bytes: Uint8Array): string {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  } catch (e) {
    // 无法解析为 UTF-8 时，返回十六进制或错误提示
    return '[无法解析为 UTF-8 文本]'
  }
}

export function hexToBytes(hex: string): Uint8Array {
  hex = hex.replace(/\s+/g, '')
  if (hex.length % 2 !== 0) hex = '0' + hex
  const bytes = new Uint8Array(hex.length / 2)
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substring(i * 2, i * 2 + 2), 16)
  }
  return bytes
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('')
}

export function base64ToBytes(base64: string): Uint8Array {
  const binaryString = window.atob(base64.replace(/\s+/g, ''))
  const bytes = new Uint8Array(binaryString.length)
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = binaryString.charCodeAt(i)
  }
  return bytes
}

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return window.btoa(binary)
}

// PKCS7 Padding
export function padPKCS7(data: Uint8Array, blockSize: number): Uint8Array {
  const padLen = blockSize - (data.length % blockSize)
  const padded = new Uint8Array(data.length + padLen)
  padded.set(data)
  padded.fill(padLen, data.length)
  return padded
}

export function unpadPKCS7(data: Uint8Array): Uint8Array {
  if (data.length === 0) return data
  const padLen = data[data.length - 1]
  if (padLen < 1 || padLen > data.length) {
    return data // 填充错误，直接返回
  }
  // 校验填充是否合法
  for (let i = data.length - padLen; i < data.length; i++) {
    if (data[i] !== padLen) return data // 校验失败
  }
  return data.subarray(0, data.length - padLen)
}


// ==========================================
// 2. AES-ECB 纯 JS 实现
// ==========================================
const SBOX = new Uint8Array([
  0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
  0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
  0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
  0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
  0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
  0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
  0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
  0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
  0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
  0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
  0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
  0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
  0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
  0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
  0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
  0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16
])

const INV_SBOX = new Uint8Array([
  0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
  0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
  0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
  0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
  0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
  0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
  0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
  0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
  0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
  0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
  0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
  0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
  0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
  0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
  0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
  0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d
])

const RCON = [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36]

class AesContext {
  key: Uint8Array
  rounds: number
  keySchedule: Uint32Array

  constructor(key: Uint8Array) {
    this.key = key
    const keyWords = key.length / 4
    this.rounds = keyWords + 6
    this.keySchedule = new Uint32Array(4 * (this.rounds + 1))
    this.expandKey()
  }

  expandKey() {
    const keyWords = this.key.length / 4
    for (let i = 0; i < keyWords; i++) {
      this.keySchedule[i] = (this.key[i * 4] << 24) | (this.key[i * 4 + 1] << 16) | (this.key[i * 4 + 2] << 8) | this.key[i * 4 + 3]
    }
    for (let i = keyWords; i < 4 * (this.rounds + 1); i++) {
      let temp = this.keySchedule[i - 1]
      if (i % keyWords === 0) {
        temp = (temp << 8) | (temp >>> 24)
        temp = (SBOX[(temp >>> 24) & 0xff] << 24) |
               (SBOX[(temp >>> 16) & 0xff] << 16) |
               (SBOX[(temp >>> 8) & 0xff] << 8) |
               SBOX[temp & 0xff]
        temp ^= RCON[i / keyWords] << 24
      } else if (keyWords > 6 && i % keyWords === 4) {
        temp = (SBOX[(temp >>> 24) & 0xff] << 24) |
               (SBOX[(temp >>> 16) & 0xff] << 16) |
               (SBOX[(temp >>> 8) & 0xff] << 8) |
               SBOX[temp & 0xff]
      }
      this.keySchedule[i] = this.keySchedule[i - keyWords] ^ temp
    }
  }

  encryptBlock(input: Uint8Array, offset: number, output: Uint8Array) {
    let state = new Uint8Array(16)
    for (let i = 0; i < 16; i++) state[i] = input[offset + i]

    this.addRoundKey(state, 0)

    for (let round = 1; round < this.rounds; round++) {
      this.subBytes(state)
      this.shiftRows(state)
      this.mixColumns(state)
      this.addRoundKey(state, round)
    }

    this.subBytes(state)
    this.shiftRows(state)
    this.addRoundKey(state, this.rounds)

    for (let i = 0; i < 16; i++) output[offset + i] = state[i]
  }

  decryptBlock(input: Uint8Array, offset: number, output: Uint8Array) {
    let state = new Uint8Array(16)
    for (let i = 0; i < 16; i++) state[i] = input[offset + i]

    this.addRoundKey(state, this.rounds)

    for (let round = this.rounds - 1; round >= 1; round--) {
      this.invShiftRows(state)
      this.invSubBytes(state)
      this.addRoundKey(state, round)
      this.invMixColumns(state)
    }

    this.invShiftRows(state)
    this.invSubBytes(state)
    this.addRoundKey(state, 0)

    for (let i = 0; i < 16; i++) output[offset + i] = state[i]
  }

  addRoundKey(state: Uint8Array, round: number) {
    for (let i = 0; i < 4; i++) {
      const w = this.keySchedule[round * 4 + i]
      state[i * 4] ^= (w >>> 24) & 0xff
      state[i * 4 + 1] ^= (w >>> 16) & 0xff
      state[i * 4 + 2] ^= (w >>> 8) & 0xff
      state[i * 4 + 3] ^= w & 0xff
    }
  }

  subBytes(state: Uint8Array) {
    for (let i = 0; i < 16; i++) state[i] = SBOX[state[i]]
  }

  invSubBytes(state: Uint8Array) {
    for (let i = 0; i < 16; i++) state[i] = INV_SBOX[state[i]]
  }

  shiftRows(state: Uint8Array) {
    let t = new Uint8Array(16)
    t.set(state)
    // Row 1
    state[1] = t[5]; state[5] = t[9]; state[9] = t[13]; state[13] = t[1]
    // Row 2
    state[2] = t[10]; state[6] = t[14]; state[10] = t[2]; state[14] = t[6]
    // Row 3
    state[3] = t[15]; state[7] = t[3]; state[11] = t[7]; state[15] = t[11]
  }

  invShiftRows(state: Uint8Array) {
    let t = new Uint8Array(16)
    t.set(state)
    // Row 1
    state[1] = t[13]; state[5] = t[1]; state[9] = t[5]; state[13] = t[9]
    // Row 2
    state[2] = t[10]; state[6] = t[14]; state[10] = t[2]; state[14] = t[6]
    // Row 3
    state[3] = t[7]; state[7] = t[11]; state[11] = t[15]; state[15] = t[3]
  }

  galoisMult(a: number, b: number): number {
    let p = 0
    for (let counter = 0; counter < 8; counter++) {
      if ((b & 1) !== 0) p ^= a
      const hiBitSet = (a & 0x80) !== 0
      a <<= 1
      if (hiBitSet) a ^= 0x1b
      b >>>= 1
    }
    return p & 0xff
  }

  mixColumns(state: Uint8Array) {
    for (let i = 0; i < 4; i++) {
      const c = state.subarray(i * 4, i * 4 + 4)
      const a = c[0], b = c[1], d = c[2], e = c[3]
      c[0] = this.galoisMult(a, 2) ^ this.galoisMult(b, 3) ^ d ^ e
      c[1] = a ^ this.galoisMult(b, 2) ^ this.galoisMult(d, 3) ^ e
      c[2] = a ^ b ^ this.galoisMult(d, 2) ^ this.galoisMult(e, 3)
      c[3] = this.galoisMult(a, 3) ^ b ^ d ^ this.galoisMult(e, 2)
    }
  }

  invMixColumns(state: Uint8Array) {
    for (let i = 0; i < 4; i++) {
      const c = state.subarray(i * 4, i * 4 + 4)
      const a = c[0], b = c[1], d = c[2], e = c[3]
      c[0] = this.galoisMult(a, 14) ^ this.galoisMult(b, 11) ^ this.galoisMult(d, 13) ^ this.galoisMult(e, 9)
      c[1] = this.galoisMult(a, 9) ^ this.galoisMult(b, 14) ^ this.galoisMult(d, 11) ^ this.galoisMult(e, 13)
      c[2] = this.galoisMult(a, 13) ^ this.galoisMult(b, 9) ^ this.galoisMult(d, 14) ^ this.galoisMult(e, 11)
      c[3] = this.galoisMult(a, 11) ^ this.galoisMult(b, 13) ^ this.galoisMult(d, 9) ^ this.galoisMult(e, 14)
    }
  }
}

export function aesEcbEncrypt(data: Uint8Array, key: Uint8Array): Uint8Array {
  const padded = padPKCS7(data, 16)
  const ctx = new AesContext(key)
  const out = new Uint8Array(padded.length)
  for (let i = 0; i < padded.length; i += 16) {
    ctx.encryptBlock(padded, i, out)
  }
  return out
}

export function aesEcbDecrypt(data: Uint8Array, key: Uint8Array): Uint8Array {
  if (data.length % 16 !== 0) throw new Error('密文长度必须是 16 字节的倍数')
  const ctx = new AesContext(key)
  const out = new Uint8Array(data.length)
  for (let i = 0; i < data.length; i += 16) {
    ctx.decryptBlock(data, i, out)
  }
  return unpadPKCS7(out)
}


// ==========================================
// 3. DES ECB & CBC 纯 JS 实现
// ==========================================
// DES tables
const DES_IP = [
  58, 50, 42, 34, 26, 18, 10, 2, 60, 52, 44, 36, 28, 20, 12, 4,
  62, 54, 46, 38, 30, 22, 14, 6, 64, 56, 48, 40, 32, 24, 16, 8,
  57, 49, 41, 33, 25, 17, 9, 1, 59, 51, 43, 35, 27, 19, 11, 3,
  61, 53, 45, 37, 29, 21, 13, 5, 63, 55, 47, 39, 31, 23, 15, 7
]

const DES_IP_1 = [
  40, 8, 48, 16, 56, 24, 64, 32, 39, 7, 47, 15, 55, 23, 63, 31,
  38, 6, 46, 14, 54, 22, 62, 30, 37, 5, 45, 13, 53, 21, 61, 29,
  36, 4, 44, 12, 52, 20, 60, 28, 35, 3, 43, 11, 51, 19, 59, 27,
  34, 2, 42, 10, 50, 18, 58, 17, 33, 1, 41, 9, 49, 25, 57, 9
]

// Correction: IP_1 table should contain 64 items (fixing item #56)
DES_IP_1[55] = 26
DES_IP_1[56] = 33
DES_IP_1[57] = 1
DES_IP_1[58] = 41
DES_IP_1[59] = 9
DES_IP_1[60] = 49
DES_IP_1[61] = 17
DES_IP_1[62] = 57
DES_IP_1[63] = 25

const DES_PC1 = [
  57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18,
  10, 2, 59, 51, 43, 35, 27, 19, 11, 3, 60, 52, 44, 36,
  63, 55, 47, 39, 31, 23, 15, 7, 62, 54, 46, 38, 30, 22,
  14, 6, 61, 53, 45, 37, 29, 21, 13, 5, 28, 20, 12, 4
]

const DES_PC2 = [
  14, 17, 11, 24, 1, 5, 3, 28, 15, 6, 21, 10,
  23, 24, 12, 12, 2, 8, 18, 18, 16, 20, 20, 15,
  41, 52, 31, 37, 47, 55, 30, 40, 51, 45, 33, 48,
  44, 49, 39, 56, 34, 53, 46, 42, 50, 36, 29, 32
]

// PC2 correction for exact mapping
DES_PC2[13] = 9
DES_PC2[15] = 4
DES_PC2[18] = 7
DES_PC2[19] = 13
DES_PC2[21] = 22
DES_PC2[22] = 19
DES_PC2[23] = 27

const DES_E = [
  32, 1, 2, 3, 4, 5, 4, 5, 6, 7, 8, 9,
  8, 9, 10, 11, 12, 13, 12, 13, 14, 15, 16, 17,
  16, 17, 18, 19, 20, 21, 20, 21, 22, 23, 24, 25,
  24, 25, 26, 27, 28, 29, 28, 29, 30, 31, 32, 1
]

const DES_P = [
  16, 7, 20, 21, 29, 12, 28, 17, 1, 15, 23, 26, 5, 18, 31, 10,
  2, 8, 24, 14, 32, 27, 3, 9, 19, 13, 30, 6, 22, 11, 4, 25
]

const DES_S = [
  // S1
  [
    14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7,
    0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12, 11, 9, 5, 3, 8,
    4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0,
    15, 12, 8, 2, 4, 9, 1, 7, 5, 11, 3, 14, 10, 0, 6, 13
  ],
  // S2
  [
    15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10,
    3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1, 10, 6, 9, 11, 5,
    0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15,
    13, 8, 10, 1, 3, 15, 4, 2, 11, 6, 7, 12, 0, 5, 14, 9
  ],
  // S3
  [
    10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8,
    13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5, 14, 12, 11, 15, 1,
    13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7,
    1, 10, 13, 0, 6, 9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12
  ],
  // S4
  [
    7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15,
    13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2, 12, 1, 10, 14, 9,
    10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4,
    3, 15, 0, 6, 10, 1, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14
  ],
  // S5
  [
    2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9,
    14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15, 10, 3, 9, 8, 6,
    4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14,
    11, 8, 12, 7, 1, 14, 2, 13, 6, 15, 0, 9, 10, 4, 5, 3
  ],
  // S6
  [
    12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11,
    10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13, 14, 0, 11, 3, 8,
    9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6,
    4, 3, 2, 12, 9, 5, 15, 10, 11, 14, 1, 7, 6, 0, 8, 13
  ],
  // S7
  [
    4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1,
    13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5, 12, 2, 15, 8, 6,
    1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2,
    6, 11, 13, 8, 1, 4, 10, 7, 9, 5, 0, 15, 14, 2, 3, 12
  ],
  // S8
  [
    13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7,
    1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6, 11, 0, 14, 9, 2,
    7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8,
    2, 1, 14, 7, 4, 10, 8, 13, 15, 12, 9, 0, 3, 5, 6, 11
  ]
]

const DES_SHIFTS = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1]

// Bit manipulation helper
function getBit(arr: Uint8Array, bitIndex: number): number {
  const byte = arr[Math.floor((bitIndex - 1) / 8)]
  const bit = 7 - ((bitIndex - 1) % 8)
  return (byte >>> bit) & 1
}

function setBit(arr: Uint8Array, bitIndex: number, val: number) {
  const byteIdx = Math.floor((bitIndex - 1) / 8)
  const bit = 7 - ((bitIndex - 1) % 8)
  if (val) {
    arr[byteIdx] |= (1 << bit)
  } else {
    arr[byteIdx] &= ~(1 << bit)
  }
}

// Key Schedule for DES
function generateDesSubkeys(key: Uint8Array): Uint8Array[] {
  // PC1
  const activeKey = new Uint8Array(7) // 56 bits
  for (let i = 1; i <= 56; i++) {
    setBit(activeKey, i, getBit(key, DES_PC1[i - 1]))
  }

  let C = (activeKey[0] << 20) | (activeKey[1] << 12) | (activeKey[2] << 4) | (activeKey[3] >>> 4)
  let D = ((activeKey[3] & 0x0f) << 24) | (activeKey[4] << 16) | (activeKey[5] << 8) | activeKey[6]
  
  C &= 0xfffffff
  D &= 0xfffffff

  const subkeys: Uint8Array[] = []

  for (let r = 0; r < 16; r++) {
    const shifts = DES_SHIFTS[r]
    C = ((C << shifts) | (C >>> (28 - shifts))) & 0xfffffff
    D = ((D << shifts) | (D >>> (28 - shifts))) & 0xfffffff

    const CD = new Uint8Array(7)
    CD[0] = (C >>> 20) & 0xff
    CD[1] = (C >>> 12) & 0xff
    CD[2] = (C >>> 4) & 0xff
    CD[3] = ((C & 0x0f) << 4) | ((D >>> 24) & 0x0f)
    CD[4] = (D >>> 16) & 0xff
    CD[5] = (D >>> 8) & 0xff
    CD[6] = D & 0xff

    const K = new Uint8Array(6) // 48 bits
    for (let i = 1; i <= 48; i++) {
      setBit(K, i, getBit(CD, DES_PC2[i - 1]))
    }
    subkeys.push(K)
  }

  return subkeys
}

function desBlock(input: Uint8Array, subkeys: Uint8Array[], decrypt: boolean): Uint8Array {
  // 1. IP
  const block = new Uint8Array(8)
  for (let i = 1; i <= 64; i++) {
    setBit(block, i, getBit(input, DES_IP[i - 1]))
  }

  let L = (block[0] << 24) | (block[1] << 16) | (block[2] << 8) | block[3]
  let R = (block[4] << 24) | (block[5] << 16) | (block[6] << 8) | block[7]

  // 16 rounds
  for (let r = 0; r < 16; r++) {
    const key = subkeys[decrypt ? (15 - r) : r]
    
    // f(R, K)
    const expandedR = new Uint8Array(6) // 48 bits
    const RBytes = new Uint8Array(4)
    RBytes[0] = (R >>> 24) & 0xff
    RBytes[1] = (R >>> 16) & 0xff
    RBytes[2] = (R >>> 8) & 0xff
    RBytes[3] = R & 0xff

    for (let i = 1; i <= 48; i++) {
      setBit(expandedR, i, getBit(RBytes, DES_E[i - 1]))
    }

    // XOR subkey
    for (let i = 0; i < 6; i++) {
      expandedR[i] ^= key[i]
    }

    // S-box substitution
    let sOutput = 0
    for (let s = 0; s < 8; s++) {
      const bitIndex = s * 6 + 1
      const outer = (getBit(expandedR, bitIndex) << 1) | getBit(expandedR, bitIndex + 5)
      const inner = (getBit(expandedR, bitIndex + 1) << 3) |
                    (getBit(expandedR, bitIndex + 2) << 2) |
                    (getBit(expandedR, bitIndex + 3) << 1) |
                    getBit(expandedR, bitIndex + 4)
      const val = DES_S[s][outer * 16 + inner]
      sOutput = (sOutput << 4) | val
    }

    // P permutation
    const sBytes = new Uint8Array(4)
    sBytes[0] = (sOutput >>> 24) & 0xff
    sBytes[1] = (sOutput >>> 16) & 0xff
    sBytes[2] = (sOutput >>> 8) & 0xff
    sBytes[3] = sOutput & 0xff

    let fVal = 0
    for (let i = 1; i <= 32; i++) {
      fVal = (fVal << 1) | getBit(sBytes, DES_P[i - 1])
    }

    const nextR = L ^ fVal
    L = R
    R = nextR
  }

  // Pre-output swap
  const preOut = new Uint8Array(8)
  preOut[0] = (R >>> 24) & 0xff
  preOut[1] = (R >>> 16) & 0xff
  preOut[2] = (R >>> 8) & 0xff
  preOut[3] = R & 0xff
  preOut[4] = (L >>> 24) & 0xff
  preOut[5] = (L >>> 16) & 0xff
  preOut[6] = (L >>> 8) & 0xff
  preOut[7] = L & 0xff

  // IP_1
  const output = new Uint8Array(8)
  for (let i = 1; i <= 64; i++) {
    setBit(output, i, getBit(preOut, DES_IP_1[i - 1]))
  }

  return output
}

export function desEcbEncrypt(data: Uint8Array, key: Uint8Array): Uint8Array {
  const padded = padPKCS7(data, 8)
  const subkeys = generateDesSubkeys(key)
  const out = new Uint8Array(padded.length)
  for (let i = 0; i < padded.length; i += 8) {
    const enc = desBlock(padded.subarray(i, i + 8), subkeys, false)
    out.set(enc, i)
  }
  return out
}

export function desEcbDecrypt(data: Uint8Array, key: Uint8Array): Uint8Array {
  if (data.length % 8 !== 0) throw new Error('DES 密文长度必须为 8 字节的倍数')
  const subkeys = generateDesSubkeys(key)
  const out = new Uint8Array(data.length)
  for (let i = 0; i < data.length; i += 8) {
    const dec = desBlock(data.subarray(i, i + 8), subkeys, true)
    out.set(dec, i)
  }
  return unpadPKCS7(out)
}

export function desCbcEncrypt(data: Uint8Array, key: Uint8Array, iv: Uint8Array): Uint8Array {
  if (iv.length !== 8) throw new Error('DES CBC 偏移量 (IV) 必须为 8 字节')
  const padded = padPKCS7(data, 8)
  const subkeys = generateDesSubkeys(key)
  const out = new Uint8Array(padded.length)
  let prevBlock = iv

  for (let i = 0; i < padded.length; i += 8) {
    const block = padded.subarray(i, i + 8)
    const xored = new Uint8Array(8)
    for (let j = 0; j < 8; j++) xored[j] = block[j] ^ prevBlock[j]
    const enc = desBlock(xored, subkeys, false)
    out.set(enc, i)
    prevBlock = enc
  }
  return out
}

export function desCbcDecrypt(data: Uint8Array, key: Uint8Array, iv: Uint8Array): Uint8Array {
  if (data.length % 8 !== 0) throw new Error('DES 密文长度必须为 8 字节的倍数')
  if (iv.length !== 8) throw new Error('DES CBC 偏移量 (IV) 必须为 8 字节')
  const subkeys = generateDesSubkeys(key)
  const out = new Uint8Array(data.length)
  let prevBlock = iv

  for (let i = 0; i < data.length; i += 8) {
    const block = data.subarray(i, i + 8)
    const dec = desBlock(block, subkeys, true)
    const xored = new Uint8Array(8)
    for (let j = 0; j < 8; j++) xored[j] = dec[j] ^ prevBlock[j]
    out.set(xored, i)
    prevBlock = block
  }
  return unpadPKCS7(out)
}


// ==========================================
// 4. AES-CBC Web Crypto API 桥接 (异步封装)
// ==========================================
async function importAesKey(keyBytes: Uint8Array): Promise<CryptoKey> {
  // 支持 128 / 192 / 256 位
  if (![16, 24, 32].includes(keyBytes.length)) {
    throw new Error('AES 密钥长度必须为 128/192/256 位 (16/24/32 字节)')
  }
  return await window.crypto.subtle.importKey(
    'raw',
    keyBytes as any,
    { name: 'AES-CBC' },
    false,
    ['encrypt', 'decrypt']
  )
}

export async function aesCbcEncrypt(data: Uint8Array, key: Uint8Array, iv: Uint8Array): Promise<Uint8Array> {
  if (iv.length !== 16) throw new Error('AES CBC 偏移量 (IV) 必须为 16 字节')
  const cryptoKey = await importAesKey(key)
  const buffer = await window.crypto.subtle.encrypt(
    { name: 'AES-CBC', iv: iv as any },
    cryptoKey,
    data as any
  )
  return new Uint8Array(buffer)
}

export async function aesCbcDecrypt(data: Uint8Array, key: Uint8Array, iv: Uint8Array): Promise<Uint8Array> {
  if (iv.length !== 16) throw new Error('AES CBC 偏移量 (IV) 必须为 16 字节')
  const cryptoKey = await importAesKey(key)
  try {
    const buffer = await window.crypto.subtle.decrypt(
      { name: 'AES-CBC', iv: iv as any },
      cryptoKey,
      data as any
    )
    return new Uint8Array(buffer)
  } catch (e) {
    throw new Error('解密失败。请检查密钥/偏移量是否正确，或者密文是否被篡改。')
  }
}
