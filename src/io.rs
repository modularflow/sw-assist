use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
pub async fn read_file_to_string_async(path: &Path) -> Result<String> {
    let data = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("reading file: {}", path.display()))?;
    Ok(data)
}

pub async fn read_file_segment_range_async(path: &Path, start: usize, end: usize) -> Result<String> {
    // start/end are 1-based inclusive line numbers
    let text = read_file_to_string_async(path).await?;
    let mut result = String::new();
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        if line_no >= start && line_no <= end {
            result.push_str(line);
            result.push('\n');
        }
    }
    Ok(result)
}

/// Chunk text by approximate token limit. Returns Vec of (chunk_index, text)
pub fn chunk_text_for_token_limit(text: &str, max_tokens_per_chunk: usize) -> Vec<(usize, String)> {
    if text.is_empty() {
        return vec![];
    }
    // Heuristic: 1 token ~= 4 chars
    let approx_chars_per_token = 4usize;
    let max_chars = max_tokens_per_chunk.saturating_mul(approx_chars_per_token);
    if max_chars == 0 {
        return vec![(0, String::new())];
    }
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let bytes = text.as_bytes();
    let mut idx = 0usize;
    let mut last_break = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'\n' || bytes[idx] == b' ' { last_break = idx; }
        if idx - start >= max_chars {
            let split = if last_break > start { last_break } else { idx };
            let piece = &text[start..split];
            chunks.push(piece.to_string());
            start = split + 1; // skip break char
            last_break = start;
        }
        idx += 1;
    }
    if start < text.len() {
        chunks.push(text[start..].to_string());
    }
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, s)| (i, s))
        .collect()
}

pub fn filename_only(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

pub async fn read_diff_file_async(path: &Path) -> Result<String> {
    let data = read_file_to_string_async(path).await?;
    // Basic validation: look for diff headers
    if !(data.contains("--- ") && data.contains("+++ ")) {
        // still return content; caller can decide
        return Ok(data);
    }
    Ok(data)
}

pub fn scan_todos(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let ln = i + 1;
        let upper = line.to_uppercase();
        if upper.contains("TODO") || upper.contains("FIXME") || upper.contains("NOTE:") || upper.starts_with("NOTE") {
            out.push((ln, line.trim().to_string()));
        }
    }
    out
}

/// Write text content to a file asynchronously
pub async fn write_file_async(path: &Path, content: &str) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating directory: {}", parent.display()))?;
    }
    
    tokio::fs::write(path, content)
        .await
        .with_context(|| format!("writing file: {}", path.display()))?;
    Ok(())
}

/// Create a backup of a file before modification
pub async fn backup_file_async(path: &Path) -> Result<std::path::PathBuf> {
    if !path.exists() {
        return Ok(path.to_path_buf()); // No backup needed for new files
    }
    
    let backup_path = path.with_extension(format!("{}.backup", 
        path.extension().and_then(|s| s.to_str()).unwrap_or("txt")));
    
    tokio::fs::copy(path, &backup_path)
        .await
        .with_context(|| format!("creating backup: {} -> {}", path.display(), backup_path.display()))?;
    
    Ok(backup_path)
}

/// Generate a unified diff between two strings
pub fn generate_unified_diff(original: &str, new: &str, filename: &str) -> String {
    use std::fmt::Write;
    
    let original_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    
    // Simple implementation - in practice you'd use a proper diff algorithm
    let mut diff = String::new();
    
    // Diff header
    writeln!(diff, "--- a/{}", filename).unwrap();
    writeln!(diff, "+++ b/{}", filename).unwrap();
    
    if original_lines.is_empty() && !new_lines.is_empty() {
        // New file
        writeln!(diff, "@@ -0,0 +1,{} @@", new_lines.len()).unwrap();
        for line in &new_lines {
            writeln!(diff, "+{}", line).unwrap();
        }
    } else if !original_lines.is_empty() && new_lines.is_empty() {
        // File deleted
        writeln!(diff, "@@ -1,{} +0,0 @@", original_lines.len()).unwrap();
        for line in &original_lines {
            writeln!(diff, "-{}", line).unwrap();
        }
    } else {
        // File modified - simple line-by-line comparison
        let max_len = original_lines.len().max(new_lines.len());
        if max_len > 0 {
            writeln!(diff, "@@ -1,{} +1,{} @@", original_lines.len(), new_lines.len()).unwrap();
            
            // Show all original lines as removed
            for line in &original_lines {
                writeln!(diff, "-{}", line).unwrap();
            }
            // Show all new lines as added
            for line in &new_lines {
                writeln!(diff, "+{}", line).unwrap();
            }
        }
    }
    
    diff
}

/// Apply a unified diff to a file
pub fn apply_diff_to_content(original_content: &str, diff_content: &str) -> Result<String> {
    // This is a simplified diff parser. In production, you'd want a more robust implementation
    // For now, we'll look for simple +/- line patterns
    
    let mut result_lines: Vec<String> = original_content.lines().map(|s| s.to_string()).collect();
    let mut _line_offset = 0i32;
    
    for diff_line in diff_content.lines() {
        if diff_line.starts_with("@@") {
            // Parse hunk header like "@@ -1,4 +1,5 @@"
            if let Some(captures) = parse_hunk_header(diff_line) {
                _line_offset = captures.new_start as i32 - captures.old_start as i32;
            }
        } else if diff_line.starts_with('-') && !diff_line.starts_with("---") {
            // Remove line (find and remove the matching line)
            let line_content = &diff_line[1..]; // Remove the '-' prefix
            if let Some(pos) = result_lines.iter().position(|line| line == line_content) {
                result_lines.remove(pos);
            }
        } else if diff_line.starts_with('+') && !diff_line.starts_with("+++") {
            // Add line (insert at appropriate position)
            let line_content = diff_line[1..].to_string(); // Remove the '+' prefix
            // For simplicity, append new lines at the end
            // A more sophisticated implementation would track line numbers
            result_lines.push(line_content);
        }
    }
    
    Ok(result_lines.join("\n"))
}

#[derive(Debug)]
struct HunkHeader {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
}

fn parse_hunk_header(line: &str) -> Option<HunkHeader> {
    // Parse "@@ -old_start,old_count +new_start,new_count @@"
    if let Some(content) = line.strip_prefix("@@").and_then(|s| s.strip_suffix("@@")) {
        let parts: Vec<&str> = content.trim().split_whitespace().collect();
        if parts.len() >= 2 {
            let old_part = parts[0].strip_prefix('-')?;
            let new_part = parts[1].strip_prefix('+')?;
            
            let parse_range = |s: &str| -> Option<(usize, usize)> {
                if let Some((start, count)) = s.split_once(',') {
                    Some((start.parse().ok()?, count.parse().ok()?))
                } else {
                    Some((s.parse().ok()?, 1))
                }
            };
            
            let (old_start, old_count) = parse_range(old_part)?;
            let (new_start, new_count) = parse_range(new_part)?;
            
            return Some(HunkHeader { old_start, old_count, new_start, new_count });
        }
    }
    None
}

/// Git-aware file operations
pub mod git {
    use super::*;
    
    /// Find the git repository root by looking for .git directory
    pub fn find_git_root(start_path: &Path) -> Option<PathBuf> {
        let mut current = start_path;
        loop {
            if current.join(".git").exists() {
                return Some(current.to_path_buf());
            }
            current = current.parent()?;
        }
    }
    
    /// Check if a path should be ignored according to .gitignore
    pub fn is_ignored_by_git(path: &Path, git_root: Option<&Path>) -> bool {
        // Basic gitignore patterns - in practice you'd want a proper gitignore parser
        let common_ignored = [
            "node_modules", ".git", "target", "dist", "build", ".DS_Store",
            "*.log", "*.tmp", ".env", ".env.local", "coverage", "__pycache__",
            ".pytest_cache", ".mypy_cache", "*.pyc", "*.pyo", ".vscode",
            ".idea", "*.swp", "*.swo", ".cache"
        ];
        
        let path_str = path.to_string_lossy();
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        
        // Check against common patterns
        for pattern in &common_ignored {
            if pattern.contains('*') {
                let prefix = pattern.strip_suffix('*').unwrap_or(pattern);
                if filename.starts_with(prefix) || path_str.contains(prefix) {
                    return true;
                }
            } else if filename == *pattern || path_str.contains(&format!("/{}/", pattern)) {
                return true;
            }
        }
        
        // If we have a git root, check for actual .gitignore file
        if let Some(git_root) = git_root {
            let gitignore_path = git_root.join(".gitignore");
            if gitignore_path.exists() {
                // For now, just check some basic patterns
                // In a full implementation, you'd parse the .gitignore file properly
                return false; // Simplified - assume not ignored if we can't parse
            }
        }
        
        false
    }
    
    /// Get files in a directory, respecting .gitignore
    pub async fn list_files_git_aware(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
        let git_root = find_git_root(dir);
        let mut files = Vec::new();
        
        collect_files_recursive(dir, &mut files, recursive, git_root.as_deref()).await?;
        
        Ok(files)
    }
    
    fn collect_files_recursive<'a>(
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
        recursive: bool,
        git_root: Option<&'a Path>
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            if is_ignored_by_git(dir, git_root) {
                return Ok(());
            }
            
            let mut entries = tokio::fs::read_dir(dir).await
                .with_context(|| format!("reading directory: {}", dir.display()))?;
                
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                
                if is_ignored_by_git(&path, git_root) {
                    continue;
                }
                
                if path.is_file() {
                    files.push(path);
                } else if path.is_dir() && recursive {
                    collect_files_recursive(&path, files, recursive, git_root).await?;
                }
            }
            
            Ok(())
        })
    }
}

/// Enhanced file operations with pattern matching and batch processing
pub mod batch {
    use super::*;
    
    /// Pattern matching for file selection
    pub struct FilePattern {
        pub include_extensions: Vec<String>,
        pub exclude_extensions: Vec<String>,
        pub include_patterns: Vec<String>,
        pub exclude_patterns: Vec<String>,
    }
    
    impl Default for FilePattern {
        fn default() -> Self {
            Self {
                include_extensions: Vec::new(),
                exclude_extensions: Vec::new(),
                include_patterns: Vec::new(),
                exclude_patterns: Vec::new(),
            }
        }
    }
    
    impl FilePattern {
        pub fn new() -> Self {
            Self::default()
        }
        
        pub fn include_extension(mut self, ext: impl Into<String>) -> Self {
            self.include_extensions.push(ext.into());
            self
        }
        
        pub fn exclude_extension(mut self, ext: impl Into<String>) -> Self {
            self.exclude_extensions.push(ext.into());
            self
        }
        
        pub fn include_pattern(mut self, pattern: impl Into<String>) -> Self {
            self.include_patterns.push(pattern.into());
            self
        }
        
        pub fn exclude_pattern(mut self, pattern: impl Into<String>) -> Self {
            self.exclude_patterns.push(pattern.into());
            self
        }
        
        pub fn matches(&self, path: &Path) -> bool {
            let path_str = path.to_string_lossy();
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            
            // Check exclusions first
            if !self.exclude_extensions.is_empty() {
                if self.exclude_extensions.iter().any(|ext| ext == extension) {
                    return false;
                }
            }
            
            if !self.exclude_patterns.is_empty() {
                if self.exclude_patterns.iter().any(|pattern| path_str.contains(pattern)) {
                    return false;
                }
            }
            
            // Check inclusions
            if !self.include_extensions.is_empty() {
                if !self.include_extensions.iter().any(|ext| ext == extension) {
                    return false;
                }
            }
            
            if !self.include_patterns.is_empty() {
                if !self.include_patterns.iter().any(|pattern| path_str.contains(pattern)) {
                    return false;
                }
            }
            
            true
        }
    }
    
    /// Find files matching patterns with git-awareness
    pub async fn find_files(
        root: &Path,
        pattern: &FilePattern,
        recursive: bool,
        git_aware: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut all_files = if git_aware {
            git::list_files_git_aware(root, recursive).await?
        } else {
            let mut files = Vec::new();
            collect_all_files(root, &mut files, recursive).await?;
            files
        };
        
        // Filter by pattern
        all_files.retain(|path| pattern.matches(path));
        
        Ok(all_files)
    }
    
