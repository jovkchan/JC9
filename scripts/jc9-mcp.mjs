#!/usr/bin/env node
/**
 * JC9 MCP Stdio 代理入口
 * ---------------------------------------------------------------
 * 通过 stdin/stdout 实现 MCP stdio 协议,把请求转发到正在运行的
 * JC9 内置 MCP Server(HTTP 端点),复用其笔记/记忆/权限隔离能力。
 *
 * 外部 MCP 客户端配置(对齐 MCP 接入配置规范):
 *   "jc9": {
 *     "command": "node",
 *     "args": ["<本项目>/scripts/jc9-mcp.mjs"],
 *     "env": { "key": "<在 JC9 设置 → MCP → API Key 管理中生成>" }
 *   }
 *
 * 认证: 通过 env `key` 传入(兼容旧名 `JC9_MCP_KEY`)。
 * 前提: JC9 桌面应用需保持运行(数据在内置 MCP Server 上)。
 * 可选: env `JC9_URL` 覆盖内置 MCP Server 地址(默认 http://127.0.0.1:18899)。
 */
import { createInterface } from 'node:readline'
import { stdin, stdout } from 'node:process'

const BASE_URL = (process.env.JC9_URL || '__JC9_MCP_BASE_URL__').replace(/\/+$/, '')
const API_KEY = process.env.key || process.env.JC9_MCP_KEY
const SERVER_INFO = { name: 'jc9-mcp-proxy', version: '1.0.0' }

if (!API_KEY) {
  console.error(
    '[jc9-mcp] 错误: 未设置 key 环境变量。请先在 JC9 设置 → MCP → API Key 管理中生成一个 Key，' +
    '并在外部工具配置的 env 中填入 key。'
  )
  process.exit(1)
}

let msgSeq = 0

// 待处理请求计数 + stdin 关闭标记：确保所有转发完成后再退出进程
let pending = 0
let stdinClosed = false

function respond(msg) {
  stdout.write(JSON.stringify(msg) + '\n')
}

function maybeExit() {
  if (stdinClosed && pending === 0) process.exit(0)
}

/** 转发 JSON-RPC 请求到 JC9 内置 MCP Server,返回其 result */
async function forward(method, params = {}) {
  const id = ++msgSeq
  let res
  try {
    res = await fetch(`${BASE_URL}/message`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${API_KEY}`,
      },
      body: JSON.stringify({ jsonrpc: '2.0', id, method, params }),
    })
  } catch (e) {
    const cause = e.cause?.code || e.message || String(e)
    throw new Error(
      `无法连接 JC9 MCP Server (${BASE_URL})：${cause}。` +
      '请确认 JC9 桌面应用已启动（其内置 MCP Server 需保持运行）。'
    )
  }
  if (!res.ok) {
    const reason = res.status === 401 ? '（API Key 无效）' : ''
    throw new Error(`JC9 MCP Server 返回 HTTP ${res.status}${reason}`)
  }
  const data = await res.json()
  if (data.error) {
    throw new Error(`JC9 错误 ${data.error.code}: ${data.error.message}`)
  }
  return data.result
}

/** 处理单个 JSON-RPC 请求;通知(无 id)返回 undefined */
async function handle(req) {
  const method = req.method
  const params = req.params || {}
  switch (method) {
    case 'initialize':
      return {
        protocolVersion: '2024-11-05',
        capabilities: { tools: {} },
        serverInfo: SERVER_INFO,
      }
    case 'ping':
      return {}
    case 'tools/list':
      return await forward('tools/list', params)
    case 'tools/call':
      return await forward('tools/call', params)
    case 'notifications/initialized':
      return undefined
    default:
      throw Object.assign(new Error(`未知方法: ${method}`), { code: -32601 })
  }
}

const rl = createInterface({ input: stdin, crlfDelay: Infinity })

rl.on('line', (line) => {
  const text = line.trim()
  if (!text) return
  let req
  try {
    req = JSON.parse(text)
  } catch {
    respond({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'JSON 解析错误' } })
    return
  }
  pending++
  // 通知(无 id)只执行不响应
  if (req.id == null) {
    handle(req)
      .catch(() => {})
      .finally(() => {
        pending--
        maybeExit()
      })
    return
  }
  handle(req)
    .then((result) => {
      if (result === undefined) return
      respond({ jsonrpc: '2.0', id: req.id, result })
    })
    .catch((e) => {
      respond({
        jsonrpc: '2.0',
        id: req.id,
        error: { code: e.code || -32000, message: e.message },
      })
    })
    .finally(() => {
      pending--
      maybeExit()
    })
})

rl.on('close', () => {
  stdinClosed = true
  maybeExit()
})
