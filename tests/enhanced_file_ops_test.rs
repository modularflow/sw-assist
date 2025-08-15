// Enhanced File Operations Tests
// These tests verify the new file operation capabilities: git-awareness, checkpointing, and batch operations

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;
use std::path::Path;

/// Test 1: Git-aware file listing
#[test]
fn test_git_aware_file_listing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = temp_dir.path();
    
    // Create a mock git repository structure
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::create_dir_all(base_path.join("target/debug")).unwrap();
    fs::create_dir_all(base_path.join("node_modules/package")).unwrap();
    
    fs::write(base_path.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(base_path.join("target/debug/app"), "binary").unwrap();
    fs::write(base_path.join("node_modules/package/index.js"), "module.exports = {}").unwrap();
    fs::write(base_path.join("README.md"), "# Project").unwrap();
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "files", "list",
        "--path", base_path.to_str().unwrap(),
        "--recursive",

        "--include-ext", "rs,md"
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Should include source files and README
    assert!(output_str.contains("main.rs"));
    assert!(output_str.contains("README.md"));
    
    // Should exclude git-ignored directories
    assert!(!output_str.contains("target"));
    assert!(!output_str.contains("node_modules"));
}

/// Test 2: File pattern matching
#[test]
fn test_file_pattern_matching() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = temp_dir.path();
    
    // Create test files
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::create_dir_all(base_path.join("tests")).unwrap();
    
    fs::write(base_path.join("src/lib.rs"), "// Library code").unwrap();
    fs::write(base_path.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(base_path.join("tests/integration.rs"), "// Tests").unwrap();
    fs::write(base_path.join("README.md"), "# Documentation").unwrap();
    fs::write(base_path.join("package.json"), "{}").unwrap();
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "files", "list",
        "--path", base_path.to_str().unwrap(),
        "--recursive",
        "--include-ext", "rs",
        "--exclude-pattern", "test"
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Should include .rs files
    assert!(output_str.contains("lib.rs"));
    assert!(output_str.contains("main.rs"));
    
    // Should exclude files with "test" pattern
    assert!(!output_str.contains("integration.rs"));
    
    // Should exclude non-.rs files
    assert!(!output_str.contains("README.md"));
    assert!(!output_str.contains("package.json"));
}

/// Test 3: Git root detection
#[test]
fn test_git_root_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = temp_dir.path();
    
    // Create nested directory structure with .git at root
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src/submodule")).unwrap();
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "files", "git-root",
        "--path", base_path.join("src/submodule").to_str().unwrap()
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains(&base_path.display().to_string()));
}

/// Test 4: Checkpoint creation and listing
#[test]
fn test_checkpoint_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = temp_dir.path();
    
    // Create test files
    let file1 = base_path.join("test1.txt");
    let file2 = base_path.join("test2.txt");
    fs::write(&file1, "Original content 1").unwrap();
    fs::write(&file2, "Original content 2").unwrap();
    
    // Test checkpoint creation
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.current_dir(base_path)
        .args(&[
            "checkpoint", "create",
            "--description", "Test checkpoint",
            "--files", file1.to_str().unwrap(),
            "--files", file2.to_str().unwrap()
        ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Checkpoint created"));
    assert!(output_str.contains("Files saved: 2"));
    
    // Test checkpoint listing
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.current_dir(base_path)
        .args(&["checkpoint", "list"]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Test checkpoint"));
}

/// Test 5: Batch code generation with checkpointing
#[test]
fn test_batch_generation_with_checkpoint() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = temp_dir.path();
    
    // Create target files for generation
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::write(base_path.join("src/lib.rs"), "// Empty library").unwrap();
    fs::write(base_path.join("src/utils.rs"), "// Empty utils").unwrap();
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.current_dir(base_path)
        .args(&[
            "batch", "generate",
            "--provider", "mock",
            "--instruction", "Add comprehensive documentation and examples",
            "--path", "src",
            "--include-ext", "rs",
            "--checkpoint"
        ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Processed"));
    assert!(output_str.contains("Checkpoint created"));
    
    // Verify files were modified
    let lib_content = fs::read_to_string(base_path.join("src/lib.rs")).unwrap();
    assert!(lib_content.contains("Mock generated code"));
}

/// Test 6: Batch transformation with safety
#[test]
fn test_batch_transformation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = temp_dir.path();
    
    // Create target files
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::write(base_path.join("src/old_style.js"), "var x = 1; function test() { return x; }").unwrap();
    fs::write(base_path.join("src/legacy.js"), "var data = []; for(var i=0; i<10; i++) data.push(i);").unwrap();
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.current_dir(base_path)
        .args(&[
            "batch", "transform",
            "--provider", "mock",
            "--instruction", "Convert to modern ES6+ syntax",
            "--path", "src",
            "--include-ext", "js",
            "--checkpoint"
        ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Generated diffs"));
    assert!(output_str.contains("Checkpoint created"));
    
    // Should suggest applying diffs
    assert!(output_str.contains("sw diff apply"));
}

/// Test 7: Comprehensive file operations workflow
#[test]
fn test_comprehensive_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = temp_dir.path();
    
    // Create a realistic project structure
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src/components")).unwrap();
    fs::create_dir_all(base_path.join("src/utils")).unwrap();
    fs::create_dir_all(base_path.join("tests")).unwrap();
    fs::create_dir_all(base_path.join("node_modules/package")).unwrap();
    
    // Source files
    fs::write(base_path.join("src/main.js"), "console.log('Hello world');").unwrap();
    fs::write(base_path.join("src/components/Button.js"), "export default function Button() {}").unwrap();
    fs::write(base_path.join("src/utils/helpers.js"), "export const helper = () => {};").unwrap();
    
    // Test files
    fs::write(base_path.join("tests/main.test.js"), "// Test file").unwrap();
    
    // Files that should be ignored
    fs::write(base_path.join("node_modules/package/index.js"), "module.exports = {};").unwrap();
    fs::write(base_path.join(".env"), "SECRET=value").unwrap();
    
    // Step 1: List all JavaScript files with git-awareness
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "files", "list",
        "--path", base_path.to_str().unwrap(),
        "--recursive",

        "--include-ext", "js"
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success());
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Should find source and test files
    assert!(output_str.contains("main.js"));
    assert!(output_str.contains("Button.js"));
    assert!(output_str.contains("helpers.js"));
    assert!(output_str.contains("main.test.js"));
    
    // Should exclude ignored files
    assert!(!output_str.contains("node_modules"));
    assert!(!output_str.contains(".env"));
    
    // Step 2: Create checkpoint before modifications
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.current_dir(base_path)
        .args(&[
            "checkpoint", "create",
            "--description", "Before adding JSDoc comments",
            "--files", "src/main.js",
            "--files", "src/components/Button.js"
        ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success());
    
    // Step 3: Verify we can list checkpoints
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.current_dir(base_path)
        .args(&["checkpoint", "list"]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success());
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Before adding JSDoc comments"));
}
