## Software Assistant CLI — Tests, Behaviors, and Implementation Plan

This document captures the intended behaviors, natural-language tests (Given/When/Then), and cooperating function plans for a Rust-based CLI AI software assistant built with `clap` and `tokio`.

### Assumptions and scope
- **Goal**: A CLI “AI software assistant” that helps with general developer/productivity tasks (Q&A, summarization, code review, planning) via subcommands.
- **Tech**: Rust with `clap` (CLI parsing) and `tokio` (async runtime), async HTTP to model providers, optional streaming output.
- **Config**: Stored in `~/.config/sw-assistant/config.toml` with per-profile model settings and API keys via env or keyring.
- **Portability**: No platform-specific dependencies beyond common Rust crates.

## Proposed CLI surface (high level)
- `sw init` — interactive setup of API keys, defaults.
- `sw models list [--provider NAME] [--refresh] [--json]` — list available models (local config + provider fetch, cache with TTL).
- `sw ask "question" [-m MODEL] [--provider NAME] [--stream] [--session NAME] [--json]`
- `sw chat [-m MODEL] [--session NAME]` — interactive multi-turn.
- `sw summarize --file PATH [-m MODEL] [--provider NAME] [--max-tokens N] [--json]`
- `sw explain --file PATH [--range START:END] [--provider NAME] [--json]`
- `sw review --diff-file PATH [--provider NAME]` — review a diff/patch.
- `sw commit-msg --diff-file PATH [--provider NAME] [--json]` — suggest a commit message.
- `sw todos --file PATH [--provider NAME] [--normalize] [--json]` — extract/normalize TODOs/action items.
- `sw plan --goal "text" [--constraints "text"]`
- `sw session {new|list|switch|show|search} [NAME] [--contains TEXT] [--json]`
- `sw agent` — proposal-driven assistant for development actions (grep the codebase, propose code diffs, generate and run bash scripts) with explicit Accept/Reject gating.
- `sw grep --pattern PATTERN [--path PATH] [--type rs|py|ts|js|go|...] [--regex|--fixed] [--json]` — fast code search (ripgrep-based) for the current workspace.
- `sw diff {propose|apply} [--instruction TEXT] [--file PATH] [--files FILES...] [--yes] [--json]` — propose unified diffs from instructions or apply a provided diff after approval.
- `sw script {gen|run} [--goal TEXT | --file PATH] [--dry-run] [--yes] [--json]` — generate bash scripts for a goal and optionally run them with safety checks and approval.
- Global flags: `-m/--model`, `-p/--profile`, `--json`, `--no-color`, `-v/--verbose`, `--timeout SEC`.

---

## Use cases with natural-language tests and function compositions

### 1) First-time setup with `sw init`
**Behavior**: Guide user through config creation; validate keys; persist defaults.

**Natural-language tests**
- Given no config file exists, when the user runs `sw init`, then the assistant prompts for provider, API key (or detects env vars), preferred model, and writes a valid `config.toml`.
- Given an invalid API key is provided, when validation runs, then the assistant reports the error and re-prompts or allows skipping validation.
- Given config already exists, when `sw init` runs, then the assistant offers to update/overwrite or create a new profile.

**Functions that work together**
- `parse_cli_for_init`: Parse flags; detect non-interactive mode.
- `load_config_file_or_default`: Read config if present, else empty config.
- `prompt_for_setup_inputs`: Interactive TUI/TTY prompts (fallback to flags/env).
- `validate_provider_credentials_async`: Minimal provider call via async HTTP.
- `write_config_to_disk`: Serialize TOML to `~/.config/...`.
- `print_init_summary`: Display active profile/model.

### 2) Ask a one-shot question with `sw ask` (Status: implemented)
**Behavior**: Send a prompt to the selected model; support streaming and JSON output.

**Natural-language tests**
- Given a configured default model, when the user runs `sw ask "What is Rust async?"`, then the assistant returns a coherent answer, exits with code 0, and respects `--stream` by printing tokens as they arrive.
- Given `--json`, when the command completes, then stdout is valid JSON with fields: `model`, `usage`, `answer`.
- Given no API key in config or env, when `sw ask` runs, then it exits non-zero with actionable error text.

