1. 结构先行 (Structure First)
必须使用 # 标题分层：确保文档有清晰的层级（H1, H2, H3...）。这是向量模型进行语义分块（Chunking） 的首要依据。

保持章节语义独立：每个 ## 二级标题下的内容，应围绕一个独立的子主题展开，避免跨标题的强逻辑依赖。确保检索命中该块时，信息能自解释。

2. 元数据驱动 (Metadata-Driven)
使用 YAML Frontmatter：在文档开头 --- 代码块中声明元数据。这是利用 sqlite-vec 的 元数据预过滤（Pre-filtering） 功能、提升查询速度的关键。

必填字段建议：

id：文档唯一标识。

type：文档大类（如 FAQ、Guide、Manual）。

tags：标签列表（如 ["SQL", "Performance"]）。

3. 内容表达清晰 (Clarity in Content)
段落首句总结：每个段落的第一句话尽量概括该段主旨，这有助于向量模型捕捉长文本中的核心语义。

善用列表和代码块：用 - 或 1. 列出并列信息，用 ` 包裹代码。结构化的内容在向量化后，与查询的语义相似度（Cosine Similarity） 计算更精准。

4. 存储与检索优化 (Storage & Retrieval)
明确向量维度：在涉及 SQL 示例时，明确标注向量维度（如 embedding float[768]），方便后续表结构设计。

记录块标题（Chunk Title）：在文档中，为每个 ## 部分打上标签，便于存入元数据列。查询时可根据块标题进行针对性检索。

5. 混合搜索意识 (Hybrid Search Awareness)
保留关键词密度：不要为了“语义”而完全抛弃“关键词”。在关键概念旁保留高频术语（如 sqlite-vec、向量检索），以便与 SQLite FTS5（全文搜索） 进行混合检索（Hybrid Search），实现互补。