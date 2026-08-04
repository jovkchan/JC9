#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `jc9 mcp` → 以 stdio 方式运行内置 MCP Server（不启动 GUI），
    // 供外部 MCP 客户端（Claude Desktop / VS Code / Cursor 等）接入笔记与记忆
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("mcp") {
        jc9_lib::run_mcp_stdio();
        return;
    }
    jc9_lib::run()
}