**Functions that work together**
- `parse_cli_for_ask`: Extract prompt, flags, session options.
- `resolve_effective_model`: Merge CLI, profile, defaults.
- `build_llm_request`: Create provider-specific request body.
- `send_llm_request_streaming_async`: Async HTTP with optional streaming.
- `render_response_to_stdout`: Pretty-print or JSON.
- `record_interaction_in_session_store`: Save Q/A with metadata.

### 3) Interactive chat with `sw chat` (Status: implemented)
**Behavior**: Maintain conversation context; support history persistence and model switching.

**Natural-language tests**
- Given a new session, when the user sends multiple messages, then the assistant includes prior turns for context and persists history.
- Given `--session NAME` matches an existing session, when `sw chat` starts, then prior conversation is loaded and continued.
- Given the user presses Ctrl+C, then the assistant gracefully saves the session and exits 0.

**Functions that work together**
- `parse_cli_for_chat`
- `load_or_create_session_history`: File- or sqlite-backed.
- `read_user_line_async`: TTY line-by-line input.
- `build_chat_completion_request`: Include past turns with truncation rules.
- `send_llm_request_streaming_async`
- `append_assistant_turn_and_save`: Persist after each turn.
- `handle_interrupt_signal`: Tokio signal handling to flush state.

### 4) Summarize a file with `sw summarize --file` (Status: implemented: mock path; real provider path for multi-chunk synthesis)
**Behavior**: Read a file, chunk if large, summarize, and merge summaries.

**Natural-language tests**
- Given a text file under token limits, when `sw summarize --file README.md` runs, then the assistant outputs a concise summary capturing key points.
- Given a large file, when run, then the assistant chunks content, summarizes each chunk, and produces a final merged summary.
- Given file path missing, then command exits non-zero with a helpful message.

**Functions that work together**
- `parse_cli_for_summarize`
- `read_file_to_string_async`
- `chunk_text_for_token_limit`: Uses tokenizer estimates.
- `summarize_chunk_async` (concurrent with Tokio tasks)
- `merge_summaries_with_prompt`: Second-pass synthesis.
- `render_summary_output`: Text/JSON modes.

### 5) Explain code region with `sw explain --file --range` (Status: implemented: mock path; real provider path available)
**Behavior**: Explain a code snippet or selected range with context-aware prompt.

**Natural-language tests**
- Given a file and range, when run, then the assistant explains what the code does, key functions, and possible pitfalls.
- Given no range, when run, then the assistant auto-detects top-level elements (e.g., fn or struct) around cursor-like markers or explains whole file briefly.

**Functions that work together**
- `parse_cli_for_explain`
- `read_file_segment_async`
- `build_code_explain_prompt`: Adds language hints based on extension.
- `send_llm_request_streaming_async`
- `render_explanation`

### 6) Review a diff with `sw review --diff-file` (Status: implemented)
**Behavior**: Provide code review feedback on a unified diff or patch file.

**Natural-language tests**
- Given a valid diff file in text mode, when run offline (no provider/env), then the assistant prints five headings with one bullet each: Correctness, Style, Security, Tests, Suggestions.
- Given a valid diff file and `--json`, when run offline, then stdout contains exactly one JSON object: `{ "feedback": { "correctness": string[], "style": string[], "security": string[], "tests": string[], "suggestions": string[] } }`, and `suggestions` has at least one element.
- Given no diff or invalid format, then exit non-zero with error text.

**Functions that work together**
- `parse_cli_for_review`
- `read_diff_file_async`
- `build_code_review_prompt`: Structured review rubric.
- `send_llm_request_async` with Provider timeout respected via `--timeout`
- `render_review_feedback`: Sections with headings and JSON rendering via `render::print_json`

### 7) Generate commit message with `sw commit-msg --diff-file` (Status: implemented)
**Behavior**: Produce conventional commit-style messages.

**Natural-language tests**
- Given a diff file, when run, then the assistant prints a single-line subject and optional body following Conventional Commits.
- Given `--json`, then output contains `type`, `scope`, `subject`, `body`.

