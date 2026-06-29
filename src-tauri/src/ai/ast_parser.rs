use std::path::Path;
use serde::{Deserialize, Serialize};

/// AST 解析器 - 基于 Tree-sitter 的精确符号提取
/// 支持 TypeScript, Rust 语法树解析，Vue SFC 通过提取 script 块解析

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub column: usize,
    pub parent: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    TypeAlias,
    Module,
    Trait,
    Implementation,
    Variable,
    Constant,
    Export,
    Unknown,
}

pub struct AstParser {
    ts_parser: tree_sitter::Parser,
    rust_parser: tree_sitter::Parser,
}

impl AstParser {
    pub fn new() -> Self {
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&tree_sitter_typescript::language_typescript())
            .expect("无法加载 TypeScript Tree-sitter 语法");

        let mut rust_parser = tree_sitter::Parser::new();
        rust_parser.set_language(&tree_sitter_rust::language())
            .expect("无法加载 Rust Tree-sitter 语法");

        Self { ts_parser, rust_parser }
    }

    /// 解析文件并提取符号大纲
    pub fn parse_file(&mut self, path: &Path) -> Result<Vec<SymbolInfo>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取文件失败: {}", e))?;

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "ts" | "js" | "tsx" | "jsx" => self.parse_typescript(&content),
            "vue" => self.parse_vue_sfc(&content),
            "rs" => self.parse_rust(&content),
            _ => {
                // 对于不支持的文件类型，回退到正则提取
                Ok(self.fallback_regex_parse(&content))
            }
        }
    }

    /// 解析 TypeScript/JavaScript 文件
    fn parse_typescript(&mut self, content: &str) -> Result<Vec<SymbolInfo>, String> {
        let tree = self.ts_parser.parse(content, None)
            .ok_or("Tree-sitter 解析失败")?;

        let root = tree.root_node();
        let mut symbols = Vec::new();
        self.collect_ts_symbols(root, content, None, &mut symbols);
        Ok(symbols)
    }

    /// 解析 Vue SFC 文件（提取 script 块）
    fn parse_vue_sfc(&mut self, content: &str) -> Result<Vec<SymbolInfo>, String> {
        // 简单提取 <script> ... </script> 块
        let script_start = content.find("<script")
            .and_then(|i| content[i..].find('>').map(|j| i + j + 1));
        let script_end = content.find("</script>");

        if let (Some(start), Some(end)) = (script_start, script_end) {
            let script_content = &content[start..end];
            return self.parse_typescript(script_content);
        }

        Ok(vec![])
    }

    /// 解析 Rust 文件
    fn parse_rust(&mut self, content: &str) -> Result<Vec<SymbolInfo>, String> {
        let tree = self.rust_parser.parse(content, None)
            .ok_or("Tree-sitter 解析失败")?;

        let root = tree.root_node();
        let mut symbols = Vec::new();
        self.collect_rust_symbols(root, content, None, &mut symbols);
        Ok(symbols)
    }

    /// 递归遍历 TypeScript AST 收集符号
    fn collect_ts_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        parent: Option<String>,
        symbols: &mut Vec<SymbolInfo>,
    ) {
        let kind = node.kind();
        let (symbol_kind, name) = match kind {
            "function_declaration" | "method_definition" => {
                let name = self.node_name(node, source);
                (SymbolKind::Function, name)
            }
            "class_declaration" => {
                let name = self.node_name(node, source);
                (SymbolKind::Class, name)
            }
            "interface_declaration" => {
                let name = self.node_name(node, source);
                (SymbolKind::Interface, name)
            }
            "type_alias_declaration" => {
                let name = self.node_name(node, source);
                (SymbolKind::TypeAlias, name)
            }
            "enum_declaration" => {
                let name = self.node_name(node, source);
                (SymbolKind::Enum, name)
            }
            "export_statement" => {
                let name = self.node_name(node, source);
                (SymbolKind::Export, name)
            }
            "variable_declarator" => {
                let name = self.node_name(node, source);
                if let Some(parent_node) = node.parent() {
                    if parent_node.kind() == "lexical_declaration"
                        && parent_node.parent().map_or(false, |p| p.kind() == "program" || p.kind() == "module")
                    {
                        (SymbolKind::Constant, name)
                    } else {
                        return; // 跳过局部变量
                    }
                } else {
                    return;
                }
            }
            _ => return, // 跳过不感兴趣的类型
        };

        let start = node.start_position();
        symbols.push(SymbolInfo {
            name,
            kind: symbol_kind,
            line: start.row + 1,
            column: start.column + 1,
            parent: parent.clone(),
            visibility: None,
        });

        // 递归子节点
        let current_name = symbols.last().map(|s| s.name.clone());
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_ts_symbols(child, source, current_name.clone(), symbols);
            }
        }
    }

    /// 递归遍历 Rust AST 收集符号
    fn collect_rust_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        parent: Option<String>,
        symbols: &mut Vec<SymbolInfo>,
    ) {
        let kind = node.kind();
        let (symbol_kind, name, visibility) = match kind {
            "function_item" => {
                let name = self.node_name(node, source);
                let vis = self.rust_visibility(node, source);
                (SymbolKind::Function, name, vis)
            }
            "struct_item" => {
                let name = self.node_name(node, source);
                let vis = self.rust_visibility(node, source);
                (SymbolKind::Struct, name, vis)
            }
            "enum_item" => {
                let name = self.node_name(node, source);
                let vis = self.rust_visibility(node, source);
                (SymbolKind::Enum, name, vis)
            }
            "trait_item" => {
                let name = self.node_name(node, source);
                let vis = self.rust_visibility(node, source);
                (SymbolKind::Trait, name, vis)
            }
            "impl_item" => {
                let name = self.node_name(node, source);
                let vis = self.rust_visibility(node, source);
                (SymbolKind::Implementation, name, vis)
            }
            "mod_item" => {
                let name = self.node_name(node, source);
                (SymbolKind::Module, name, None)
            }
            _ => return,
        };

        let start = node.start_position();
        symbols.push(SymbolInfo {
            name,
            kind: symbol_kind,
            line: start.row + 1,
            column: start.column + 1,
            parent: parent.clone(),
            visibility,
        });

        let current_name = symbols.last().map(|s| s.name.clone());
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_rust_symbols(child, source, current_name.clone(), symbols);
            }
        }
    }

    /// 从 AST 节点中提取名称
    fn node_name(&self, node: tree_sitter::Node, source: &str) -> String {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "name" {
                return child.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
            }
        }
        // 对于 impl 块，从 type_identifier 获取
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                return child.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
            }
        }
        "unknown".to_string()
    }

    /// 检查 Rust 符号的可见性
    fn rust_visibility(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                return Some(child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
            }
        }
        None
    }

    /// 正则回退解析（用于不支持的文件类型）
    fn fallback_regex_parse(&self, content: &str) -> Vec<SymbolInfo> {
        let re_rust = regex::Regex::new(r"(?m)^\s*(pub\s+)?(fn|struct|enum|trait|impl|mod)\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_js_ts = regex::Regex::new(r"(?m)^\s*(export\s+)?(async\s+)?(function|class|interface|type|const)\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        let mut symbols = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            if let Some(caps) = re_rust.captures(line) {
                let kind = match caps.get(2).map(|m| m.as_str()) {
                    Some("fn") => SymbolKind::Function,
                    Some("struct") => SymbolKind::Struct,
                    Some("enum") => SymbolKind::Enum,
                    Some("trait") => SymbolKind::Trait,
                    Some("impl") => SymbolKind::Implementation,
                    Some("mod") => SymbolKind::Module,
                    _ => SymbolKind::Unknown,
                };
                symbols.push(SymbolInfo {
                    name: caps.get(3).map(|m| m.as_str().to_string()).unwrap_or_default(),
                    kind,
                    line: idx + 1,
                    column: 1,
                    parent: None,
                    visibility: caps.get(1).map(|m| m.as_str().to_string()),
                });
            } else if let Some(caps) = re_js_ts.captures(line) {
                let kind = match caps.get(3).map(|m| m.as_str()) {
                    Some("function") => SymbolKind::Function,
                    Some("class") => SymbolKind::Class,
                    Some("interface") => SymbolKind::Interface,
                    Some("type") => SymbolKind::TypeAlias,
                    Some("const") => SymbolKind::Constant,
                    _ => SymbolKind::Unknown,
                };
                symbols.push(SymbolInfo {
                    name: caps.get(4).map(|m| m.as_str().to_string()).unwrap_or_default(),
                    kind,
                    line: idx + 1,
                    column: 1,
                    parent: None,
                    visibility: caps.get(1).map(|m| m.as_str().to_string()),
                });
            }
        }
        symbols
    }
}
