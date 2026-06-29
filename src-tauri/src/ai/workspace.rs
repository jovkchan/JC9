use std::path::{Path, PathBuf};
use std::fs;
use super::diff_merge::DiffMergeEngine;

/// COW 临时工作区隔离管理器 - 支持 base 与 ours 双区备份三向合并机制
pub struct WorkspaceManager {
    original_root: PathBuf,
    temp_root: PathBuf,
}

impl WorkspaceManager {
    pub fn new(original_root: PathBuf) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let temp_dir = std::env::temp_dir().join(format!("jc9_cov_{}", id));
        Self {
            original_root,
            temp_root: temp_dir,
        }
    }

    /// 创建隔离的 COW 工作空间，生成 ours（工作区）与 base（初始版本备份）双子目录
    pub fn prepare_sandbox(&self) -> Result<PathBuf, String> {
        let ours_dir = self.temp_root.join("ours");
        let base_dir = self.temp_root.join("base");

        fs::create_dir_all(&ours_dir).map_err(|e| e.to_string())?;
        fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;
        
        // 分别复制文件结构至 ours 和 base 目录
        copy_dir_all_selective(&self.original_root, &ours_dir)?;
        copy_dir_all_selective(&self.original_root, &base_dir)?;

        Ok(ours_dir)
    }

    /// 使用 DiffMergeEngine 对 ours、base 和 theirs (原始目录) 进行三向冲突合并后写回
    pub fn apply_to_original(&self) -> Result<(), String> {
        let ours_dir = self.temp_root.join("ours");
        let base_dir = self.temp_root.join("base");

        if !ours_dir.exists() || !base_dir.exists() {
            return Err("COW 隔离工作区双目录结构损坏，无法合并".into());
        }

        self.sync_and_merge(&ours_dir, &base_dir, &self.original_root)
    }

    /// 销毁临时区
    pub fn cleanup(&self) {
        if self.temp_root.exists() {
            let _ = fs::remove_dir_all(&self.temp_root);
        }
    }

    fn sync_and_merge(&self, ours_dir: &Path, base_dir: &Path, theirs_dir: &Path) -> Result<(), String> {
        if ours_dir.is_dir() {
            for entry in fs::read_dir(ours_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let filename = entry.file_name();
                let ours_path = entry.path();
                let base_path = base_dir.join(&filename);
                let theirs_path = theirs_dir.join(&filename);

                if ours_path.is_dir() {
                    self.sync_and_merge(&ours_path, &base_path, &theirs_path)?;
                } else {
                    // 如果该文件在 base 目录下也存在（也就是非新建文件）
                    if base_path.exists() {
                        let ours_content = fs::read_to_string(&ours_path).map_err(|e| e.to_string())?;
                        let base_content = fs::read_to_string(&base_path).map_err(|e| e.to_string())?;

                        // 如果 AI (ours) 修改了此文件
                        if ours_content != base_content {
                            if theirs_path.exists() {
                                let theirs_content = fs::read_to_string(&theirs_path).map_err(|e| e.to_string())?;
                                
                                // 进行三向 Diff 行级合并检测
                                let file_rel = theirs_path.strip_prefix(&self.original_root)
                                    .unwrap_or(&theirs_path)
                                    .to_string_lossy();
                                
                                match DiffMergeEngine::merge_files(&file_rel, &base_content, &ours_content, &theirs_content) {
                                    Ok(merged_content) => {
                                        fs::write(&theirs_path, merged_content).map_err(|e| e.to_string())?;
                                    }
                                    Err(conflict) => {
                                        return Err(format!(
                                            "【三向合并冲突拦截】文件 '{}' 发生冲突！AI 修改行: '{}'，外部修改行: '{}'，合并被迫中断。",
                                            conflict.file_path, conflict.ours_content, conflict.theirs_content
                                        ));
                                    }
                                }
                            } else {
                                // 原始目录下的文件被外部删除了，但 AI 修改了它。直接重新同步过去
                                if let Some(parent) = theirs_path.parent() {
                                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                                }
                                fs::copy(&ours_path, &theirs_path).map_err(|e| e.to_string())?;
                            }
                        }
                    } else {
                        // base 目录下没有，说明是 AI (ours) 新建的文件。直接拷贝回 original
                        if let Some(parent) = theirs_path.parent() {
                            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                        }
                        fs::copy(&ours_path, &theirs_path).map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn copy_dir_all_selective(src: &Path, dst: &Path) -> Result<(), String> {
    if src.is_dir() {
        if let Some(name) = src.file_name() {
            let name_str = name.to_string_lossy();
            if ["node_modules", ".git", "target", "dist", "build", ".svelte-kit"].contains(&name_str.as_ref()) {
                return Ok(());
            }
        }

        if !dst.exists() {
            fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        }

        for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());
            copy_dir_all_selective(&path, &dest_path)?;
        }
    } else {
        if let Some(ext) = src.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ["exe", "dll", "dylib", "so", "zip", "tar", "gz", "lock"].contains(&ext_str.as_str()) {
                return Ok(());
            }
        }
        let should_copy = if !dst.exists() {
            true
        } else {
            let src_meta = fs::metadata(src).map_err(|e| e.to_string())?;
            let dst_meta = fs::metadata(dst).map_err(|e| e.to_string())?;
            src_meta.len() != dst_meta.len()
        };
        if should_copy {
            fs::copy(src, dst).map_err(|e| format!("拷贝文件失败 {:?}: {}", src, e))?;
        }
    }
    Ok(())
}
