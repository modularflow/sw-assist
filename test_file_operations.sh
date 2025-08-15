#!/bin/bash

# Gemini CLI - Enhanced File Operations Test Suite
# Tests all file operations capabilities with comprehensive coverage

set -e  # Exit on any error

echo "🚀 Starting Enhanced File Operations Test Suite"
echo "================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counter
TEST_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0

# Helper functions
run_test() {
    local test_name="$1"
    local command="$2"
    local expected_success="${3:-true}"
    
    TEST_COUNT=$((TEST_COUNT + 1))
    echo -e "\n${BLUE}Test $TEST_COUNT: $test_name${NC}"
    echo "Command: $command"
    
    if [[ "$expected_success" == "true" ]]; then
        if eval "$command" > /dev/null 2>&1; then
            echo -e "${GREEN}✅ PASS${NC}"
            PASS_COUNT=$((PASS_COUNT + 1))
        else
            echo -e "${RED}❌ FAIL${NC}"
            FAIL_COUNT=$((FAIL_COUNT + 1))
        fi
    else
        if ! eval "$command" > /dev/null 2>&1; then
            echo -e "${GREEN}✅ PASS (Expected failure)${NC}"
            PASS_COUNT=$((PASS_COUNT + 1))
        else
            echo -e "${RED}❌ FAIL (Expected failure but succeeded)${NC}"
            FAIL_COUNT=$((FAIL_COUNT + 1))
        fi
    fi
}

run_test_with_output() {
    local test_name="$1"
    local command="$2"
    
    TEST_COUNT=$((TEST_COUNT + 1))
    echo -e "\n${BLUE}Test $TEST_COUNT: $test_name${NC}"
    echo "Command: $command"
    
    if eval "$command"; then
        echo -e "${GREEN}✅ PASS${NC}"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "${RED}❌ FAIL${NC}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

# Setup test environment
echo -e "\n${YELLOW}Setting up test environment...${NC}"

# Create test directories and files
mkdir -p test_workspace/{src,tests,docs,backup}
mkdir -p test_workspace/src/{components,utils,services}
mkdir -p test_workspace/.git

# Create sample files for testing
cat > test_workspace/src/main.rs << 'EOF'
use std::io;

// TODO: Add error handling
fn main() {
    println!("Hello, world!");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
}

async fn handle_request() -> Result<String, io::Error> {
    Ok("response".to_string())
}
EOF

cat > test_workspace/src/utils/auth.js << 'EOF'
const jwt = require('jsonwebtoken');

// Hardcoded secret - security issue!
const SECRET = "super_secret_key_12345";

function authenticateUser(username, password) {
    // SQL injection vulnerability
    const query = "SELECT * FROM users WHERE username = '" + username + "' AND password = '" + password + "'";
    
    if (Math.random() > 0.5) {
        return jwt.sign({user: username}, SECRET);
    }
    return null;
}

module.exports = { authenticateUser };
EOF

cat > test_workspace/src/components/Button.jsx << 'EOF'
import React from 'react';

// XSS vulnerability
const Button = ({onClick, children, dangerousHTML}) => {
    return (
        <button onClick={onClick} dangerouslySetInnerHTML={{__html: dangerousHTML}}>
            {children}
        </button>
    );
};

export default Button;
EOF

cat > test_workspace/package.json << 'EOF'
{
  "name": "test-project",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "4.17.20",
    "jquery": "3.4.1",
    "express": "4.17.0"
  }
}
EOF

cat > test_workspace/.gitignore << 'EOF'
node_modules/
*.log
.env
target/
dist/
EOF

cat > test_workspace/Cargo.toml << 'EOF'
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
tokio = "1.0"
serde = "1.0"
EOF

# Change to test workspace
cd test_workspace

echo -e "${YELLOW}Test environment created successfully!${NC}"

# Test 1: Basic File Operations
echo -e "\n${YELLOW}=== 1. BASIC FILE OPERATIONS ===${NC}"

run_test "List files in current directory" \
    "cargo run --manifest-path ../Cargo.toml files list --path ."