**Functions that work together**
- `parse_cli_for_commit_msg`
- `read_diff_file_async`
- `build_commit_prompt_with_rules`
- `send_llm_request_async` (non-streaming)
- `render_commit_message`

### 8) Extract TODOs/action items with `sw todos --file` (Status: implemented; regex pre-pass + optional `--normalize` via LLM)
**Behavior**: Parse file and extract actionable tasks from comments and text.

**Natural-language tests**
- Given a file with various TODO/FIXME comments, then the assistant outputs a list of normalized tasks with priority and owners if found.
- Given `--json`, then output is a JSON array of tasks with `line`, `text`, `priority`.

**Functions that work together**
- `parse_cli_for_todos`
- `read_file_to_string_async`
- `detect_todos_regex_and_llm`: Hybrid: regex pre-pass + LLM normalization.
- `render_todo_list`

### 9) Planning assistance with `sw plan --goal` (Status: implemented; JSON schema)
**Behavior**: Create a step-by-step plan and artifacts list.

**Natural-language tests**
- Given a goal and constraints, when run, then assistant outputs a clear, ordered plan, risks, and success criteria.
- Given `--json`, returns machine-parseable plan with steps and dependencies.

**Functions that work together**
- `parse_cli_for_plan`
- `build_planning_prompt`
- `send_llm_request_async`
- `render_plan_output`

### 10) Models discovery with `sw models list` (Status: implemented; remote fetch, cache with TTL, `--refresh`)
**Behavior**: List available models from config and provider APIs.

**Natural-language tests**
- Given connectivity and credentials, then listing shows local configured models and fetched remote models with capabilities.
- Given offline or invalid key, then the command gracefully falls back to config-only and warns.

**Functions that work together**
- `parse_cli_for_models_list`
- `load_config_file_or_default`
- `fetch_provider_models_async` (with timeout and cache)
- `merge_model_lists_and_capabilities`
- `render_models_table`

### 11) Session management with `sw session {new|list|switch|show|search}` (Status: implemented; JSON output for list/show)
**Behavior**: Manage persistent conversation sessions.

**Natural-language tests**
- Given `session new NAME`, then a new session is created and becomes active.
- Given `session list`, then existing sessions are listed with last-used time and model.
- Given `session switch NAME`, then active session changes and is persisted.

**Functions that work together**
- `parse_cli_for_session`
- `ensure_sessions_store_exists`
- `create_session_metadata`
- `list_sessions_metadata`
- `set_active_session`
- `render_sessions_info`

### 12) Error handling, retries, and timeouts (cross-cutting) (Status: implemented)
**Behavior**: Provide helpful errors and robust network behavior.

**Natural-language tests**
- Given a transient network error, when a request fails, then the assistant retries with exponential backoff and jitter up to a limit and surfaces a concise error if all retries fail.
- Given a `--timeout` flag, then requests are canceled if exceeding timeout, exiting non-zero with a clear message.
 - Given `--json` and an error occurs (e.g., missing file), then stdout contains exactly one JSON error object `{ "code": string, "message": string, "hint": string|null }` and the process exits non-zero.

**Functions that work together**
- `with_retry_policy_async<F, T>`: Generic retry wrapper.
- `with_request_timeout_async<F, T>`
- `map_provider_errors_to_user_friendly_messages`
- `structured_logging`: Optional `-v` levels.
 - `render::print_json_error`: Structured JSON errors in `--json` mode.

### 13) Codebase exploration via `sw agent` and `sw grep` (Status: planned)
**Behavior**: The assistant can search the codebase to understand structure and locate symbols/usages. The user can Accept/Reject proposed grep queries. In JSON mode, outputs a machine-parseable list of matches.

**Natural-language tests**
- Given a project with Rust files, when the user runs `sw grep --pattern "fn main\(" --type rs --json`, then stdout is a JSON array of matches with fields `{ file, line, text }` and the process exits 0.
- Given `sw agent` and the user asks "Where is session history saved?", when the agent proposes a grep query `session.*jsonl` with an Accept/Reject prompt, then selecting Accept runs the grep and prints results; selecting Reject prompts the user for a refined instruction before running anything.
- Given no matches, when `--json` is set, then output is `[]` (empty array) with exit 0.

