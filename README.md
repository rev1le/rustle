## Concept
A Rust utility for disk space analysis that classifies files as "needed" / "trash" via a multi-level AI agent.

## Differentiators
- AI classification (heuristics + local LLM)
- Context for a photographer (RAW, projects) and a full-stack developer (node_modules, cache)
- Privacy: the LLM runs locally

## Architecture
```
[Rust: FS scanner] → [cache/DB] → [classification router]
                                    ├─ L1: hard rules (ext, patterns)
                                    ├─ L2: heuristics (size, age, path)
                                    └─ L3: LLM agent (only uncertain)
                                        ↓
                                    [recommendations + UI]
```

## Stack
- `walkdir`/`ignore` — filesystem traversal
- `rayon` — parallelism
- `clap` — CLI
- `indicatif` — progress
- `ratatui` + `crossterm` — TUI (optional)
- `serde` + `toml` — rules config
- Local LLM: Ollama / llama.cpp (Qwen/Llama 3 GGUF)

## Classification model
```rust
enum Classification {
    Keep,
    Trash,
    Uncertain,
}
```
The numeric trash-confidence score (`0.0..=1.0`) is exposed by each layer's
`ClassifierLayer::score`, not by the enum variant — a `Classification` is purely
categorical.

## Road-map

**Stage 1 — MVP (1-2 weeks)**
- [ ] Recursive filesystem traversal with `walkdir`
- [ ] Collect metadata: path, size, ext, mtime
- [ ] Aggregation by directories
- [ ] Top-N largest files
- [ ] CLI output via `clap`

**Stage 2 — Rule-based classifier (1 week)**
- [ ] Whitelist/blacklist of extensions
- [ ] Trash patterns (thumbs.db, .DS_Store, node_modules)
- [ ] Heuristics: age + size
- [ ] Config in TOML (user-defined rules)

**Stage 3 — AI agent (1-2 weeks)**
- [ ] Local LLM integration (Ollama API)
- [ ] Build context for the LLM
- [ ] Cache responses by file hash
- [ ] Confidence threshold for invoking the LLM

**Stage 4 — UX and visualization (1 week)**
- [ ] TUI via `ratatui` (tree + heatmap)
- [ ] Interactive deletion/archiving
- [ ] Reports (HTML/JSON)

**Stage 5 — Advanced (optional)**
- [ ] Duplicate detection by hash
- [ ] Media analysis (RAW/JPEG pairs)
- [ ] Web interface (actix-web + treemap)
- [ ] Cross-platform (Win/Linux/macOS)

## Key decisions
- LLM runs locally only (privacy)
- Hybrid: heuristics first, LLM on demand
- Configurable rules (TOML)
- Cache LLM responses across re-runs
