// Code Generation & Editing Capability Tests
// These tests verify the core code generation functionality described in SPEC.md

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Test 1: Basic Code Generation Test
/// From SPEC: "Create a REST API endpoint for user authentication using JWT tokens in Express.js"
#[test]
fn test_code_generation_rest_api() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_file = temp_dir.path().join("auth.js");
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "generate",
        "--provider", "mock",
        "--instruction", "Create a REST API endpoint for user authentication using JWT tokens in Express.js",
        "--file", output_file.to_str().unwrap()
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    
    // Should generate valid Express.js authentication code
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let generated_code = fs::read_to_string(&output_file).expect("Failed to read generated file");
    
    // Verify the generated code contains expected patterns
    assert!(generated_code.contains("express"));
    assert!(generated_code.contains("jwt") || generated_code.contains("JWT"));
    assert!(generated_code.contains("auth") || generated_code.contains("login"));
    assert!(generated_code.contains("router") || generated_code.contains("app."));
}

/// Test 2: Code Refactoring Test  
/// From SPEC: "Refactor this legacy JavaScript code to use modern ES6+ features and improve readability"
#[test]
fn test_code_refactoring() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_file = temp_dir.path().join("legacy.js");
    
    // Create legacy JavaScript code to refactor
    let legacy_code = r#"
function processUsers(users) {
    var result = [];
    for (var i = 0; i < users.length; i++) {
        var user = users[i];
        if (user.active == true) {
            var userData = {
                id: user.id,
                name: user.name,
                email: user.email
            };
            result.push(userData);
        }
    }
    return result;
}
"#;
    
    fs::write(&input_file, legacy_code).expect("Failed to write legacy code");
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "generate",
        "--provider", "mock",
        "--instruction", "Refactor this legacy JavaScript code to use modern ES6+ features and improve readability",
        "--file", input_file.to_str().unwrap()
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Should generate modernized code in the file
    let refactored_code = fs::read_to_string(&input_file).expect("Failed to read refactored file");
    assert!(refactored_code.contains("const") || refactored_code.contains("filter") || refactored_code.contains("map"));
}

/// Test 3: Test Generation Test
/// From SPEC: "Generate comprehensive unit tests for this user service class using Jest"
#[test] 
fn test_test_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let service_file = temp_dir.path().join("userService.js");
    let test_file = temp_dir.path().join("userService.test.js");
    
    // Create a simple user service
    let service_code = r#"
class UserService {
    constructor(database) {
        this.db = database;
    }
    
    async createUser(userData) {
        if (!userData.email) {
            throw new Error('Email is required');
        }
        return await this.db.users.create(userData);
    }
    
    async getUserById(id) {
        return await this.db.users.findById(id);
    }
    
    async updateUser(id, updates) {
        return await this.db.users.update(id, updates);
    }
    
    async deleteUser(id) {
        return await this.db.users.delete(id);
    }
}

module.exports = UserService;
"#;
    
    fs::write(&service_file, service_code).expect("Failed to write service code");
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "generate",
        "--provider", "mock",
        "--instruction", "Generate comprehensive unit tests for this user service class using Jest",
        "--file", test_file.to_str().unwrap()
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let generated_tests = fs::read_to_string(&test_file).expect("Failed to read test file");
    
    // Verify comprehensive test coverage
    assert!(generated_tests.contains("jest") || generated_tests.contains("describe"));
    assert!(generated_tests.contains("createUser"));
    assert!(generated_tests.contains("getUserById"));
    assert!(generated_tests.contains("updateUser"));
    assert!(generated_tests.contains("deleteUser"));
    assert!(generated_tests.contains("expect"));
}

/// Test 4: Documentation Generation Test
/// From SPEC: "Create a README.md file for this project with installation instructions and API docs"
#[test]
fn test_documentation_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let readme_file = temp_dir.path().join("README.md");
    
    // Create a simple package.json to give context
    let package_json = temp_dir.path().join("package.json");
    fs::write(&package_json, r#"
{
    "name": "user-api",
    "version": "1.0.0",
    "description": "User management API",
    "main": "index.js",
    "scripts": {
        "start": "node index.js",
        "test": "jest"
    },
    "dependencies": {
        "express": "^4.18.0",
        "jsonwebtoken": "^9.0.0"
    }
}
"#).expect("Failed to write package.json");
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "generate",
        "--provider", "mock",
        "--instruction", "Create a README.md file for this project with installation instructions and API docs",
        "--file", readme_file.to_str().unwrap()
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let readme_content = fs::read_to_string(&readme_file).expect("Failed to read README");
    
    // Verify README has proper structure and content
    assert!(readme_content.contains("# ") || readme_content.contains("## "));  // Headers
    assert!(readme_content.contains("install"));  // Installation instructions
    assert!(readme_content.contains("npm") || readme_content.contains("yarn"));  // Package manager
    assert!(readme_content.contains("API") || readme_content.contains("endpoint"));  // API docs
}

/// Test 5: File Modification Safety Test
/// Ensures that diff operations are safe and can be reviewed before application
#[test]
fn test_diff_safety_and_approval() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_file = temp_dir.path().join("test.js");
    
    let original_code = "console.log('hello world');";
    fs::write(&input_file, original_code).expect("Failed to write original file");
    
    // Test that propose generates a diff without modifying the original file
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "diff", "propose",
        "--provider", "mock",
        "--instruction", "Add error handling to this console.log statement",
        "--file", input_file.to_str().unwrap()
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Original file should remain unchanged after propose
    let current_content = fs::read_to_string(&input_file).expect("Failed to read file");
    assert_eq!(current_content, original_code, "Original file was modified during propose");
    
    // Output should contain diff format
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("---") || output_str.contains("+++") || output_str.contains("@@"));
}

/// Test 6: Multiple File Generation Test
/// Test the ability to work with multiple files simultaneously
#[test]
fn test_multiple_file_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let models_file = temp_dir.path().join("models.js");
    let routes_file = temp_dir.path().join("routes.js");
    
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "generate",
        "--provider", "mock",
        "--instruction", "Create a basic MVC structure with user model and routes",
        "--files", models_file.to_str().unwrap(),
        "--files", routes_file.to_str().unwrap()
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Should generate content for both files
    assert!(models_file.exists());
    assert!(routes_file.exists());
}

/// Test 7: Diff Propose Safety Test
/// Test that diff propose generates proper diffs without modifying files
#[test]
fn test_diff_propose_safety() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_file = temp_dir.path().join("test.js");
    
    let original_code = "console.log('hello world');";
    fs::write(&input_file, original_code).expect("Failed to write original file");
    
    // Test that diff propose generates a proper diff
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(&[
        "diff", "propose",
        "--provider", "mock",
        "--instruction", "Add error handling to this console.log statement",
        "--file", input_file.to_str().unwrap()
    ]);
    
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Original file should remain unchanged after propose
    let current_content = fs::read_to_string(&input_file).expect("Failed to read file");
    assert_eq!(current_content, original_code, "Original file was modified during propose");
    
    // Output should contain diff format
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("---") && output_str.contains("+++") && output_str.contains("@@"), 
           "Output should be a proper unified diff format");
}