**Functions that work together**
- `parse_cli_for_grep`
- `detect_workspace_root` (default to CWD)
- `run_ripgrep_async` (with `--type`, `--regex|--fixed`)
- `render_grep_results` (text/JSON)
- `agent::propose_grep_query` (suggestions with Accept/Reject)
- `agent::prompt_accept_reject` (TTY buttons/prompt; non-interactive requires `--yes`)

### 14) Propose and apply code diffs with approval (Status: planned)
**Behavior**: The assistant proposes unified diffs to implement a change. The user must explicitly Accept to apply; Reject requires the user to enter a new instruction. Diff application writes to the working tree safely and reports changed files.

**Natural-language tests**
- Given an instruction "rename function X to Y in `src/lib.rs`", when `sw diff propose --instruction "rename X to Y in src/lib.rs" --json` runs, then stdout is a JSON object `{ summary, diff, changed_files }` and the process exits 0.
- Given a valid diff file, when the user runs `sw diff apply --file changes.diff` interactively, then the CLI shows a preview and an Accept/Reject prompt; Accept applies the diff and prints `applied N hunks to M files`, Reject aborts and prints `aborted`.
- Given non-interactive mode (piped), when `sw diff apply --file changes.diff` runs without `--yes`, then the command exits non-zero with a JSON error `{ code: "approval_required", ... }` in `--json` mode.

**Functions that work together**
- `parse_cli_for_diff`
- `build_edit_proposal_prompt` (summarize intent and candidate edits)
- `generate_diff_from_instruction_async` (LLM-backed; mock path offline)
- `validate_diff_safety` (no binary edits; within workspace)
- `preview_diff_with_context` (unified diff preview)
- `agent::prompt_accept_reject`
- `apply_unified_diff_to_fs` (atomic writes; create backups)
- `render_diff_result` (text/JSON)

### 15) Generate and run bash scripts with approval (Status: planned)
**Behavior**: The assistant generates a bash script to accomplish a goal (e.g., scaffolding, formatting) and can execute it only after user approval. Scripts run in a sandboxed shell with environment and timeout controls. In non-interactive mode, `--yes` is required to run.

**Natural-language tests**
- Given a goal "list largest files in `logs/`", when `sw script gen --goal "list largest files in logs/" --json` runs, then stdout contains `{ script, explanation }` and exit 0.
- Given a script file, when `sw script run --file ./tmp/script.sh` runs interactively, then the CLI shows the script with an Accept/Reject prompt; Accept executes and prints exit code, stdout/stderr; Reject aborts without side effects.
- Given `--json` and `--dry-run`, when `sw script run --file ./tmp/script.sh --dry-run --json` runs, then stdout is `{ script, would_run: true }` and the script is not executed.

**Functions that work together**
- `parse_cli_for_script`
- `build_script_generation_prompt` (LLM-backed; mock offline)
- `validate_script_safety` (denylist for `rm -rf /`, network, sudo unless explicitly allowed)
- `agent::prompt_accept_reject`
- `execute_script_captured_async` (timeout, cwd control, env passthrough)
- `render_script_result` (text/JSON)

### 16) File at-mentions in chat/agent using `@` (Status: planned)
**Behavior**: In `sw chat` or `sw agent`, the user can type `@PATH` to include file contents in context. Multiple files can be referenced. Missing files are handled with clear errors (and JSON error objects in `--json` mode).

**Natural-language tests**
- Given `sw chat --session s`, when the user types `Explain @src/main.rs`, then the assistant includes `src/main.rs` contents in the prompt (truncated by token budget) and proceeds.
- Given a missing file `@nope.txt`, when in `--json` mode, then the CLI prints exactly one JSON error `{ code: "file_not_found", ... }` and exits non-zero.
- Given multiple mentions `@src/a.rs @src/b.rs`, then both files are resolved and included (subject to size limits) with an ordering note.

