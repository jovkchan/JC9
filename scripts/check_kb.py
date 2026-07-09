"""检查 JC9 知识库同步和向量引擎状态"""
import sqlite3, os

db = os.path.expanduser('~/.jc9/data/jc9.db')
c = sqlite3.connect(db)

# 笔记数
cnt = c.execute("SELECT COUNT(*) FROM notes WHERE is_deleted=0").fetchone()[0]
print(f"notes 表: {cnt} 条")

# knowledge 中 note_ 条目
cnt2 = c.execute("SELECT COUNT(*) FROM knowledge WHERE id LIKE 'note_%'").fetchone()[0]
print(f"knowledge 表 note_ 条目: {cnt2} 条")

# embeddings
try:
    cnt3 = c.execute("SELECT COUNT(*) FROM embeddings").fetchone()[0]
    print(f"embeddings 表: {cnt3} 条")
except Exception as e:
    print(f"embeddings: {e}")

# sqlite-vec
try:
    cnt4 = c.execute("SELECT COUNT(*) FROM vec_embeddings").fetchone()[0]
    print(f"vec_embeddings (sqlite-vec): {cnt4} 条")
except Exception as e:
    print(f"vec_embeddings: 未加载 ({e})")

# 最新笔记
print("\n最新 5 条笔记:")
for row in c.execute("SELECT id,title,group_id,updated_at FROM notes WHERE is_deleted=0 ORDER BY updated_at DESC LIMIT 5"):
    print(f"  [{row[0][:8]}] {row[1][:40]}")
    print(f"       分组: {row[2] or '无'} | 更新: {row[3][:19]}")

# knowledge 中的条目预览
print("\nknowledge 中 note_ 条目预览:")
for row in c.execute("SELECT id,title,tags FROM knowledge WHERE id LIKE 'note_%' ORDER BY updated_at DESC LIMIT 5"):
    print(f"  [{row[0][:8]}] {row[1][:40]} tags={row[2][:30]}")

# API Key 配置
row = c.execute("SELECT value FROM settings WHERE key='mcp_server_config'").fetchone()
if row:
    import json
    cfg = json.loads(row[0])
    print(f"\nAPI Key 配置: enabled={cfg['enabled']}, port={cfg['port']}")
    print(f"  root_group_id: {cfg.get('rootGroupId') or cfg.get('root_group_id') or '未绑定'}")

c.close()
