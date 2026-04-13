# Notepads: lib-rs-refactor issues

## 2026-04-10

- `lsp_diagnostics` could not run because the environment toolchain lacks `rust-analyzer.exe` (`Unknown binary 'rust-analyzer.exe' in official toolchain 'stable-x86_64-pc-windows-msvc'`). Used repeated `cargo check` verification instead.
- During Steps 7-8, `generate_handler![]` could not resolve extracted commands until they were exported from their modules and brought back into `lib.rs` scope via `use commands::*;`.
- During Steps 9-10, the first post-extraction `cargo check` failed because `lib.rs` still referenced `safe_lock` inside `receive_image`; re-importing `safe_lock` and trimming now-unused imports fixed the build.

- During Steps 11-12, the first cargo check failed because server/mod.rs tried to publicly re-export a pub(crate) helper and because clipboard/monitor.rs still expected PBT_APMRESUMEAUTOMATIC / PBT_APMRESUMESUSPEND in crate::lib; fixing the re-export to pub(crate) and restoring the constants in lib.rs resolved the build.