run_test "List files recursively" \
    "cargo run --manifest-path ../Cargo.toml files list --path . --recursive"

run_test "List files with extension filtering" \
    "cargo run --manifest-path ../Cargo.toml files list --path . --recursive --include-ext rs,js"

run_test "List files excluding certain extensions" \
    "cargo run --manifest-path ../Cargo.toml files list --path . --recursive --exclude-ext json,toml"

# Test 2: Git-Aware Operations
echo -e "\n${YELLOW}=== 2. GIT-AWARE OPERATIONS ===${NC}"

run_test "Git-aware file listing (default behavior)" \
    "cargo run --manifest-path ../Cargo.toml files list --path . --recursive"

run_test "Disable git-awareness" \
    "cargo run --manifest-path ../Cargo.toml files list --path . --recursive --no-git"

# Test 3: File Analysis
echo -e "\n${YELLOW}=== 3. FILE ANALYSIS ===${NC}"

run_test "Analyze single file" \
    "cargo run --manifest-path ../Cargo.toml files analyze --path src/main.rs"

run_test "Analyze directory recursively" \
    "cargo run --manifest-path ../Cargo.toml files analyze --path src --recursive"

run_test "Detailed analysis with dependencies" \
    "cargo run --manifest-path ../Cargo.toml files analyze --path . --recursive --detailed --dependencies"

run_test "Analysis with JSON output" \
    "cargo run --manifest-path ../Cargo.toml files analyze --path src/main.rs --json"

# Test 4: Security Scanning
echo -e "\n${YELLOW}=== 4. SECURITY SCANNING ===${NC}"

run_test_with_output "Basic security scan" \
    "cargo run --manifest-path ../Cargo.toml files security --path ."

run_test_with_output "Security scan with high severity only" \
    "cargo run --manifest-path ../Cargo.toml files security --path . --high-only"

run_test "Security scan with minimum risk score" \
    "cargo run --manifest-path ../Cargo.toml files security --path . --min-risk 50"

run_test "Security scan with specific checks" \
    "cargo run --manifest-path ../Cargo.toml files security --path . --check-credentials --check-injection"

run_test "Security scan with file type filtering" \
    "cargo run --manifest-path ../Cargo.toml files security --path . --types js,jsx,json"

# Test 5: Advanced Search
echo -e "\n${YELLOW}=== 5. ADVANCED SEARCH ===${NC}"

run_test "Basic text search" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern 'function' --path ."

run_test "Regex search" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern 'async\s+fn' --regex --path ."

run_test "Semantic search" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern 'authenticateUser' --semantic --path ."

run_test "Fuzzy search" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern 'authnticate' --fuzzy --path ."

run_test "Search with context lines" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern 'TODO' --context 2 --path ."

run_test "Search with file type filtering" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern 'const' --types js,jsx --path ."

# Test 6: Search and Replace
echo -e "\n${YELLOW}=== 6. SEARCH AND REPLACE ===${NC}"

run_test "Search and replace (dry run)" \
    "cargo run --manifest-path ../Cargo.toml files replace --pattern 'TODO:' --replace 'FIXME:' --dry-run --path ."

run_test "Search and replace with regex (dry run)" \
    "cargo run --manifest-path ../Cargo.toml files replace --pattern 'const\s+' --replace 'let ' --regex --dry-run --path ."

# Test 7: Templates and Scaffolding
echo -e "\n${YELLOW}=== 7. TEMPLATES AND SCAFFOLDING ===${NC}"

run_test "List available templates" \
    "cargo run --manifest-path ../Cargo.toml template list"

run_test "Generate Rust CLI project" \
    "cargo run --manifest-path ../Cargo.toml template generate --template rust-cli --name test-cli --output ../temp_rust_cli --author 'Test User'"

run_test "Generate Node Express project" \
    "cargo run --manifest-path ../Cargo.toml template generate --template node-express --name test-api --output ../temp_node_api --author 'Test User'"

