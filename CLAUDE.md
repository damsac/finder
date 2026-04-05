# Finder

iOS app: point your camera, describe what you lost, see it highlighted.

## Architecture

- `crates/core/` — Rust: ppq.ai vision client, frame processing, SQLite history
- `crates/ffi/` — UniFFI bridge to Swift
- `ios/` — SwiftUI camera app with bounding box overlay
- RMP (Rust Multi-Platform) scaffold — XcodeGen, iOS CI, e2e video pipeline

## Commands

```
just test              # cargo test --workspace
just build             # cargo build --workspace
just clippy            # Lint
just ios-build         # Cross-compile + generate Swift bindings + XCFramework
just ios-build-release # Release build
just pre-merge         # fmt + clippy + test
```

## API

Uses ppq.ai (OpenAI-compatible endpoint) with Claude Sonnet for vision.
API key injected via Info.plist `PPQAPIKey` at build time.

## Key files

- `crates/core/src/vision.rs` — ppq.ai vision client
- `crates/core/src/models.rs` — BBox, DetectionResult, SearchResult
- `crates/ffi/src/lib.rs` — AppCore UniFFI object
- `ios/Sources/App.swift` — Camera + search UI + bbox overlay