**Functions that work together**
- `parse_at_mentions_from_input`
- `expand_globs_and_normalize_paths`
- `read_file_segment_async` + `chunk_text_for_token_limit`
- `build_chat_completion_request_with_files`
- `render_file_inclusion_notice`

---

## Core modules and responsibilities

- **`cli`**
  - `build_command_tree_with_clap`: Define subcommands/flags.
  - `dispatch_subcommand_async`: Route to handlers.
- **`config`**
  - `load_config_file_or_default`
  - `write_config_to_disk`
  - `resolve_effective_model`
- **`llm`**
  - `send_llm_request_async`
  - `send_llm_request_streaming_async`
  - `validate_provider_credentials_async`
  - Provider adapters trait: `ModelProvider` with impls (OpenAI, Anthropic, etc.)
- **`session`**
  - `load_or_create_session_history`
  - `append_assistant_turn_and_save`
  - `list_sessions_metadata`, `set_active_session`
- **`io`**
  - `read_file_to_string_async`, `read_file_segment_async`, `read_diff_file_async`
  - `chunk_text_for_token_limit`
- **`render`**
  - `render_response_to_stdout`, `render_models_table`, `render_plan_output`, etc.
  - Respect `--json`, `--no-color`.
- **`util`**
  - `with_retry_policy_async`, `with_request_timeout_async`
  - `estimate_tokens_for_text` (heuristic)
  - `handle_interrupt_signal`
 - **`agent`**
   - `propose_grep_query`, `propose_edit_diff`, `propose_script`
   - `prompt_accept_reject` (TTY buttons/prompt; non-interactive requires `--yes`)
   - `apply_unified_diff_to_fs`, `execute_script_captured_async`

## Cross-cutting behaviors and tests

- **Streaming output**
  - Test: Given `--stream`, tokens appear incrementally and the final newline is printed exactly once.
- **JSON mode**
  - Test: Given `--json`, outputs are single-line valid JSON per command schema, with no extraneous logs on stdout.
- **Verbosity and logging**
  - Test: Given `-v`, debug logs go to stderr; stdout remains clean for piping.
- **Color and TTY detection**
  - Test: Given piped stdout or `--no-color`, output contains no ANSI codes.
- **Profiles**
  - Test: Given `-p profile`, model and credentials resolve from that profile.
 - **Approvals and safety (Accept/Reject gating)**
   - Test: Given `sw agent` proposes a diff or script, selecting Accept proceeds; selecting Reject prompts for a new instruction.
   - Test: Given non-interactive mode, running an action that changes files (`diff apply`) or executes commands (`script run`) without `--yes` exits non-zero with `{ code: "approval_required" }` in `--json` mode.
   - Test: Given a diff proposes edits outside the workspace root, the command exits with `{ code: "invalid_args" }`.
   - Test: Given a script contains dangerous commands (e.g., `rm -rf /`), the runner blocks and surfaces `{ code: "blocked_action" }` unless an explicit `--allow-unsafe` is provided.

## Minimal state model

- Config at `~/.config/sw-assistant/config.toml`
  - `profiles.{name}.provider`, `profiles.{name}.model`, `profiles.{name}.api_key_ref` (env var or keyring ID), defaults.
- Sessions at `~/.local/share/sw-assistant/sessions/{name}.jsonl` (or sqlite)
  - Each line: `{timestamp, role, content, model, usage}`.
- Cache at `~/.cache/sw-assistant/models.json` for model discovery.
 - Action log (optional) at `~/.local/share/sw-assistant/actions.jsonl` capturing proposals and decisions: each line `{ timestamp, kind, proposal_hash, accepted: bool, details }`.

## Non-functional requirements

- Clear and actionable error messages; non-zero exit codes for failures.
- Respect network timeouts; retries with backoff for transient errors.
- Deterministic, machine-parseable output in `--json` mode only; logs to stderr.
- No ANSI colors when `--no-color` or non-TTY stdout.

## Deferred but planned

- Tooling/plugins (e.g., shell commands, web fetch) via a `Tool` trait.
- Local model backends via `llama.cpp`/`ollama` adapter.
- Inline citations and sources for retrieval-augmented tasks.

