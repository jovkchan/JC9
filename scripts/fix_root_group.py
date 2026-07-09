"""将 API Key 的根分组改为 JCGO 分组"""
import sqlite3, os, json

db = os.path.expanduser('~/.jc9/data/jc9.db')
c = sqlite3.connect(db)

# 查看所有分组
print("所有分组:")
rows = c.execute("SELECT id, name FROM note_groups").fetchall()
for r in rows:
    print(f"  {r[0][:8]}... {r[1]}")

# 更新 root_group_id
row = c.execute("SELECT value FROM settings WHERE key='mcp_server_config'").fetchone()
if row:
    cfg = json.loads(row[0])
    old = cfg.get('rootGroupId') or cfg.get('root_group_id')
    # 改成 JCGO 分组
    cfg['rootGroupId'] = '21bb6be3-5078-49de-ae2c-ba40fb4a2574'
    c.execute("INSERT OR REPLACE INTO settings (key,value) VALUES ('mcp_server_config',?)",
              (json.dumps(cfg),))
    c.commit()
    print(f"\n✅ root_group_id: {old} → {cfg['rootGroupId']}")
else:
    print("\n⚠️ 无配置")

c.close()
