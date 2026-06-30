/**
 * 前端小工具的纯 JS/TS 核心算法执行器
 * 供 AI Agent 双向 IPC 桥梁调用，使 AI 能免 UI 调用前端功能。
 */

// 1. Base64 编解码工具
export function executeBase64(mode: 'encode' | 'decode', input: string): string {
  if (!input) return '';
  if (mode === 'encode') {
    const bytes = new TextEncoder().encode(input);
    let binString = '';
    bytes.forEach(b => {
      binString += String.fromCharCode(b);
    });
    return btoa(binString);
  } else {
    const binString = atob(input.trim());
    const len = binString.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
      bytes[i] = binString.charCodeAt(i);
    }
    return new TextDecoder().decode(bytes);
  }
}

// 可以在这里扩展其他 32 个工具的核心纯计算函数，例如 JSON 格式化、URL 编码、Hex/Radix 转换等。
export function dispatchFrontendTool(name: string, args: any): string {
  switch (name) {
    case 'frontend_base64':
      return executeBase64(args.mode, args.input);
    default:
      throw new Error(`未实现的前端工具: ${name}`);
  }
}