run_test "Generate React component" \
    "cargo run --manifest-path ../Cargo.toml template generate --template react-component --name TestComponent --output ../temp_react --author 'Test User'"

# Test 8: Directory Operations
echo -e "\n${YELLOW}=== 8. DIRECTORY OPERATIONS ===${NC}"

# Create backup directory structure
mkdir -p ../backup_test/{src,tests}
cp -r src/* ../backup_test/src/ 2>/dev/null || true

run_test "Compare directories" \
    "cargo run --manifest-path ../Cargo.toml files compare --source . --target ../backup_test"

run_test "Compare directories with content analysis" \
    "cargo run --manifest-path ../Cargo.toml files compare --source . --target ../backup_test --content"

run_test "Directory sync (dry run)" \
    "cargo run --manifest-path ../Cargo.toml files sync --source . --target ../backup_test --dry-run"

run_test "Find duplicate files" \
    "cargo run --manifest-path ../Cargo.toml files duplicates --path . --recursive"

# Test 9: Checkpointing System
echo -e "\n${YELLOW}=== 9. CHECKPOINTING SYSTEM ===${NC}"

run_test "Create checkpoint" \
    "cargo run --manifest-path ../Cargo.toml checkpoint create --name test-checkpoint --description 'Test checkpoint creation'"

run_test "List checkpoints" \
    "cargo run --manifest-path ../Cargo.toml checkpoint list"

# Test 10: Batch Operations
# echo -e "\n${YELLOW}=== 10. BATCH OPERATIONS ===${NC}"

# run_test "Batch generate with mock provider" \
#     "cargo run --manifest-path ../Cargo.toml batch generate --pattern '*.rs' --instruction 'Add comprehensive error handling' --provider mock"

# run_test "Batch generate with checkpointing" \
#     "cargo run --manifest-path ../Cargo.toml batch generate --pattern '*.js' --instruction 'Add JSDoc comments' --provider mock --checkpoint"

# run_test "Batch transform with mock provider" \
#     "cargo run --manifest-path ../Cargo.toml batch transform --files 'src/main.rs' --instruction 'Convert to async/await pattern' --provider mock"

# Test 11: JSON Output Tests
echo -e "\n${YELLOW}=== 11. JSON OUTPUT TESTS ===${NC}"

run_test "File listing with JSON output" \
    "cargo run --manifest-path ../Cargo.toml files list --path . --json"

run_test "Security scan with JSON output" \
    "cargo run --manifest-path ../Cargo.toml files security --path . --json"

run_test "Search with JSON output" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern 'function' --path . --json"

run_test "Directory comparison with JSON output" \
    "cargo run --manifest-path ../Cargo.toml files compare --source . --target ../backup_test --json"

# Test 12: Error Handling and Edge Cases
echo -e "\n${YELLOW}=== 12. ERROR HANDLING ===${NC}"

run_test "Handle non-existent directory" \
    "cargo run --manifest-path ../Cargo.toml files list --path /non/existent/path" false

run_test "Handle invalid file pattern" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern '[invalid' --regex --path ." false

run_test "Handle empty search pattern" \
    "cargo run --manifest-path ../Cargo.toml files search --pattern '' --path ." false

# Cleanup
echo -e "\n${YELLOW}Cleaning up test environment...${NC}"
cd ..
rm -rf test_workspace temp_rust_cli temp_node_api temp_react backup_test 2>/dev/null || true

# Test Results Summary
echo -e "\n${YELLOW}===============================================${NC}"
echo -e "${YELLOW}           TEST RESULTS SUMMARY${NC}"
echo -e "${YELLOW}===============================================${NC}"
echo -e "Total Tests: $TEST_COUNT"
echo -e "${GREEN}Passed: $PASS_COUNT${NC}"
echo -e "${RED}Failed: $FAIL_COUNT${NC}"

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "\n${GREEN}🎉 ALL TESTS PASSED! 🎉${NC}"
    echo -e "${GREEN}Enhanced File Operations are working correctly!${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed. Please review the output above.${NC}"
    exit 1
fi