    fn collect_all_files<'a>(
        dir: &'a Path, 
        files: &'a mut Vec<PathBuf>, 
        recursive: bool
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            let mut entries = tokio::fs::read_dir(dir).await
                .with_context(|| format!("reading directory: {}", dir.display()))?;
                
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                
                if path.is_file() {
                    files.push(path);
                } else if path.is_dir() && recursive {
                    collect_all_files(&path, files, recursive).await?;
                }
            }
            
            Ok(())
        })
    }
    
    /// Apply a function to multiple files with progress tracking
    pub async fn process_files<F, Fut>(
        files: Vec<PathBuf>,
        mut processor: F,
    ) -> Result<Vec<Result<String>>>
    where
        F: FnMut(PathBuf) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let mut results = Vec::new();
        
        for file in files {
            let result = processor(file).await;
            results.push(result);
        }
        
        Ok(results)
    }
}

/// Checkpointing and recovery system
pub mod checkpoint {
    use super::*;
    use std::time::SystemTime;
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Checkpoint {
        pub id: String,
        pub timestamp: u64,
        pub description: String,
        pub files: Vec<CheckpointFile>,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct CheckpointFile {
        pub path: PathBuf,
        pub content: String,
        pub hash: String,
    }
    
    impl Checkpoint {
        pub fn new(description: impl Into<String>) -> Self {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            Self {
                id: format!("checkpoint_{}", timestamp),
                timestamp,
                description: description.into(),
                files: Vec::new(),
            }
        }
        
        pub async fn add_file(&mut self, path: &Path) -> Result<()> {
            if !path.exists() {
                return Ok(());
            }
            
            let content = read_file_to_string_async(path).await?;
            let hash = format!("{:x}", md5::compute(&content));
            
            self.files.push(CheckpointFile {
                path: path.to_path_buf(),
                content,
                hash,
            });
            
            Ok(())
        }
        
        pub async fn save(&self, checkpoint_dir: &Path) -> Result<PathBuf> {
            tokio::fs::create_dir_all(checkpoint_dir).await?;
            
            let checkpoint_file = checkpoint_dir.join(format!("{}.json", self.id));
            let json = serde_json::to_string_pretty(self)?;
            
            write_file_async(&checkpoint_file, &json).await?;
            
            Ok(checkpoint_file)
        }
        
        pub async fn load(checkpoint_file: &Path) -> Result<Self> {
            let content = read_file_to_string_async(checkpoint_file).await?;
            let checkpoint: Checkpoint = serde_json::from_str(&content)?;
            Ok(checkpoint)
        }
        
        pub async fn restore(&self) -> Result<()> {
            for file in &self.files {
                // Create backup of current state
                if file.path.exists() {
                    backup_file_async(&file.path).await?;
                }
                
                // Restore from checkpoint
                write_file_async(&file.path, &file.content).await?;
            }
            Ok(())
        }
    }
    
    /// Create automatic checkpoint before file modifications
    pub async fn create_auto_checkpoint(
        files: &[PathBuf],
        description: impl Into<String>,
    ) -> Result<PathBuf> {
        let checkpoint_dir = std::env::current_dir()?.join(".sw-checkpoints");
        let mut checkpoint = Checkpoint::new(description);
        
        for file in files {
            checkpoint.add_file(file).await?;
        }
        
        checkpoint.save(&checkpoint_dir).await
    }
    
    /// List available checkpoints
    pub async fn list_checkpoints() -> Result<Vec<Checkpoint>> {
        let checkpoint_dir = std::env::current_dir()?.join(".sw-checkpoints");
        
        if !checkpoint_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut checkpoints = Vec::new();
        let mut entries = tokio::fs::read_dir(&checkpoint_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(checkpoint) = Checkpoint::load(&path).await {
                    checkpoints.push(checkpoint);
                }
            }
        }
        
        // Sort by timestamp (newest first)
        checkpoints.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(checkpoints)
    }
}

