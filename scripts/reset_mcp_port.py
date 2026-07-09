"""重置 MCP Server 端口为 19799 并启用"""
import sqlite3, json, os

db_path = os.path.expanduser('~/.jc9/data/jc9.db')
conn = sqlite3.connect(db_path)
cur = conn.execute("SELECT value FROM settings WHERE key='mcp_server_config'")
row = cur.fetchone()

if row:
    cfg = json.loads(row[0])
    old_port = cfg.get('port', '?')
    cfg['port'] = 19799
    cfg['enabled'] = True
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
        ('mcp_server_config', json.dumps(cfg))
    )
    conn.commit()
    print(f'✅ 端口 {old_port} → 19799，已启用')
else:
    print('⚠️ 未找到 MCP Server 配置')
conn.close()
