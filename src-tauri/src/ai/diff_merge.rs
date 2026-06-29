use super::types::{MergeConflict, ConflictType};

/// 三向行级合并冲突检测引擎
pub struct DiffMergeEngine;

impl DiffMergeEngine {
    /// 执行三向合并。若 ours 和 theirs 修改了相同的内容则返回 Err(MergeConflict)
    pub fn merge_files(
        file_path: &str,
        base: &str,
        ours: &str,
        theirs: &str,
    ) -> Result<String, MergeConflict> {
        // 如果 ours 和 theirs 内容完全相同，直接采纳
        if ours == theirs {
            return Ok(ours.to_string());
        }
        // 如果 ours 等于 base，说明 AI (ours) 根本没有改动，采纳 theirs 的外部改动
        if base == ours {
            return Ok(theirs.to_string());
        }
        // 如果 theirs 等于 base，说明外部没有改动，采纳 ours 的修改
        if base == theirs {
            return Ok(ours.to_string());
        }

        let base_lines: Vec<&str> = base.lines().collect();
        let ours_lines: Vec<&str> = ours.lines().collect();
        let theirs_lines: Vec<&str> = theirs.lines().collect();

        // 行数对齐的直接行级冲突匹配
        if base_lines.len() == ours_lines.len() && base_lines.len() == theirs_lines.len() {
            let mut merged = Vec::new();
            for i in 0..base_lines.len() {
                let b = base_lines[i];
                let o = ours_lines[i];
                let t = theirs_lines[i];

                if o == t {
                    merged.push(o);
                } else if b == o {
                    merged.push(t);
                } else if b == t {
                    merged.push(o);
                } else {
                    // 同一行发生了不同的冲突改动
                    return Err(MergeConflict {
                        file_path: file_path.to_string(),
                        base_content: b.to_string(),
                        ours_content: o.to_string(),
                        theirs_content: t.to_string(),
                        conflict_type: ConflictType::ContentConflict,
                        resolution: None,
                    });
                }
            }
            return Ok(merged.join("\n"));
        }

        // 行数不对称时的结构性变动检测：
        // 在第一阶段和第二阶段，如果 ours 增加/删除了行，而 theirs 同样做出了修改，直接视作潜在的结构性冲突挂起
        Err(MergeConflict {
            file_path: file_path.to_string(),
            base_content: format!("原文件总行数: {}", base_lines.len()),
            ours_content: format!("隔离工作区修改后总行数: {}", ours_lines.len()),
            theirs_content: format!("主项目工作区当前总行数: {}", theirs_lines.len()),
            conflict_type: ConflictType::StructuralConflict,
            resolution: None,
        })
    }
}