pub mod analysis {
    use super::*;
    use std::collections::HashMap;
    use regex::Regex;

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct FileAnalysis {
        pub file_path: PathBuf,
        pub file_type: FileType,
        pub language: String,
        pub lines_of_code: usize,
        pub dependencies: Vec<Dependency>,
        pub exports: Vec<Export>,
        pub functions: Vec<Function>,
        pub classes: Vec<Class>,
        pub imports: Vec<Import>,
        pub todos: Vec<Todo>,
        pub complexity: ComplexityMetrics,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub enum FileType {
        Source,
        Test,
        Config,
        Documentation,
        Build,
        Unknown,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Dependency {
        pub name: String,
        pub version: Option<String>,
        pub source: DependencySource,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub enum DependencySource {
        Import,
        Require,
        Package,
        Include,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Export {
        pub name: String,
        pub export_type: ExportType,
        pub line: usize,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub enum ExportType {
        Function,
        Class,
        Variable,
        Default,
        Named,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Function {
        pub name: String,
        pub parameters: Vec<String>,
        pub return_type: Option<String>,
        pub line_start: usize,
        pub line_end: usize,
        pub is_async: bool,
        pub visibility: Visibility,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Class {
        pub name: String,
        pub extends: Option<String>,
        pub implements: Vec<String>,
        pub line_start: usize,
        pub line_end: usize,
        pub methods: Vec<Function>,
        pub properties: Vec<Property>,
        pub visibility: Visibility,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Property {
        pub name: String,
        pub property_type: Option<String>,
        pub line: usize,
        pub visibility: Visibility,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub enum Visibility {
        Public,
        Private,
        Protected,
        Internal,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Import {
        pub module: String,
        pub items: Vec<String>,
        pub alias: Option<String>,
        pub line: usize,
        pub import_type: ImportType,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub enum ImportType {
        Default,
        Named,
        Star,
        Side,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Todo {
        pub content: String,
        pub line: usize,
        pub todo_type: TodoType,
        pub assigned: Option<String>,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub enum TodoType {
        Todo,
        Fixme,
        Hack,
        Note,
        Bug,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct ComplexityMetrics {
        pub cyclomatic_complexity: usize,
        pub cognitive_complexity: usize,
        pub nesting_depth: usize,
        pub function_count: usize,
        pub class_count: usize,
    }

    impl FileAnalysis {
        pub async fn analyze_file(path: &Path) -> Result<FileAnalysis> {
            let content = read_file_to_string_async(path).await?;
            let extension = path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            let language = detect_language(extension, &content);
            let file_type = detect_file_type(path, &content);
            
            let mut analysis = FileAnalysis {
                file_path: path.to_path_buf(),
                file_type,
                language: language.clone(),
                lines_of_code: count_lines_of_code(&content),
                dependencies: Vec::new(),
                exports: Vec::new(),
                functions: Vec::new(),
                classes: Vec::new(),
                imports: Vec::new(),
                todos: Vec::new(),
                complexity: ComplexityMetrics {
                    cyclomatic_complexity: 0,
                    cognitive_complexity: 0,
                    nesting_depth: 0,
                    function_count: 0,
                    class_count: 0,
                },
            };

            // Analyze based on language
            match language.as_str() {
                "javascript" | "typescript" => analyze_javascript(&mut analysis, &content)?,
                "python" => analyze_python(&mut analysis, &content)?,
                "rust" => analyze_rust(&mut analysis, &content)?,
                "java" => analyze_java(&mut analysis, &content)?,
                _ => analyze_generic(&mut analysis, &content)?,
            }

            Ok(analysis)
        }

        pub fn summary(&self) -> String {
            format!(
                "File: {}\nLanguage: {}\nType: {:?}\nLines: {}\nFunctions: {}\nClasses: {}\nImports: {}\nTODOs: {}\nComplexity: {} cyclomatic, {} cognitive",
                self.file_path.display(),
                self.language,
                self.file_type,
                self.lines_of_code,
                self.functions.len(),
                self.classes.len(),
                self.imports.len(),
                self.todos.len(),
                self.complexity.cyclomatic_complexity,
                self.complexity.cognitive_complexity
            )
        }
    }

    pub async fn analyze_directory(dir_path: &Path, recursive: bool, patterns: Option<&super::batch::FilePattern>) -> Result<Vec<FileAnalysis>> {
        let files = if let Some(pattern) = patterns {
            super::batch::find_files(dir_path, pattern, recursive, true).await?
        } else {
            let mut default_pattern = super::batch::FilePattern::new();
            for ext in ["js", "ts", "py", "rs", "java", "cpp", "c", "h", "hpp"] {
                default_pattern = default_pattern.include_extension(ext);
            }
            super::batch::find_files(dir_path, &default_pattern, recursive, true).await?
        };

        let mut analyses = Vec::new();
        for file in files {
            if let Ok(analysis) = FileAnalysis::analyze_file(&file).await {
                analyses.push(analysis);
            }
        }

        Ok(analyses)
    }

    pub fn generate_dependency_graph(analyses: &[FileAnalysis]) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();
        
        for analysis in analyses {
            let file_name = analysis.file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            let dependencies: Vec<String> = analysis.imports.iter()
                .map(|imp| imp.module.clone())
                .collect();
            
            graph.insert(file_name, dependencies);
        }
        
        graph
    }

    fn detect_language(extension: &str, content: &str) -> String {
        match extension {
            "js" | "jsx" | "mjs" => "javascript".to_string(),
            "ts" | "tsx" => "typescript".to_string(),
            "py" | "pyw" => "python".to_string(),
            "rs" => "rust".to_string(),
            "java" => "java".to_string(),
            "cpp" | "cc" | "cxx" => "cpp".to_string(),
            "c" => "c".to_string(),
            "h" | "hpp" => "c_header".to_string(),
            "go" => "go".to_string(),
            "php" => "php".to_string(),
            "rb" => "ruby".to_string(),
            "swift" => "swift".to_string(),
            "kt" => "kotlin".to_string(),
            "scala" => "scala".to_string(),
            "hs" => "haskell".to_string(),
            "ml" => "ocaml".to_string(),
            "sh" | "bash" => "shell".to_string(),
            _ => {
                // Try to detect from content
                if content.contains("#!/usr/bin/env python") || content.contains("import ") {
                    "python".to_string()
                } else if content.contains("function ") || content.contains("const ") || content.contains("let ") {
                    "javascript".to_string()
                } else if content.contains("fn ") || content.contains("use ") || content.contains("pub ") {
                    "rust".to_string()
                } else {
                    "unknown".to_string()
                }
            }
        }
    }

    fn detect_file_type(path: &Path, content: &str) -> FileType {
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if filename.contains("test") || filename.contains("spec") || path.to_string_lossy().contains("/test/") {
            FileType::Test
        } else if filename.ends_with(".md") || filename.ends_with(".txt") || filename.ends_with(".doc") {
            FileType::Documentation
        } else if filename.contains("config") || filename.contains("setting") || 
                 filename.ends_with(".json") || filename.ends_with(".yaml") || filename.ends_with(".toml") {
            FileType::Config
        } else if filename.contains("build") || filename.contains("make") || filename.contains("cmake") ||
                 filename.contains("package") || filename.contains("cargo") {
            FileType::Build
        } else if content.len() > 0 && (content.contains("function") || content.contains("class") || content.contains("def ")) {
            FileType::Source
        } else {
            FileType::Unknown
        }
    }

    fn count_lines_of_code(content: &str) -> usize {
        content.lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("#")
            })
            .count()
    }

    fn analyze_javascript(analysis: &mut FileAnalysis, content: &str) -> Result<()> {
        // Import analysis
        let import_re = Regex::new(r#"(?m)^(?:import|const|let|var)\s+(?:\{([^}]+)\}|\*\s+as\s+(\w+)|(\w+))\s+from\s+["']([^"']+)["']"#)?;
        for caps in import_re.captures_iter(content) {
            let module = caps.get(4).map_or("", |m| m.as_str()).to_string();
            let line = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            
            let items = if let Some(named) = caps.get(1) {
                named.as_str().split(',').map(|s| s.trim().to_string()).collect()
            } else if let Some(star) = caps.get(2) {
                vec![star.as_str().to_string()]
            } else if let Some(default) = caps.get(3) {
                vec![default.as_str().to_string()]
            } else {
                vec![]
            };

            analysis.imports.push(Import {
                module,
                items,
                alias: None,
                line,
                import_type: ImportType::Named,
            });
        }

        // Function analysis
        let func_re = Regex::new(r"(?m)^(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)")?;
        for caps in func_re.captures_iter(content) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let params_str = caps.get(2).map_or("", |m| m.as_str());
            let parameters: Vec<String> = params_str.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            
            let line_start = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            let is_async = caps.get(0).unwrap().as_str().contains("async");

            analysis.functions.push(Function {
                name,
                parameters,
                return_type: None,
                line_start,
                line_end: line_start + 1, // Simplified
                is_async,
                visibility: Visibility::Public,
            });
        }

        // Export analysis
        let export_re = Regex::new(r"(?m)^export\s+(?:default\s+)?(?:function|class|const|let|var)\s+(\w+)")?;
        for caps in export_re.captures_iter(content) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let line = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            let is_default = caps.get(0).unwrap().as_str().contains("default");

            analysis.exports.push(Export {
                name,
                export_type: if is_default { ExportType::Default } else { ExportType::Named },
                line,
            });
        }

        // TODO analysis
        analyze_todos(analysis, content)?;

        analysis.complexity.function_count = analysis.functions.len();
        analysis.complexity.cyclomatic_complexity = calculate_cyclomatic_complexity(content);

        Ok(())
    }

    fn analyze_python(analysis: &mut FileAnalysis, content: &str) -> Result<()> {
        // Import analysis
        let import_re = Regex::new(r"(?m)^(?:from\s+(\S+)\s+)?import\s+(.+)")?;
        for caps in import_re.captures_iter(content) {
            let module = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let items_str = caps.get(2).map_or("", |m| m.as_str());
            let line = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            
            let items: Vec<String> = items_str.split(',')
                .map(|s| s.trim().to_string())
                .collect();

            analysis.imports.push(Import {
                module: if module.is_empty() { items_str.to_string() } else { module },
                items,
                alias: None,
                line,
                import_type: ImportType::Named,
            });
        }

        // Function analysis
        let func_re = Regex::new(r"(?m)^(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^:]+))?")?;
        for caps in func_re.captures_iter(content) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let params_str = caps.get(2).map_or("", |m| m.as_str());
            let return_type = caps.get(3).map(|m| m.as_str().trim().to_string());
            
            let parameters: Vec<String> = params_str.split(',')
                .map(|p| p.trim().split(':').next().unwrap_or(p.trim()).to_string())
                .filter(|p| !p.is_empty() && p != "self")
                .collect();
            
            let line_start = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            let is_async = caps.get(0).unwrap().as_str().contains("async");

            analysis.functions.push(Function {
                name,
                parameters,
                return_type,
                line_start,
                line_end: line_start + 1, // Simplified
                is_async,
                visibility: Visibility::Public,
            });
        }

        // Class analysis
        let class_re = Regex::new(r"(?m)^class\s+(\w+)(?:\(([^)]*)\))?")?;
        for caps in class_re.captures_iter(content) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let extends_str = caps.get(2).map_or("", |m| m.as_str());
            let line_start = content[..caps.get(0).unwrap().start()].lines().count() + 1;

            analysis.classes.push(Class {
                name,
                extends: if extends_str.is_empty() { None } else { Some(extends_str.to_string()) },
                implements: Vec::new(),
                line_start,
                line_end: line_start + 1, // Simplified
                methods: Vec::new(),
                properties: Vec::new(),
                visibility: Visibility::Public,
            });
        }

        analyze_todos(analysis, content)?;

        analysis.complexity.function_count = analysis.functions.len();
        analysis.complexity.class_count = analysis.classes.len();
        analysis.complexity.cyclomatic_complexity = calculate_cyclomatic_complexity(content);

        Ok(())
    }

    fn analyze_rust(analysis: &mut FileAnalysis, content: &str) -> Result<()> {
        // Use/import analysis
        let use_re = Regex::new(r"(?m)^use\s+([^;]+);")?;
        for caps in use_re.captures_iter(content) {
            let use_str = caps.get(1).map_or("", |m| m.as_str());
            let line = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            
            analysis.imports.push(Import {
                module: use_str.to_string(),
                items: vec![],
                alias: None,
                line,
                import_type: ImportType::Named,
            });
        }

        // Function analysis
        let func_re = Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^{]+))?")?;
        for caps in func_re.captures_iter(content) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let params_str = caps.get(2).map_or("", |m| m.as_str());
            let return_type = caps.get(3).map(|m| m.as_str().trim().to_string());
            
            let parameters: Vec<String> = params_str.split(',')
                .map(|p| {
                    let param = p.trim();
                    if let Some(colon_pos) = param.find(':') {
                        param[..colon_pos].trim().to_string()
                    } else {
                        param.to_string()
                    }
                })
                .filter(|p| !p.is_empty() && p != "&self" && p != "&mut self" && p != "self")
                .collect();
            
            let line_start = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            let is_async = caps.get(0).unwrap().as_str().contains("async");
            let is_pub = caps.get(0).unwrap().as_str().contains("pub");

            analysis.functions.push(Function {
                name,
                parameters,
                return_type,
                line_start,
                line_end: line_start + 1, // Simplified
                is_async,
                visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            });
        }

        analyze_todos(analysis, content)?;

        analysis.complexity.function_count = analysis.functions.len();
        analysis.complexity.cyclomatic_complexity = calculate_cyclomatic_complexity(content);

        Ok(())
    }

    fn analyze_java(analysis: &mut FileAnalysis, content: &str) -> Result<()> {
        // Import analysis
        let import_re = Regex::new(r"(?m)^import\s+(?:static\s+)?([^;]+);")?;
        for caps in import_re.captures_iter(content) {
            let import_str = caps.get(1).map_or("", |m| m.as_str());
            let line = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            
            analysis.imports.push(Import {
                module: import_str.to_string(),
                items: vec![],
                alias: None,
                line,
                import_type: ImportType::Named,
            });
        }

        // Class analysis
        let class_re = Regex::new(r"(?m)^(?:public\s+)?(?:abstract\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+([^{]+))?")?;
        for caps in class_re.captures_iter(content) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let extends = caps.get(2).map(|m| m.as_str().to_string());
            let implements_str = caps.get(3).map_or("", |m| m.as_str());
            let line_start = content[..caps.get(0).unwrap().start()].lines().count() + 1;

            let implements: Vec<String> = implements_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            analysis.classes.push(Class {
                name,
                extends,
                implements,
                line_start,
                line_end: line_start + 1, // Simplified
                methods: Vec::new(),
                properties: Vec::new(),
                visibility: Visibility::Public,
            });
        }

        analyze_todos(analysis, content)?;

        analysis.complexity.class_count = analysis.classes.len();
        analysis.complexity.cyclomatic_complexity = calculate_cyclomatic_complexity(content);

        Ok(())
    }

    fn analyze_generic(analysis: &mut FileAnalysis, content: &str) -> Result<()> {
        // Just analyze TODOs for generic files
        analyze_todos(analysis, content)?;
        Ok(())
    }

    fn analyze_todos(analysis: &mut FileAnalysis, content: &str) -> Result<()> {
        let todo_re = Regex::new(r"(?i)(?://|#|<!--)\s*(TODO|FIXME|HACK|NOTE|BUG)(?::|\s+)([^\r\n]*)")?;
        
        for caps in todo_re.captures_iter(content) {
            let todo_type_str = caps.get(1).map_or("", |m| m.as_str()).to_uppercase();
            let content_str = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
            let line = content[..caps.get(0).unwrap().start()].lines().count() + 1;

            let todo_type = match todo_type_str.as_str() {
                "TODO" => TodoType::Todo,
                "FIXME" => TodoType::Fixme,
                "HACK" => TodoType::Hack,
                "NOTE" => TodoType::Note,
                "BUG" => TodoType::Bug,
                _ => TodoType::Todo,
            };

            analysis.todos.push(Todo {
                content: content_str,
                line,
                todo_type,
                assigned: None,
            });
        }

        Ok(())
    }

    fn calculate_cyclomatic_complexity(content: &str) -> usize {
        let decision_keywords = ["if", "else", "while", "for", "switch", "case", "catch", "&&", "||", "?"];
        let mut complexity = 1; // Base complexity

        for keyword in decision_keywords {
            complexity += content.matches(keyword).count();
        }

        complexity
    }
}

pub mod templates {
    use super::*;
    use std::collections::HashMap;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Template {
        pub name: String,
        pub description: String,
        pub language: String,
        pub files: Vec<TemplateFile>,
        pub variables: Vec<TemplateVariable>,
        pub dependencies: Vec<String>,
        pub scripts: HashMap<String, String>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TemplateFile {
        pub path: String,
        pub content: String,
        pub executable: bool,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TemplateVariable {
        pub name: String,
        pub description: String,
        pub default_value: Option<String>,
        pub required: bool,
    }

    #[derive(Debug, Clone)]
    pub struct TemplateContext {
        pub variables: HashMap<String, String>,
        pub project_name: String,
        pub author: String,
        pub timestamp: String,
    }

    impl Template {
        pub fn new(name: &str, description: &str, language: &str) -> Self {
            Template {
                name: name.to_string(),
                description: description.to_string(),
                language: language.to_string(),
                files: Vec::new(),
                variables: Vec::new(),
                dependencies: Vec::new(),
                scripts: HashMap::new(),
            }
        }

        pub fn add_file(mut self, path: &str, content: &str) -> Self {
            self.files.push(TemplateFile {
                path: path.to_string(),
                content: content.to_string(),
                executable: false,
            });
            self
        }

        pub fn add_executable_file(mut self, path: &str, content: &str) -> Self {
            self.files.push(TemplateFile {
                path: path.to_string(),
                content: content.to_string(),
                executable: true,
            });
            self
        }

        pub fn add_variable(mut self, name: &str, description: &str, default: Option<&str>, required: bool) -> Self {
            self.variables.push(TemplateVariable {
                name: name.to_string(),
                description: description.to_string(),
                default_value: default.map(|s| s.to_string()),
                required,
            });
            self
        }

        pub fn add_dependency(mut self, dep: &str) -> Self {
            self.dependencies.push(dep.to_string());
            self
        }

        pub fn add_script(mut self, name: &str, command: &str) -> Self {
            self.scripts.insert(name.to_string(), command.to_string());
            self
        }

        pub async fn generate(&self, output_dir: &Path, context: &TemplateContext) -> Result<Vec<PathBuf>> {
            let mut created_files = Vec::new();

            // Create output directory if it doesn't exist
            tokio::fs::create_dir_all(output_dir).await
                .with_context(|| format!("creating output directory: {}", output_dir.display()))?;

            // Generate files
            for template_file in &self.files {
                let rendered_path = self.render_template(&template_file.path, context)?;
                let rendered_content = self.render_template(&template_file.content, context)?;
                
                let file_path = output_dir.join(&rendered_path);
                
                // Create parent directories
                if let Some(parent) = file_path.parent() {
                    tokio::fs::create_dir_all(parent).await
                        .with_context(|| format!("creating parent directory: {}", parent.display()))?;
                }
                
                // Write file
                tokio::fs::write(&file_path, rendered_content).await
                    .with_context(|| format!("writing file: {}", file_path.display()))?;
                
                // Set executable permission if needed
                if template_file.executable {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = tokio::fs::metadata(&file_path).await?.permissions();
                        perms.set_mode(0o755);
                        tokio::fs::set_permissions(&file_path, perms).await?;
                    }
                }
                
                created_files.push(file_path);
            }

            Ok(created_files)
        }

        fn render_template(&self, template: &str, context: &TemplateContext) -> Result<String> {
            let mut result = template.to_string();
            
            // Replace built-in variables
            result = result.replace("{{project_name}}", &context.project_name);
            result = result.replace("{{author}}", &context.author);
            result = result.replace("{{timestamp}}", &context.timestamp);
            result = result.replace("{{year}}", &chrono::Utc::now().format("%Y").to_string());
            result = result.replace("{{date}}", &chrono::Utc::now().format("%Y-%m-%d").to_string());
            
            // Replace custom variables
            for (key, value) in &context.variables {
                let placeholder = format!("{{{{{}}}}}", key);
                result = result.replace(&placeholder, value);
            }
            
            Ok(result)
        }
    }

    pub fn get_builtin_templates() -> Vec<Template> {
        vec![
            create_rust_cli_template(),
            create_node_express_template(),
            create_python_fastapi_template(),
            create_react_component_template(),
            create_typescript_library_template(),
        ]
    }

    fn create_rust_cli_template() -> Template {
        Template::new("rust-cli", "Rust CLI application with clap", "rust")
            .add_variable("app_name", "Application name", Some("my-cli"), true)
            .add_variable("description", "Application description", Some("A CLI application"), false)
            .add_dependency("clap")
            .add_dependency("anyhow")
            .add_dependency("tokio")
            .add_file("Cargo.toml", r#"[package]
name = "{{app_name}}"
version = "0.1.0"
edition = "2021"
description = "{{description}}"
authors = ["{{author}}"]

[dependencies]
clap = { version = "4.0", features = ["derive"] }
anyhow = "1.0"
tokio = { version = "1.0", features = ["full"] }
"#)
            .add_file("src/main.rs", r#"use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "{{app_name}}")]
#[command(about = "{{description}}")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Example command
    Hello {
        /// Name to greet
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Hello { name } => {
            println!("Hello, {}!", name);
        }
    }

    Ok(())
}
"#)
            .add_file("README.md", r#"# {{project_name}}

{{description}}

## Installation

```bash
cargo install --path .
```

## Usage

```bash
{{app_name}} hello "World"
```

## License

MIT License
"#)
            .add_script("build", "cargo build")
            .add_script("test", "cargo test")
            .add_script("run", "cargo run")
    }

    fn create_node_express_template() -> Template {
        Template::new("node-express", "Node.js Express API server", "javascript")
            .add_variable("app_name", "Application name", Some("my-api"), true)
            .add_variable("port", "Server port", Some("3000"), false)
            .add_file("package.json", r#"{
  "name": "{{app_name}}",
  "version": "1.0.0",
  "description": "Express API server",
  "main": "server.js",
  "scripts": {
    "start": "node server.js",
    "dev": "nodemon server.js",
    "test": "jest"
  },
  "dependencies": {
    "express": "^4.18.0",
    "cors": "^2.8.5",
    "helmet": "^6.0.0",
    "morgan": "^1.10.0"
  },
  "devDependencies": {
    "nodemon": "^2.0.20",
    "jest": "^29.0.0"
  },
  "author": "{{author}}",
  "license": "MIT"
}
"#)
            .add_file("server.js", r#"const express = require('express');
const cors = require('cors');
const helmet = require('helmet');
const morgan = require('morgan');

const app = express();
const PORT = process.env.PORT || {{port}};

// Middleware
app.use(helmet());
app.use(cors());
app.use(morgan('combined'));
app.use(express.json());

// Routes
app.get('/', (req, res) => {
    res.json({ message: 'Welcome to {{app_name}} API' });
});

app.get('/health', (req, res) => {
    res.json({ status: 'healthy', timestamp: new Date().toISOString() });
});

// Error handling
app.use((err, req, res, next) => {
    console.error(err.stack);
    res.status(500).json({ error: 'Something went wrong!' });
});

app.listen(PORT, () => {
    console.log(`{{app_name}} server running on port ${PORT}`);
});
"#)
            .add_file(".gitignore", r#"node_modules/
.env
.env.local
npm-debug.log*
yarn-debug.log*
yarn-error.log*
.DS_Store
"#)
            .add_script("start", "npm start")
            .add_script("dev", "npm run dev")
            .add_script("test", "npm test")
    }

    fn create_python_fastapi_template() -> Template {
        Template::new("python-fastapi", "Python FastAPI web service", "python")
            .add_variable("app_name", "Application name", Some("my-api"), true)
            .add_variable("description", "API description", Some("FastAPI web service"), false)
            .add_file("requirements.txt", r#"fastapi==0.104.1
uvicorn[standard]==0.24.0
pydantic==2.5.0
python-multipart==0.0.6
"#)
            .add_file("main.py", r#"from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Optional
import uvicorn

app = FastAPI(
    title="{{app_name}}",
    description="{{description}}",
    version="1.0.0"
)

class Item(BaseModel):
    id: Optional[int] = None
    name: str
    description: Optional[str] = None

# In-memory storage (use a database in production)
items: List[Item] = []

@app.get("/")
async def root():
    return {"message": "Welcome to {{app_name}}"}

@app.get("/health")
async def health_check():
    return {"status": "healthy"}

@app.get("/items", response_model=List[Item])
async def get_items():
    return items

@app.post("/items", response_model=Item)
async def create_item(item: Item):
    item.id = len(items) + 1
    items.append(item)
    return item

@app.get("/items/{item_id}", response_model=Item)
async def get_item(item_id: int):
    for item in items:
        if item.id == item_id:
            return item
    raise HTTPException(status_code=404, detail="Item not found")

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
"#)
            .add_file("README.md", r#"# {{project_name}}

{{description}}

## Setup

```bash
pip install -r requirements.txt
```

## Run

```bash
python main.py
```

Or with uvicorn:

```bash
uvicorn main:app --reload
```

## API Documentation

Visit http://localhost:8000/docs for interactive API documentation.
"#)
            .add_script("start", "python main.py")
            .add_script("dev", "uvicorn main:app --reload")
    }

    fn create_react_component_template() -> Template {
        Template::new("react-component", "React functional component with hooks", "javascript")
            .add_variable("component_name", "Component name", Some("MyComponent"), true)
            .add_variable("use_typescript", "Use TypeScript", Some("false"), false)
            .add_file("{{component_name}}.jsx", r#"import React, { useState, useEffect } from 'react';
import PropTypes from 'prop-types';
import './{{component_name}}.css';

const {{component_name}} = ({ title, onAction }) => {
    const [state, setState] = useState(null);

    useEffect(() => {
        // Component initialization
        console.log('{{component_name}} mounted');
        
        return () => {
            // Cleanup
            console.log('{{component_name}} unmounted');
        };
    }, []);

    const handleClick = () => {
        if (onAction) {
            onAction('button clicked');
        }
    };

    return (
        <div className="{{component_name}}">
            <h2>{title}</h2>
            <button onClick={handleClick}>
                Action
            </button>
            {state && <p>State: {state}</p>}
        </div>
    );
};

{{component_name}}.propTypes = {
    title: PropTypes.string.isRequired,
    onAction: PropTypes.func,
};

{{component_name}}.defaultProps = {
    onAction: null,
};

export default {{component_name}};
"#)
            .add_file("{{component_name}}.css", r#".{{component_name}} {
    padding: 1rem;
    border: 1px solid #ddd;
    border-radius: 8px;
    background-color: #f9f9f9;
}

.{{component_name}} h2 {
    margin-top: 0;
    color: #333;
}

.{{component_name}} button {
    padding: 0.5rem 1rem;
    background-color: #007bff;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s;
}

.{{component_name}} button:hover {
    background-color: #0056b3;
}
"#)
            .add_file("{{component_name}}.test.jsx", r#"import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import {{component_name}} from './{{component_name}}';

describe('{{component_name}}', () => {
    test('renders with title', () => {
        render(<{{component_name}} title="Test Title" />);
        expect(screen.getByText('Test Title')).toBeInTheDocument();
    });

    test('calls onAction when button clicked', () => {
        const mockAction = jest.fn();
        render(<{{component_name}} title="Test" onAction={mockAction} />);
        
        fireEvent.click(screen.getByText('Action'));
        expect(mockAction).toHaveBeenCalledWith('button clicked');
    });
});
"#)
    }

    fn create_typescript_library_template() -> Template {
        Template::new("typescript-library", "TypeScript library with build setup", "typescript")
            .add_variable("lib_name", "Library name", Some("my-lib"), true)
            .add_variable("description", "Library description", Some("A TypeScript library"), false)
            .add_file("package.json", r#"{
  "name": "{{lib_name}}",
  "version": "1.0.0",
  "description": "{{description}}",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "test": "jest",
    "lint": "eslint src/**/*.ts",
    "prepare": "npm run build"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/jest": "^29.0.0",
    "jest": "^29.0.0",
    "ts-jest": "^29.0.0",
    "eslint": "^8.0.0",
    "@typescript-eslint/eslint-plugin": "^6.0.0",
    "@typescript-eslint/parser": "^6.0.0"
  },
  "author": "{{author}}",
  "license": "MIT"
}
"#)
            .add_file("tsconfig.json", r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "**/*.test.ts"]
}
"#)
            .add_file("src/index.ts", r#"/**
 * {{description}}
 * @author {{author}}
 */

export interface Config {
    debug?: boolean;
    timeout?: number;
}

export class {{lib_name}} {
    private config: Config;

    constructor(config: Config = {}) {
        this.config = {
            debug: false,
            timeout: 5000,
            ...config
        };
    }

    public hello(name: string): string {
        if (this.config.debug) {
            console.log(`Greeting ${name}`);
        }
        return `Hello, ${name}!`;
    }

    public getConfig(): Config {
        return { ...this.config };
    }
}

export default {{lib_name}};
"#)
            .add_file("src/index.test.ts", r#"import {{lib_name}} from './index';

describe('{{lib_name}}', () => {
    test('should create instance', () => {
        const lib = new {{lib_name}}();
        expect(lib).toBeInstanceOf({{lib_name}});
    });

    test('should greet user', () => {
        const lib = new {{lib_name}}();
        expect(lib.hello('World')).toBe('Hello, World!');
    });

    test('should accept config', () => {
        const lib = new {{lib_name}}({ debug: true, timeout: 1000 });
        const config = lib.getConfig();
        expect(config.debug).toBe(true);
        expect(config.timeout).toBe(1000);
    });
});
"#)
            .add_script("build", "npm run build")
            .add_script("test", "npm test")
            .add_script("lint", "npm run lint")
    }

    pub async fn list_templates() -> Result<Vec<Template>> {
        Ok(get_builtin_templates())
    }

    pub async fn generate_from_template(
        template_name: &str,
        output_dir: &Path,
        variables: HashMap<String, String>,
        project_name: &str,
        author: &str,
    ) -> Result<Vec<PathBuf>> {
        let templates = get_builtin_templates();
        let template = templates
            .into_iter()
            .find(|t| t.name == template_name)
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;

        let context = TemplateContext {
            variables,
            project_name: project_name.to_string(),
            author: author.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        template.generate(output_dir, &context).await
    }
}

pub mod sync {
    use super::*;
    use std::collections::HashMap;
    
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct FileDiff {
        pub path: PathBuf,
        pub status: DiffStatus,
        pub size_old: Option<u64>,
        pub size_new: Option<u64>,
        pub modified_old: Option<String>,
        pub modified_new: Option<String>,
        pub content_diff: Option<String>,
        pub similarity: Option<f64>,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub enum DiffStatus {
        Added,
        Deleted,
        Modified,
        Renamed { old_path: PathBuf },
        Identical,
    }

    #[derive(Debug, Clone)]
    pub struct SyncOptions {
        pub recursive: bool,
        pub include_content: bool,
        pub ignore_timestamps: bool,
        pub ignore_size: bool,
        pub similarity_threshold: f64,
        pub exclude_patterns: Vec<String>,
    }

    impl Default for SyncOptions {
        fn default() -> Self {
            SyncOptions {
                recursive: true,
                include_content: true,
                ignore_timestamps: false,
                ignore_size: false,
                similarity_threshold: 0.8,
                exclude_patterns: vec![".git".to_string(), "node_modules".to_string(), "target".to_string()],
            }
        }
    }

    pub async fn compare_directories(
        source_dir: &Path,
        target_dir: &Path,
        options: &SyncOptions,
    ) -> Result<Vec<FileDiff>> {
        let source_files = collect_files(source_dir, options).await?;
        let target_files = collect_files(target_dir, options).await?;
        
        let mut diffs = Vec::new();
        let mut processed_target_files = HashSet::new();
        
        // Check for added, modified, or identical files
        for (rel_path, source_metadata) in &source_files {
            if let Some(target_metadata) = target_files.get(rel_path) {
                processed_target_files.insert(rel_path);
                
                let diff = compare_files(
                    &source_dir.join(rel_path),
                    &target_dir.join(rel_path),
                    source_metadata,
                    target_metadata,
                    options,
                ).await?;
                
                diffs.push(diff);
            } else {
                // File exists in source but not in target
                diffs.push(FileDiff {
                    path: rel_path.clone(),
                    status: DiffStatus::Added,
                    size_old: None,
                    size_new: Some(source_metadata.len()),
                    modified_old: None,
                    modified_new: Some(format_timestamp(&source_metadata.modified()?)?),
                    content_diff: None,
                    similarity: None,
                });
            }
        }
        
        // Check for deleted files
        for (rel_path, target_metadata) in &target_files {
            if !processed_target_files.contains(rel_path) {
                diffs.push(FileDiff {
                    path: rel_path.clone(),
                    status: DiffStatus::Deleted,
                    size_old: Some(target_metadata.len()),
                    size_new: None,
                    modified_old: Some(format_timestamp(&target_metadata.modified()?)?),
                    modified_new: None,
                    content_diff: None,
                    similarity: None,
                });
            }
        }
        
        Ok(diffs)
    }

    pub async fn compare_files(
        source_file: &Path,
        target_file: &Path,
        source_metadata: &std::fs::Metadata,
        target_metadata: &std::fs::Metadata,
        options: &SyncOptions,
    ) -> Result<FileDiff> {
        let rel_path = source_file.file_name()
            .and_then(|n| n.to_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| source_file.to_path_buf());

        let size_old = target_metadata.len();
        let size_new = source_metadata.len();
        let modified_old = format_timestamp(&target_metadata.modified()?)?;
        let modified_new = format_timestamp(&source_metadata.modified()?)?;

        // Quick checks for identical files
        if !options.ignore_size && size_old != size_new {
            return Ok(FileDiff {
                path: rel_path,
                status: DiffStatus::Modified,
                size_old: Some(size_old),
                size_new: Some(size_new),
                modified_old: Some(modified_old),
                modified_new: Some(modified_new),
                content_diff: None,
                similarity: None,
            });
        }

        if !options.ignore_timestamps && modified_old != modified_new {
            if options.include_content {
                // Need to check content to be sure
                let content_diff = generate_content_diff(source_file, target_file).await?;
                if content_diff.is_empty() {
                    return Ok(FileDiff {
                        path: rel_path,
                        status: DiffStatus::Identical,
                        size_old: Some(size_old),
                        size_new: Some(size_new),
                        modified_old: Some(modified_old),
                        modified_new: Some(modified_new),
                        content_diff: None,
                        similarity: Some(1.0),
                    });
                } else {
                    return Ok(FileDiff {
                        path: rel_path,
                        status: DiffStatus::Modified,
                        size_old: Some(size_old),
                        size_new: Some(size_new),
                        modified_old: Some(modified_old),
                        modified_new: Some(modified_new),
                        content_diff: Some(content_diff),
                        similarity: None,
                    });
                }
            } else {
                return Ok(FileDiff {
                    path: rel_path,
                    status: DiffStatus::Modified,
                    size_old: Some(size_old),
                    size_new: Some(size_new),
                    modified_old: Some(modified_old),
                    modified_new: Some(modified_new),
                    content_diff: None,
                    similarity: None,
                });
            }
        }

        // Files appear identical
        Ok(FileDiff {
            path: rel_path,
            status: DiffStatus::Identical,
            size_old: Some(size_old),
            size_new: Some(size_new),
            modified_old: Some(modified_old),
            modified_new: Some(modified_new),
            content_diff: None,
            similarity: Some(1.0),
        })
    }

    pub async fn sync_files(
        source_dir: &Path,
        target_dir: &Path,
        diffs: &[FileDiff],
        dry_run: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut synced_files = Vec::new();
        
        for diff in diffs {
            match &diff.status {
                DiffStatus::Added => {
                    let source_path = source_dir.join(&diff.path);
                    let target_path = target_dir.join(&diff.path);
                    
                    if !dry_run {
                        if let Some(parent) = target_path.parent() {
                            tokio::fs::create_dir_all(parent).await?;
                        }
                        tokio::fs::copy(&source_path, &target_path).await?;
                    }
                    
                    synced_files.push(target_path);
                }
                DiffStatus::Modified => {
                    let source_path = source_dir.join(&diff.path);
                    let target_path = target_dir.join(&diff.path);
                    
                    if !dry_run {
                        tokio::fs::copy(&source_path, &target_path).await?;
                    }
                    
                    synced_files.push(target_path);
                }
                DiffStatus::Deleted => {
                    let target_path = target_dir.join(&diff.path);
                    
                    if !dry_run && target_path.exists() {
                        if target_path.is_file() {
                            tokio::fs::remove_file(&target_path).await?;
                        } else {
                            tokio::fs::remove_dir_all(&target_path).await?;
                        }
                    }
                    
                    synced_files.push(target_path);
                }
                DiffStatus::Renamed { old_path } => {
                    let source_path = source_dir.join(&diff.path);
                    let old_target_path = target_dir.join(old_path);
                    let new_target_path = target_dir.join(&diff.path);
                    
                    if !dry_run {
                        if old_target_path.exists() {
                            tokio::fs::rename(&old_target_path, &new_target_path).await?;
                        } else {
                            tokio::fs::copy(&source_path, &new_target_path).await?;
                        }
                    }
                    
                    synced_files.push(new_target_path);
                }
                DiffStatus::Identical => {
                    // No action needed
                }
            }
        }
        
        Ok(synced_files)
    }

    async fn collect_files(
        dir: &Path,
        options: &SyncOptions,
    ) -> Result<HashMap<PathBuf, std::fs::Metadata>> {
        let mut files = HashMap::new();
        collect_files_recursive(dir, dir, &mut files, options).await?;
        Ok(files)
    }

    fn collect_files_recursive<'a>(
        base_dir: &'a Path,
        current_dir: &'a Path,
        files: &'a mut HashMap<PathBuf, std::fs::Metadata>,
        options: &'a SyncOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            let mut entries = tokio::fs::read_dir(current_dir).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // Check exclusion patterns
                if options.exclude_patterns.iter().any(|pattern| file_name.contains(pattern)) {
                    continue;
                }

                let metadata = entry.metadata().await?;
                
                if metadata.is_file() {
                    let rel_path = path.strip_prefix(base_dir)?;
                    files.insert(rel_path.to_path_buf(), metadata);
                } else if metadata.is_dir() && options.recursive {
                    collect_files_recursive(base_dir, &path, files, options).await?;
                }
            }
            
            Ok(())
        })
    }

    async fn generate_content_diff(source_file: &Path, target_file: &Path) -> Result<String> {
        let source_content = read_file_to_string_async(source_file).await?;
        let target_content = read_file_to_string_async(target_file).await?;
        
        if source_content == target_content {
            return Ok(String::new());
        }
        
        // Generate a simple unified diff
        Ok(generate_unified_diff(&target_content, &source_content, source_file.to_string_lossy().as_ref()))
    }

    fn format_timestamp(system_time: &std::time::SystemTime) -> Result<String> {
        let datetime: chrono::DateTime<chrono::Utc> = (*system_time).into();
        Ok(datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string())
    }

    pub async fn calculate_file_similarity(file1: &Path, file2: &Path) -> Result<f64> {
        let content1 = read_file_to_string_async(file1).await?;
        let content2 = read_file_to_string_async(file2).await?;
        
        // Simple similarity calculation based on lines
        let lines1: Vec<&str> = content1.lines().collect();
        let lines2: Vec<&str> = content2.lines().collect();
        
        if lines1.is_empty() && lines2.is_empty() {
            return Ok(1.0);
        }
        
        if lines1.is_empty() || lines2.is_empty() {
            return Ok(0.0);
        }
        
        let mut common_lines = 0;
        let max_lines = lines1.len().max(lines2.len());
        
        for line1 in &lines1 {
            if lines2.contains(line1) {
                common_lines += 1;
            }
        }
        
        Ok(common_lines as f64 / max_lines as f64)
    }

    pub async fn find_duplicate_files(dir: &Path, recursive: bool) -> Result<Vec<Vec<PathBuf>>> {
        let mut files_by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        let mut files_by_hash: HashMap<String, Vec<PathBuf>> = HashMap::new();
        
        collect_files_by_size(dir, &mut files_by_size, recursive).await?;
        
        // Only hash files that have the same size
        for (size, paths) in files_by_size {
            if paths.len() > 1 && size > 0 {
                for path in paths {
                    let hash = calculate_file_hash(&path).await?;
                    files_by_hash.entry(hash).or_insert_with(Vec::new).push(path);
                }
            }
        }
        
        // Return groups of duplicate files
        Ok(files_by_hash
            .into_values()
            .filter(|group| group.len() > 1)
            .collect())
    }

    fn collect_files_by_size<'a>(
        dir: &'a Path,
        files_by_size: &'a mut HashMap<u64, Vec<PathBuf>>,
        recursive: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
        let mut entries = tokio::fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            
            if metadata.is_file() {
                let size = metadata.len();
                files_by_size.entry(size).or_insert_with(Vec::new).push(path);
            } else if metadata.is_dir() && recursive {
                collect_files_by_size(&path, files_by_size, recursive).await?;
            }
        }
        
        Ok(())
        })
    }

    async fn calculate_file_hash(path: &Path) -> Result<String> {
        let content = tokio::fs::read(path).await?;
        let digest = md5::compute(&content);
        Ok(format!("{:x}", digest))
    }
}

pub mod search {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use regex::Regex;

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct SearchResult {
        pub file_path: PathBuf,
        pub matches: Vec<SearchMatch>,
        pub total_matches: usize,
        pub file_size: u64,
        pub last_modified: String,
        pub file_type: String,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct SearchMatch {
        pub line_number: usize,
        pub column: usize,
        pub line_content: String,
        pub match_text: String,
        pub context_before: Vec<String>,
        pub context_after: Vec<String>,
        pub match_type: MatchType,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub enum MatchType {
        Exact,
        Regex,
        Fuzzy,
        Semantic,
        FunctionName,
        ClassName,
        Variable,
        Comment,
        Import,
    }

    #[derive(Debug, Clone)]
    pub struct SearchOptions {
        pub pattern: String,
        pub case_sensitive: bool,
        pub whole_word: bool,
        pub regex: bool,
        pub fuzzy: bool,
        pub semantic: bool,
        pub include_comments: bool,
        pub include_strings: bool,
        pub context_lines: usize,
        pub max_matches_per_file: Option<usize>,
        pub file_types: Vec<String>,
        pub exclude_patterns: Vec<String>,
        pub min_file_size: Option<u64>,
        pub max_file_size: Option<u64>,
        pub modified_after: Option<chrono::DateTime<chrono::Utc>>,
        pub modified_before: Option<chrono::DateTime<chrono::Utc>>,
    }

    impl Default for SearchOptions {
        fn default() -> Self {
            SearchOptions {
                pattern: String::new(),
                case_sensitive: false,
                whole_word: false,
                regex: false,
                fuzzy: false,
                semantic: false,
                include_comments: true,
                include_strings: true,
                context_lines: 2,
                max_matches_per_file: Some(100),
                file_types: vec!["rs", "js", "ts", "py", "java", "cpp", "c", "h", "md", "txt"].iter().map(|s| s.to_string()).collect(),
                exclude_patterns: vec![".git".to_string(), "node_modules".to_string(), "target".to_string(), ".vscode".to_string()],
                min_file_size: None,
                max_file_size: Some(10 * 1024 * 1024), // 10MB
                modified_after: None,
                modified_before: None,
            }
        }
    }

    pub async fn search_files(
        search_dir: &Path,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let files = collect_search_files(search_dir, options).await?;
        let mut results = Vec::new();

        for file_path in files {
            if let Ok(search_result) = search_file(&file_path, options).await {
                if !search_result.matches.is_empty() {
                    results.push(search_result);
                }
            }
        }

        // Sort by relevance (number of matches, file type priority)
        results.sort_by(|a, b| {
            let score_a = calculate_relevance_score(a, options);
            let score_b = calculate_relevance_score(b, options);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    pub async fn search_file(
        file_path: &Path,
        options: &SearchOptions,
    ) -> Result<SearchResult> {
        let content = read_file_to_string_async(file_path).await?;
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_type = detect_file_type_from_extension(file_path);
        
        let lines: Vec<&str> = content.lines().collect();
        let mut matches = Vec::new();
        let mut match_count = 0;

        // Create regex if needed
        let regex = if options.regex {
            Some(create_regex(&options.pattern, options.case_sensitive)?)
        } else {
            None
        };

        for (line_idx, line) in lines.iter().enumerate() {
            let line_number = line_idx + 1;
            
            // Skip if we've hit the max matches limit
            if let Some(max) = options.max_matches_per_file {
                if match_count >= max {
                    break;
                }
            }

            let line_matches = find_matches_in_line(
                line,
                line_number,
                options,
                &regex,
                &lines,
                line_idx,
            )?;

            match_count += line_matches.len();
            matches.extend(line_matches);
        }

        // Add semantic matches if enabled
        if options.semantic {
            let semantic_matches = find_semantic_matches(&content, file_path, options).await?;
            matches.extend(semantic_matches);
        }

        let last_modified = format_file_timestamp(&metadata.modified()?)?;

        Ok(SearchResult {
            file_path: file_path.to_path_buf(),
            matches,
            total_matches: match_count,
            file_size: metadata.len(),
            last_modified,
            file_type,
        })
    }

    fn find_matches_in_line(
        line: &str,
        line_number: usize,
        options: &SearchOptions,
        regex: &Option<Regex>,
        all_lines: &[&str],
        line_idx: usize,
    ) -> Result<Vec<SearchMatch>> {
        let mut matches = Vec::new();

        if options.regex {
            if let Some(re) = regex {
                for mat in re.find_iter(line) {
                    matches.push(create_search_match(
                        line,
                        line_number,
                        mat.start(),
                        mat.as_str(),
                        MatchType::Regex,
                        all_lines,
                        line_idx,
                        options.context_lines,
                    ));
                }
            }
        } else if options.fuzzy {
            // Simple fuzzy matching implementation
            if fuzzy_match(&options.pattern, line, options.case_sensitive) {
                matches.push(create_search_match(
                    line,
                    line_number,
                    0,
                    &options.pattern,
                    MatchType::Fuzzy,
                    all_lines,
                    line_idx,
                    options.context_lines,
                ));
            }
        } else {
            // Exact string matching
            let search_line = if options.case_sensitive { line.to_string() } else { line.to_lowercase() };
            let search_pattern = if options.case_sensitive { options.pattern.clone() } else { options.pattern.to_lowercase() };
            
            let mut start_pos = 0;
            while let Some(pos) = search_line[start_pos..].find(&search_pattern) {
                let actual_pos = start_pos + pos;
                
                // Check whole word constraint
                if options.whole_word && !is_whole_word_match(line, actual_pos, &options.pattern) {
                    start_pos = actual_pos + 1;
                    continue;
                }
                
                matches.push(create_search_match(
                    line,
                    line_number,
                    actual_pos,
                    &options.pattern,
                    MatchType::Exact,
                    all_lines,
                    line_idx,
                    options.context_lines,
                ));
                
                start_pos = actual_pos + options.pattern.len();
            }
        }

        Ok(matches)
    }

    async fn find_semantic_matches(
        content: &str,
        file_path: &Path,
        options: &SearchOptions,
    ) -> Result<Vec<SearchMatch>> {
        let mut matches = Vec::new();
        
        // Analyze code structure for semantic matches
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            match extension {
                "rs" => matches.extend(find_rust_semantic_matches(content, options)?),
                "js" | "ts" | "jsx" | "tsx" => matches.extend(find_javascript_semantic_matches(content, options)?),
                "py" => matches.extend(find_python_semantic_matches(content, options)?),
                "java" => matches.extend(find_java_semantic_matches(content, options)?),
                _ => {}
            }
        }
        
        Ok(matches)
    }

    fn find_rust_semantic_matches(content: &str, options: &SearchOptions) -> Result<Vec<SearchMatch>> {
        let mut matches = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        // Function definitions
        let func_re = Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)")?;
        for caps in func_re.captures_iter(content) {
            let func_name = caps.get(1).unwrap().as_str();
            if pattern_matches(func_name, &options.pattern, options.case_sensitive, options.fuzzy) {
                let line_num = content[..caps.get(0).unwrap().start()].lines().count() + 1;
                if let Some(line) = lines.get(line_num - 1) {
                    matches.push(create_search_match(
                        line,
                        line_num,
                        0,
                        func_name,
                        MatchType::FunctionName,
                        &lines,
                        line_num - 1,
                        options.context_lines,
                    ));
                }
            }
        }
        
        // Struct definitions
        let struct_re = Regex::new(r"(?m)^(?:pub\s+)?struct\s+(\w+)")?;
        for caps in struct_re.captures_iter(content) {
            let struct_name = caps.get(1).unwrap().as_str();
            if pattern_matches(struct_name, &options.pattern, options.case_sensitive, options.fuzzy) {
                let line_num = content[..caps.get(0).unwrap().start()].lines().count() + 1;
                if let Some(line) = lines.get(line_num - 1) {
                    matches.push(create_search_match(
                        line,
                        line_num,
                        0,
                        struct_name,
                        MatchType::ClassName,
                        &lines,
                        line_num - 1,
                        options.context_lines,
                    ));
                }
            }
        }
        
        Ok(matches)
    }

    fn find_javascript_semantic_matches(content: &str, options: &SearchOptions) -> Result<Vec<SearchMatch>> {
        let mut matches = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        // Function definitions
        let func_re = Regex::new(r"(?m)^(?:export\s+)?(?:async\s+)?function\s+(\w+)|(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(")?;
        for caps in func_re.captures_iter(content) {
            let func_name = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();
            if pattern_matches(func_name, &options.pattern, options.case_sensitive, options.fuzzy) {
                let line_num = content[..caps.get(0).unwrap().start()].lines().count() + 1;
                if let Some(line) = lines.get(line_num - 1) {
                    matches.push(create_search_match(
                        line,
                        line_num,
                        0,
                        func_name,
                        MatchType::FunctionName,
                        &lines,
                        line_num - 1,
                        options.context_lines,
                    ));
                }
            }
        }
        
        // Class definitions
        let class_re = Regex::new(r"(?m)^(?:export\s+)?class\s+(\w+)")?;
        for caps in class_re.captures_iter(content) {
            let class_name = caps.get(1).unwrap().as_str();
            if pattern_matches(class_name, &options.pattern, options.case_sensitive, options.fuzzy) {
                let line_num = content[..caps.get(0).unwrap().start()].lines().count() + 1;
                if let Some(line) = lines.get(line_num - 1) {
                    matches.push(create_search_match(
                        line,
                        line_num,
                        0,
                        class_name,
                        MatchType::ClassName,
                        &lines,
                        line_num - 1,
                        options.context_lines,
                    ));
                }
            }
        }
        
        Ok(matches)
    }

    fn find_python_semantic_matches(content: &str, options: &SearchOptions) -> Result<Vec<SearchMatch>> {
        let mut matches = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        // Function definitions
        let func_re = Regex::new(r"(?m)^(?:async\s+)?def\s+(\w+)")?;
        for caps in func_re.captures_iter(content) {
            let func_name = caps.get(1).unwrap().as_str();
            if pattern_matches(func_name, &options.pattern, options.case_sensitive, options.fuzzy) {
                let line_num = content[..caps.get(0).unwrap().start()].lines().count() + 1;
                if let Some(line) = lines.get(line_num - 1) {
                    matches.push(create_search_match(
                        line,
                        line_num,
                        0,
                        func_name,
                        MatchType::FunctionName,
                        &lines,
                        line_num - 1,
                        options.context_lines,
                    ));
                }
            }
        }
        
        // Class definitions
        let class_re = Regex::new(r"(?m)^class\s+(\w+)")?;
        for caps in class_re.captures_iter(content) {
            let class_name = caps.get(1).unwrap().as_str();
            if pattern_matches(class_name, &options.pattern, options.case_sensitive, options.fuzzy) {
                let line_num = content[..caps.get(0).unwrap().start()].lines().count() + 1;
                if let Some(line) = lines.get(line_num - 1) {
                    matches.push(create_search_match(
                        line,
                        line_num,
                        0,
                        class_name,
                        MatchType::ClassName,
                        &lines,
                        line_num - 1,
                        options.context_lines,
                    ));
                }
            }
        }
        
        Ok(matches)
    }

    fn find_java_semantic_matches(content: &str, options: &SearchOptions) -> Result<Vec<SearchMatch>> {
        let mut matches = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        // Method definitions
        let method_re = Regex::new(r"(?m)^\s*(?:public|private|protected)?\s*(?:static)?\s*\w+\s+(\w+)\s*\(")?;
        for caps in method_re.captures_iter(content) {
            let method_name = caps.get(1).unwrap().as_str();
            if pattern_matches(method_name, &options.pattern, options.case_sensitive, options.fuzzy) {
                let line_num = content[..caps.get(0).unwrap().start()].lines().count() + 1;
                if let Some(line) = lines.get(line_num - 1) {
                    matches.push(create_search_match(
                        line,
                        line_num,
                        0,
                        method_name,
                        MatchType::FunctionName,
                        &lines,
                        line_num - 1,
                        options.context_lines,
                    ));
                }
            }
        }
        
        // Class definitions
        let class_re = Regex::new(r"(?m)^(?:public\s+)?(?:abstract\s+)?class\s+(\w+)")?;
        for caps in class_re.captures_iter(content) {
            let class_name = caps.get(1).unwrap().as_str();
            if pattern_matches(class_name, &options.pattern, options.case_sensitive, options.fuzzy) {
                let line_num = content[..caps.get(0).unwrap().start()].lines().count() + 1;
                if let Some(line) = lines.get(line_num - 1) {
                    matches.push(create_search_match(
                        line,
                        line_num,
                        0,
                        class_name,
                        MatchType::ClassName,
                        &lines,
                        line_num - 1,
                        options.context_lines,
                    ));
                }
            }
        }
        
        Ok(matches)
    }

    fn create_search_match(
        line: &str,
        line_number: usize,
        column: usize,
        match_text: &str,
        match_type: MatchType,
        all_lines: &[&str],
        line_idx: usize,
        context_lines: usize,
    ) -> SearchMatch {
        let context_before = extract_context_before(all_lines, line_idx, context_lines);
        let context_after = extract_context_after(all_lines, line_idx, context_lines);
        
        SearchMatch {
            line_number,
            column,
            line_content: line.to_string(),
            match_text: match_text.to_string(),
            context_before,
            context_after,
            match_type,
        }
    }

    fn extract_context_before(all_lines: &[&str], line_idx: usize, context_lines: usize) -> Vec<String> {
        let start = if line_idx >= context_lines { line_idx - context_lines } else { 0 };
        all_lines[start..line_idx]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn extract_context_after(all_lines: &[&str], line_idx: usize, context_lines: usize) -> Vec<String> {
        let end = std::cmp::min(line_idx + 1 + context_lines, all_lines.len());
        all_lines[line_idx + 1..end]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn pattern_matches(text: &str, pattern: &str, case_sensitive: bool, fuzzy: bool) -> bool {
        if fuzzy {
            fuzzy_match(pattern, text, case_sensitive)
        } else {
            let text_cmp = if case_sensitive { text.to_string() } else { text.to_lowercase() };
            let pattern_cmp = if case_sensitive { pattern.to_string() } else { pattern.to_lowercase() };
            text_cmp.contains(&pattern_cmp)
        }
    }

    fn fuzzy_match(pattern: &str, text: &str, case_sensitive: bool) -> bool {
        let pattern = if case_sensitive { pattern.to_string() } else { pattern.to_lowercase() };
        let text = if case_sensitive { text.to_string() } else { text.to_lowercase() };
        
        let mut pattern_chars = pattern.chars().peekable();
        let mut text_chars = text.chars();
        
        while let Some(pattern_char) = pattern_chars.next() {
            let mut found = false;
            while let Some(text_char) = text_chars.next() {
                if text_char == pattern_char {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
        
        true
    }

    fn is_whole_word_match(line: &str, pos: usize, pattern: &str) -> bool {
        let before_ok = pos == 0 || !line.chars().nth(pos - 1).unwrap_or(' ').is_alphanumeric();
        let after_pos = pos + pattern.len();
        let after_ok = after_pos >= line.len() || !line.chars().nth(after_pos).unwrap_or(' ').is_alphanumeric();
        
        before_ok && after_ok
    }

    fn create_regex(pattern: &str, case_sensitive: bool) -> Result<Regex> {
        let mut builder = regex::RegexBuilder::new(pattern);
        builder.case_insensitive(!case_sensitive);
        builder.build().map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))
    }

    async fn collect_search_files(
        search_dir: &Path,
        options: &SearchOptions,
    ) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        collect_search_files_recursive(search_dir, &mut files, options).await?;
        Ok(files)
    }

    fn collect_search_files_recursive<'a>(
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
        options: &'a SearchOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            let mut entries = tokio::fs::read_dir(dir).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // Check exclusion patterns
                if options.exclude_patterns.iter().any(|pattern| file_name.contains(pattern)) {
                    continue;
                }

                let metadata = entry.metadata().await?;
                
                if metadata.is_file() {
                    // Check file type
                    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                        if !options.file_types.is_empty() && !options.file_types.contains(&extension.to_string()) {
                            continue;
                        }
                    }
                    
                    // Check file size
                    let size = metadata.len();
                    if let Some(min_size) = options.min_file_size {
                        if size < min_size {
                            continue;
                        }
                    }
                    if let Some(max_size) = options.max_file_size {
                        if size > max_size {
                            continue;
                        }
                    }
                    
                    // Check modification time
                    if let Ok(modified) = metadata.modified() {
                        let modified_chrono: chrono::DateTime<chrono::Utc> = modified.into();
                        
                        if let Some(after) = options.modified_after {
                            if modified_chrono < after {
                                continue;
                            }
                        }
                        
                        if let Some(before) = options.modified_before {
                            if modified_chrono > before {
                                continue;
                            }
                        }
                    }
                    
                    files.push(path);
                } else if metadata.is_dir() {
                    collect_search_files_recursive(&path, files, options).await?;
                }
            }
            
            Ok(())
        })
    }

    fn detect_file_type_from_extension(file_path: &Path) -> String {
        file_path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn format_file_timestamp(system_time: &std::time::SystemTime) -> Result<String> {
        let datetime: chrono::DateTime<chrono::Utc> = (*system_time).into();
        Ok(datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string())
    }

    fn calculate_relevance_score(result: &SearchResult, _options: &SearchOptions) -> f64 {
        let mut score = result.total_matches as f64;
        
        // Boost score for exact matches
        let exact_matches = result.matches.iter()
            .filter(|m| matches!(m.match_type, MatchType::Exact))
            .count() as f64;
        score += exact_matches * 2.0;
        
        // Boost score for semantic matches
        let semantic_matches = result.matches.iter()
            .filter(|m| matches!(m.match_type, MatchType::FunctionName | MatchType::ClassName))
            .count() as f64;
        score += semantic_matches * 3.0;
        
        // Boost score for smaller files (more focused)
        if result.file_size < 10000 {
            score *= 1.2;
        }
        
        // Boost score for certain file types
        match result.file_type.as_str() {
            "rs" | "js" | "ts" | "py" => score *= 1.1,
            "md" | "txt" => score *= 0.9,
            _ => {}
        }
        
        score
    }

    pub async fn search_and_replace(
        search_dir: &Path,
        search_pattern: &str,
        replace_pattern: &str,
        options: &SearchOptions,
        dry_run: bool,
    ) -> Result<Vec<(PathBuf, usize)>> {
        let mut replaced_files = Vec::new();
        let files = collect_search_files(search_dir, options).await?;
        
        for file_path in files {
            let content = read_file_to_string_async(&file_path).await?;
            let new_content = if options.regex {
                let re = create_regex(search_pattern, options.case_sensitive)?;
                re.replace_all(&content, replace_pattern).to_string()
            } else {
                if options.case_sensitive {
                    content.replace(search_pattern, replace_pattern)
                } else {
                    // Case-insensitive replacement is more complex
                    let re = create_regex(&regex::escape(search_pattern), options.case_sensitive)?;
                    re.replace_all(&content, replace_pattern).to_string()
                }
            };
            
            if new_content != content {
                let replacements = count_replacements(&content, &new_content, search_pattern);
                
                if !dry_run {
                    tokio::fs::write(&file_path, new_content).await?;
                }
                
                replaced_files.push((file_path, replacements));
            }
        }
        
        Ok(replaced_files)
    }

    fn count_replacements(original: &str, new: &str, pattern: &str) -> usize {
        let original_count = original.matches(pattern).count();
        let new_count = new.matches(pattern).count();
        original_count.saturating_sub(new_count)
    }
}

// Security scanning module
pub mod security {
    use super::*;
    use regex::Regex;
    use std::collections::{HashMap, HashSet};
    use chrono::{DateTime, Utc};
    use tokio::fs::{read_to_string, read_dir};

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct SecurityReport {
        pub file_path: PathBuf,
        pub file_size: u64,
        pub last_modified: DateTime<Utc>,
        pub scan_timestamp: DateTime<Utc>,
        pub issues: Vec<SecurityIssue>,
        pub risk_score: u32,
        pub recommendations: Vec<String>,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct SecurityIssue {
        pub issue_type: IssueType,
        pub severity: Severity,
        pub line_number: usize,
        pub line_content: String,
        pub description: String,
        pub recommendation: String,
        pub cwe_id: Option<String>,
        pub owasp_category: Option<String>,
    }

    #[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, Hash)]
    pub enum IssueType {
        HardcodedCredentials,
        SqlInjection,
        CrossSiteScripting,
        InsecureRandomness,
        WeakCryptography,
        PathTraversal,
        CommandInjection,
        SensitiveDataExposure,
        InsecureDeserialization,
        VulnerableDependency,
        WeakAuthentication,
        InsecureStorage,
        InsufficientLogging,
        ExcessivePermissions,
        UnsafeCodePattern,
        ConfigurationIssue,
    }

    #[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Severity {
        Info,
        Low,
        Medium,
        High,
        Critical,
    }

    #[derive(Debug, Clone)]
    pub struct SecurityOptions {
        pub include_info: bool,
        pub check_credentials: bool,
        pub check_injection: bool,
        pub check_crypto: bool,
        pub check_paths: bool,
        pub check_dependencies: bool,
        pub check_configuration: bool,
        pub file_types: Vec<String>,
        pub exclude_patterns: Vec<String>,
    }

    impl Default for SecurityOptions {
        fn default() -> Self {
            Self {
                include_info: false,
                check_credentials: true,
                check_injection: true,
                check_crypto: true,
                check_paths: true,
                check_dependencies: true,
                check_configuration: true,
                file_types: vec![
                    "rs".to_string(), "js".to_string(), "ts".to_string(), "py".to_string(),
                    "java".to_string(), "php".to_string(), "go".to_string(), "cpp".to_string(),
                    "c".to_string(), "cs".to_string(), "rb".to_string(), "sql".to_string(),
                    "json".to_string(), "yaml".to_string(), "yml".to_string(), "toml".to_string(),
                    "ini".to_string(), "conf".to_string(), "env".to_string()
                ],
                exclude_patterns: vec![
                    "test".to_string(), "spec".to_string(), "mock".to_string(),
                    "node_modules".to_string(), "target".to_string(), ".git".to_string(),
                    "vendor".to_string(), "dist".to_string(), "build".to_string()
                ],
            }
        }
    }

    pub async fn scan_files_security(path: &Path, options: &SecurityOptions) -> Result<Vec<SecurityReport>> {
        let mut reports = Vec::new();
        let files = collect_security_files(path, options).await?;

        for file_path in files {
            if let Ok(report) = scan_file_security(&file_path, options).await {
                reports.push(report);
            }
        }

        // Sort by risk score (highest first)
        reports.sort_by(|a, b| b.risk_score.cmp(&a.risk_score));

        Ok(reports)
    }

    pub async fn scan_file_security(file_path: &Path, options: &SecurityOptions) -> Result<SecurityReport> {
        let content = read_to_string(file_path).await?;
        let metadata = std::fs::metadata(file_path)?;
        let last_modified = DateTime::from(metadata.modified()?);
        
        let mut issues = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Check for hardcoded credentials
        if options.check_credentials {
            issues.extend(check_hardcoded_credentials(&lines));
        }

        // Check for injection vulnerabilities
        if options.check_injection {
            issues.extend(check_injection_vulnerabilities(&lines, file_path));
        }

        // Check for cryptographic issues
        if options.check_crypto {
            issues.extend(check_crypto_issues(&lines, file_path));
        }

        // Check for path traversal vulnerabilities
        if options.check_paths {
            issues.extend(check_path_traversal(&lines));
        }

        // Check for dependency vulnerabilities
        if options.check_dependencies {
            issues.extend(check_dependency_vulnerabilities(&content, file_path));
        }

        // Check for configuration issues
        if options.check_configuration {
            issues.extend(check_configuration_issues(&lines, file_path));
        }

        // Filter by severity if needed
        if !options.include_info {
            issues.retain(|issue| issue.severity != Severity::Info);
        }

        // Calculate risk score
        let risk_score = calculate_risk_score(&issues);

        // Generate recommendations
        let recommendations = generate_recommendations(&issues, file_path);

        Ok(SecurityReport {
            file_path: file_path.to_path_buf(),
            file_size: metadata.len(),
            last_modified,
            scan_timestamp: Utc::now(),
            issues,
            risk_score,
            recommendations,
        })
    }

    fn check_hardcoded_credentials(lines: &[&str]) -> Vec<SecurityIssue> {
        let mut issues = Vec::new();
        
        let patterns = vec![
            (r#"(?i)(password|pwd|pass)\s*[=:]\s*['"]([^'"]{8,})['"]"#, "Hardcoded password detected"),
            (r#"(?i)(api_key|apikey|key)\s*[=:]\s*['"]([^'"]{16,})['"]"#, "Hardcoded API key detected"),
            (r#"(?i)(secret|token)\s*[=:]\s*['"]([^'"]{16,})['"]"#, "Hardcoded secret/token detected"),
            (r#"(?i)(private_key|privatekey)\s*[=:]\s*['"]([^'"]{32,})['"]"#, "Hardcoded private key detected"),
            (r#"(?i)(database_url|db_url|connection_string)\s*[=:]\s*['"]([^'"]+://[^'"]+)['"]"#, "Hardcoded database connection string"),
            (r#"(?i)(access_token|accesstoken)\s*[=:]\s*['"]([^'"]{16,})['"]"#, "Hardcoded access token detected"),
        ];

        for (line_num, line) in lines.iter().enumerate() {
            for (pattern, description) in &patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(line) {
                        issues.push(SecurityIssue {
                            issue_type: IssueType::HardcodedCredentials,
                            severity: Severity::High,
                            line_number: line_num + 1,
                            line_content: line.to_string(),
                            description: description.to_string(),
                            recommendation: "Use environment variables or secure credential management systems".to_string(),
                            cwe_id: Some("CWE-798".to_string()),
                            owasp_category: Some("A07:2021 – Identification and Authentication Failures".to_string()),
                        });
                    }
                }
            }
        }

        issues
    }

    fn check_injection_vulnerabilities(lines: &[&str], file_path: &Path) -> Vec<SecurityIssue> {
        let mut issues = Vec::new();
        let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");

        // SQL Injection patterns
        let sql_patterns = vec![
            (r#"(?i)(query|execute)\s*\(\s*['"]\s*SELECT.*\+.*['"]\s*\)"#, "Potential SQL injection via string concatenation"),
            (r#"(?i)(query|execute)\s*\(\s*.*\+.*WHERE.*\+.*\)"#, "Potential SQL injection in WHERE clause"),
            (r#"(?i)\.format\s*\(\s*['"]\s*SELECT.*\{.*\}.*['"]\s*\)"#, "Potential SQL injection via string formatting"),
        ];

        // Command Injection patterns
        let cmd_patterns = vec![
            (r"(?i)(system|exec|eval|shell_exec|passthru)\s*\(\s*.*\$.*\)", "Potential command injection"),
            (r"(?i)(Runtime\.getRuntime\(\)\.exec|ProcessBuilder)\s*\(\s*.*\+.*\)", "Potential command injection in Java"),
            (r"(?i)(os\.system|subprocess\.call|subprocess\.run)\s*\(\s*.*\+.*\)", "Potential command injection in Python"),
        ];

        // XSS patterns
        let xss_patterns = vec![
            (r"(?i)innerHTML\s*=\s*.*\+", "Potential XSS via innerHTML"),
            (r"(?i)document\.write\s*\(\s*.*\+", "Potential XSS via document.write"),
            (r"(?i)eval\s*\(\s*.*\+", "Potential XSS/code injection via eval"),
        ];

        for (line_num, line) in lines.iter().enumerate() {
            // Check SQL injection
            for (pattern, description) in &sql_patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(line) {
                        issues.push(SecurityIssue {
                            issue_type: IssueType::SqlInjection,
                            severity: Severity::High,
                            line_number: line_num + 1,
                            line_content: line.to_string(),
                            description: description.to_string(),
                            recommendation: "Use parameterized queries or prepared statements".to_string(),
                            cwe_id: Some("CWE-89".to_string()),
                            owasp_category: Some("A03:2021 – Injection".to_string()),
                        });
                    }
                }
            }

            // Check command injection
            for (pattern, description) in &cmd_patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(line) {
                        issues.push(SecurityIssue {
                            issue_type: IssueType::CommandInjection,
                            severity: Severity::High,
                            line_number: line_num + 1,
                            line_content: line.to_string(),
                            description: description.to_string(),
                            recommendation: "Validate and sanitize input, use safe command execution methods".to_string(),
                            cwe_id: Some("CWE-78".to_string()),
                            owasp_category: Some("A03:2021 – Injection".to_string()),
                        });
                    }
                }
            }

            // Check XSS for web-related files
            if matches!(extension, "js" | "ts" | "html" | "php" | "jsp") {
                for (pattern, description) in &xss_patterns {
                    if let Ok(re) = Regex::new(pattern) {
                        if re.is_match(line) {
                            issues.push(SecurityIssue {
                                issue_type: IssueType::CrossSiteScripting,
                                severity: Severity::Medium,
                                line_number: line_num + 1,
                                line_content: line.to_string(),
                                description: description.to_string(),
                                recommendation: "Sanitize and validate user input, use safe DOM manipulation".to_string(),
                                cwe_id: Some("CWE-79".to_string()),
                                owasp_category: Some("A03:2021 – Injection".to_string()),
                            });
                        }
                    }
                }
            }
        }

        issues
    }

    fn check_crypto_issues(lines: &[&str], _file_path: &Path) -> Vec<SecurityIssue> {
        let mut issues = Vec::new();

        let weak_crypto_patterns = vec![
            (r"(?i)(MD5|SHA1|DES|RC4)", "Weak cryptographic algorithm detected", Severity::Medium),
            (r"(?i)Math\.random\(\)", "Insecure random number generation", Severity::Low),
            (r"(?i)Random\(\)", "Potentially insecure random number generation", Severity::Low),
            (r"(?i)(ECB|Electronic Codebook)", "Insecure encryption mode (ECB)", Severity::High),
            (r"(?i)hardcoded.*(?:key|iv|salt)", "Hardcoded cryptographic key/IV/salt", Severity::High),
            (r"(?i)(ssl.*verify.*false|tls.*verify.*false)", "SSL/TLS verification disabled", Severity::High),
        ];

        for (line_num, line) in lines.iter().enumerate() {
            for (pattern, description, severity) in &weak_crypto_patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(line) {
                        issues.push(SecurityIssue {
                            issue_type: IssueType::WeakCryptography,
                            severity: severity.clone(),
                            line_number: line_num + 1,
                            line_content: line.to_string(),
                            description: description.to_string(),
                            recommendation: "Use strong cryptographic algorithms (AES, SHA-256+, secure random generators)".to_string(),
                            cwe_id: Some("CWE-327".to_string()),
                            owasp_category: Some("A02:2021 – Cryptographic Failures".to_string()),
                        });
                    }
                }
            }
        }

        issues
    }

    fn check_path_traversal(lines: &[&str]) -> Vec<SecurityIssue> {
        let mut issues = Vec::new();

        let path_traversal_patterns = vec![
            (r"\.\./", "Potential path traversal with ../"),
            (r"\\\.\\\.\\", "Potential path traversal with ..\\"),
            (r"(?i)filename.*\.\./", "User-controlled filename with path traversal"),
            (r"(?i)path.*\.\./", "User-controlled path with path traversal"),
        ];

        for (line_num, line) in lines.iter().enumerate() {
            for (pattern, description) in &path_traversal_patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(line) {
                        issues.push(SecurityIssue {
                            issue_type: IssueType::PathTraversal,
                            severity: Severity::Medium,
                            line_number: line_num + 1,
                            line_content: line.to_string(),
                            description: description.to_string(),
                            recommendation: "Validate and sanitize file paths, use allowlists".to_string(),
                            cwe_id: Some("CWE-22".to_string()),
                            owasp_category: Some("A01:2021 – Broken Access Control".to_string()),
                        });
                    }
                }
            }
        }

        issues
    }

    fn check_dependency_vulnerabilities(content: &str, file_path: &Path) -> Vec<SecurityIssue> {
        let mut issues = Vec::new();
        let filename = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Known vulnerable packages (simplified - in real implementation would use vulnerability databases)
        let vulnerable_packages = vec![
            ("lodash", "4.17.20", "Prototype pollution vulnerability"),
            ("jquery", "3.4.1", "XSS vulnerability in jQuery"),
            ("express", "4.17.0", "Potential DoS vulnerability"),
            ("serialize-javascript", "3.1.0", "XSS vulnerability"),
        ];

        if matches!(filename, "package.json" | "package-lock.json" | "yarn.lock") {
            for (pkg, version, desc) in &vulnerable_packages {
                let pattern = format!(r#""{}".*"{}""#, pkg, version);
                if let Ok(re) = Regex::new(&pattern) {
                    if re.is_match(content) {
                        issues.push(SecurityIssue {
                            issue_type: IssueType::VulnerableDependency,
                            severity: Severity::Medium,
                            line_number: 1, // Simplified
                            line_content: format!("Vulnerable dependency: {} v{}", pkg, version),
                            description: desc.to_string(),
                            recommendation: "Update to latest secure version".to_string(),
                            cwe_id: Some("CWE-1104".to_string()),
                            owasp_category: Some("A06:2021 – Vulnerable and Outdated Components".to_string()),
                        });
                    }
                }
            }
        }

        issues
    }

    fn check_configuration_issues(lines: &[&str], file_path: &Path) -> Vec<SecurityIssue> {
        let mut issues = Vec::new();
        let filename = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        let config_patterns = vec![
            (r"(?i)(debug|verbose)\s*[=:]\s*(true|1|on)", "Debug mode enabled in production", Severity::Low),
            (r"(?i)(cors.*origin.*\*|access-control-allow-origin.*\*)", "Overly permissive CORS policy", Severity::Medium),
            (r"(?i)(ssl.*false|tls.*false|https.*false)", "SSL/TLS disabled", Severity::High),
            (r"(?i)(auth.*disabled|authentication.*false)", "Authentication disabled", Severity::Critical),
            (r"(?i)(admin.*true|root.*true)", "Administrative privileges enabled", Severity::Medium),
        ];

        // Check configuration files
        if matches!(filename, "config.json" | "app.config" | ".env" | "settings.py" | "application.yml") {
            for (line_num, line) in lines.iter().enumerate() {
                for (pattern, description, severity) in &config_patterns {
                    if let Ok(re) = Regex::new(pattern) {
                        if re.is_match(line) {
                            issues.push(SecurityIssue {
                                issue_type: IssueType::ConfigurationIssue,
                                severity: severity.clone(),
                                line_number: line_num + 1,
                                line_content: line.to_string(),
                                description: description.to_string(),
                                recommendation: "Review and harden configuration settings".to_string(),
                                cwe_id: Some("CWE-16".to_string()),
                                owasp_category: Some("A05:2021 – Security Misconfiguration".to_string()),
                            });
                        }
                    }
                }
            }
        }

        issues
    }

    fn calculate_risk_score(issues: &[SecurityIssue]) -> u32 {
        issues.iter().map(|issue| {
            match issue.severity {
                Severity::Critical => 100,
                Severity::High => 50,
                Severity::Medium => 25,
                Severity::Low => 10,
                Severity::Info => 1,
            }
        }).sum()
    }

    fn generate_recommendations(issues: &[SecurityIssue], file_path: &Path) -> Vec<String> {
        let mut recommendations = Vec::new();
        let issue_types: HashSet<_> = issues.iter().map(|i| &i.issue_type).collect();

        if issue_types.contains(&IssueType::HardcodedCredentials) {
            recommendations.push("Implement secure credential management using environment variables or dedicated secret stores".to_string());
        }

        if issue_types.contains(&IssueType::SqlInjection) {
            recommendations.push("Use parameterized queries and ORM frameworks to prevent SQL injection".to_string());
        }

        if issue_types.contains(&IssueType::WeakCryptography) {
            recommendations.push("Upgrade to strong cryptographic algorithms (AES-256, SHA-256+, secure random generators)".to_string());
        }

        if issue_types.contains(&IssueType::CrossSiteScripting) {
            recommendations.push("Implement input validation and output encoding to prevent XSS attacks".to_string());
        }

        if issue_types.contains(&IssueType::PathTraversal) {
            recommendations.push("Validate file paths and use allowlists to prevent directory traversal attacks".to_string());
        }

        if issue_types.contains(&IssueType::VulnerableDependency) {
            recommendations.push("Regularly update dependencies and use vulnerability scanning tools".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Continue following secure coding practices and regular security reviews".to_string());
        }

        recommendations
    }

    async fn collect_security_files(path: &Path, options: &SecurityOptions) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        collect_security_files_recursive(path, &mut files, options).await?;
        Ok(files)
    }

    fn collect_security_files_recursive<'a>(
        path: &'a Path,
        files: &'a mut Vec<PathBuf>,
        options: &'a SecurityOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            let mut entries = read_dir(path).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                
                if entry_path.is_dir() {
                    // Skip excluded directories
                    if let Some(dir_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                        if options.exclude_patterns.iter().any(|pattern| dir_name.contains(pattern)) {
                            continue;
                        }
                    }
                    
                    collect_security_files_recursive(&entry_path, files, options).await?;
                } else if entry_path.is_file() {
                    // Check if file type is included
                    if let Some(extension) = entry_path.extension().and_then(|s| s.to_str()) {
                        if options.file_types.contains(&extension.to_string()) {
                            files.push(entry_path);
                        }
                    }
                }
            }
            
            Ok(())
        })
    }
}
