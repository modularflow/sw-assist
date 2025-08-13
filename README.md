## Software Assistant CLI (sw)

A Rust-based CLI AI assistant for developers. It supports Q&A, code review, summarization, explanation, planning, session management, and model discovery across multiple providers.

### Features
- Ask and Chat with streaming or JSON output
- Summarize files (chunked, concurrent) and explain code ranges
- Code review (text or JSON rubric)
- Generate Conventional Commit messages
- TODO extraction with optional LLM normalization
- Session management (new/list/switch/show/search)
- Models discovery (config + provider fetch + cache) with capability enrichment
- Structured JSON errors in `--json` mode

### Install
- Prerequisites: Rust (stable), Cargo
- Build locally:
```
cargo build --release
```
- Run binary:
```
./target/release/sw --help
```

### Configuration
Config file (XDG): `~/.config/sw-assistant/config.toml`

Example:
```
default_profile = "default"

[profiles.default]
provider = "openai"  # e.g., openai | groq | lmstudio | mock
model = "gpt-4o-mini"
```

### Providers and environment variables
- OpenAI: set `OPENAI_API_KEY`
- Groq (OpenAI-compatible): set `GROQ_API_KEY`; base `https://api.groq.com/openai/v1`
- LM Studio (OpenAI-compatible local): set `LMSTUDIO_API_BASE` (default `http://127.0.0.1:1234/v1`)
- Future providers: Anthropic (`ANTHROPIC_API_KEY`), Google Gemini (`GOOGLE_API_KEY`)

### Common commands
- Ask (one-shot):
```
sw ask "What is Rust async?"
```
- Ask (JSON, streaming disabled automatically):
```
sw ask --json "Summarize Rust ownership."
```
- Chat (multi-turn, uses active session):
```
sw session new mychat
sw chat --session mychat
```
- Summarize a file:
```
sw summarize --file README.md
```
- Explain a range:
```
sw explain --file src/main.rs --range 10:50
```
- Code review:
```
sw review --diff-file changes.diff
sw review --diff-file changes.diff --json
```
- Commit message generation:
```
sw commit-msg --diff-file changes.diff
sw --json commit-msg --diff-file changes.diff
```
- TODOs extraction:
```
sw todos --file src/lib.rs
sw todos --file src/lib.rs --json
```
- Models discovery (with cache/refresh):
```
sw models list --provider openai --json
sw models list --provider groq --refresh --json
```

### JSON output schemas (selected)
- Ask (`--json`): `{ "model": string, "usage": object|null, "answer": string }`
- Review (`--json`): `{ "feedback": { "correctness": string[], "style": string[], "security": string[], "tests": string[], "suggestions": string[] } }`
- Commit-msg (`--json`): `{ "type": string, "scope": string|null, "subject": string, "body": string|null }`
- Summarize (`--json`): `{ "model": string, "chunks": number, "summary": string }`
- Explain (`--json`): `{ "model": string, "file": string, "range": string, "explanation": string }`
- Todos (`--json`): `[{ "line": number, "text": string, "priority": string|null, "owner": string|null }]`

### Structured JSON errors in `--json` mode
On failures, sw prints exactly one JSON error object and exits non-zero:
```
{ "code": "file_not_found", "message": "file not found: ...", "hint": "..." }
```
Stable `code` values: `file_not_found`, `missing_input`, `invalid_args`, `missing_api_key`, `timeout`, `provider_unsupported`, `network_error`, `parse_error`, `unknown`.

### Models capability enrichment
`sw models list` merges config + remote list + cache, and enriches capabilities:
- Fields per model: `provider`, `source`, `streaming`, `context_window`, `supports_json`, `supports_tools`, `modalities` (e.g., ["text"], ["text","vision"]).
- Best-effort provider metadata fetch for OpenAI; placeholders for other providers; user overrides via `model_overrides` in config.

### Sessions
- Stored at `~/.local/share/sw-assistant/sessions/{name}.jsonl`
- Commands: `session new|list|switch|show|search`

### Timeouts and retries
- Global `--timeout SEC` is honored by provider calls (default: 60s for most commands; may be 15s for model listing).
- Basic retry/backoff is implemented for provider HTTP calls.

### Development
- Run offline test suite:
```
cargo test
```
- Optional network tests (guarded):
```
RUN_NET_TESTS=1 GROQ_API_KEY=... cargo test --test net_groq
```

### Notes
- In `--json` mode, streaming is disabled automatically to ensure a single JSON object on stdout.
- For LM Studio or other OpenAI-compatible backends, sw uses the same request shape; configure the base via env.
- Mock provider is available for offline workflows and deterministic tests.