## Provider architecture and roadmap (Status: implemented v1 for OpenAI; adapters scaffolded)

- Abstraction: provider-agnostic adapter trait `ModelProviderAdapter` with `send` and `send_stream` using `LlmRequest`/`LlmResponse`. A `ProviderRegistry` maps provider names to adapters and honors `--timeout`.
- Initial implementation: **OpenAI** adapter supporting chat completion (sync + streaming). API key from `OPENAI_API_KEY` (supports `.env`).
- Implemented/compatible providers:
  - **OpenAI**: native Chat Completions; `OPENAI_API_KEY`.
  - **Groq**: OpenAI-compatible endpoint; set `GROQ_API_KEY`, API base `https://api.groq.com/openai/v1`.
  - **LM Studio**: local OpenAI-compatible server; set `LMSTUDIO_API_BASE` (default `http://127.0.0.1:1234/v1`).
- Planned adapters:
  - **Anthropic**: Claude Messages API (sync + streaming), `ANTHROPIC_API_KEY`.
  - **Grok (xAI)**: Chat Completions style API, `XAI_API_KEY`.
  - **Gemini (Google)**: Generative Language API, `GOOGLE_API_KEY`.
  - **Ollama**: Local HTTP server, model selection via `model` field; no key.
- Configuration: profiles store `provider`, `model`, and `api_key` or env ref; adapters read from env unless overridden by profile.
- Capability matrix captured by `models list`, including streaming support, token limits, and tool/function-call availability.

### Cross-cutting output schemas (consolidated)
- Ask (`--json`): `{ "model": string, "usage": object|null, "answer": string }`
- Commit-msg (`--json`): `{ "type": string, "scope": string|null, "subject": string, "body": string|null }`
- Summarize (`--json`): `{ "model": string, "chunks": number, "summary": string }`
- Explain (`--json`): `{ "model": string, "file": string, "range": string, "explanation": string }`
- Todos (`--json`): `[{ "line": number, "text": string, "priority": string|null, "owner": string|null }]`; with `--normalize`, LLM-backed normalization is used when provider != mock
- Sessions list (`--json`): `[{ "name": string, "lines": number, "size": number, "last_used_ms": number|null }]`
- Sessions show (`--json`): `{ "active": string|null, "lines": number, "size": number, "last_used_ms": number|null }`
- Models list (`--json`): `[{ "name": string, "provider": string, "source": "config|remote|cache", "streaming": bool, "context_window": number|null }]`
- Grep (`--json`): `[{ "file": string, "line": number, "text": string }]`
- Diff propose (`--json`): `{ "summary": string, "diff": string, "changed_files": string[] }`
- Diff apply (`--json`): `{ "applied_files": string[], "hunks": number }`
- Script gen (`--json`): `{ "script": string, "explanation": string }`
- Script run (`--json`): `{ "script": string, "exit_code": number, "stdout": string, "stderr": string }`
- Agent proposal wrapper (`--json`): `{ "kind": "grep|diff|script", "title": string, "details": object, "requires_approval": true }`

### Networking tests (optional)
- Guard all network tests with `RUN_NET_TESTS=1` and relevant API keys.
- Groq: `GROQ_API_KEY` must be set. Examples:
  - `sw ask --provider groq --json` returns JSON `{ model, usage|null, answer }`.
  - `sw models list --provider groq --refresh --json` succeeds.
- LM Studio: if `LMSTUDIO_API_BASE` is set (e.g., `http://127.0.0.1:1234/v1`), OpenAI-compatible paths work.

---

## Acceptance criteria for this phase

- This document enumerates use cases, Given/When/Then tests, and collaborating function plans per use case.
- Core CLI implemented with tests; streaming, JSON outputs, sessions, models cache, and retries/timeouts verified offline-first.
- Providers: OpenAI adapter functional; others scaffolded.

## Next major focus

- Polishing and providers
  - Broaden provider adapters (Anthropic, Groq, Gemini, Ollama) behind feature flags.
  - Improve models capability enrichment heuristics and caching.
  - Introduce `agent` and `tools` subsystem (grep, diff, script) with robust approvals and safety.


