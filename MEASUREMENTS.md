# Measurements

Stage 4, recorded by the agent. `B2 Require step-by-step justification`.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | rustc 1.99.0; wasm-bindgen 0.2.127 | 2026-10-12, start |
| Template, as copied | `cargo test --workspace` | the template's placeholder tests passed; replaced by this tool's | 2026-10-12, start |
| Core tests | `cargo test --workspace` | 92 passed (unit, hand-computed, edge, boundary, storage) | 2026-10-12, `ab8aeaf` |
| Full check | `./scripts/check.sh` | BUILDER CHECK PASSED; browser check 144 passes in Chromium, Firefox, WebKit | 2026-10-12, `ab8aeaf` |
| Live browser check | `BUILDER_ORIGIN=https://sgilson7.github.io/status-trends .venv/bin/python tests/browser/check.py` | BROWSER CHECK PASSED, 144 passes, three engines | 2026-10-12, gh-pages from `ab8aeaf` |
| Privacy proof | `./scripts/prove-privacy.sh` | five of five planted leaks caught | 2026-10-12, `ab8aeaf` |
| Release module | `ls -l dist/pkg/tool_shim_bg.wasm` | 535,148 bytes; one import (`__wbindgen_init_externref_table`) | 2026-10-12, `ab8aeaf` |
| Large file | 50,000 rows, 5.5 MB, Chromium | open to dashboard 0.26 s; switch to months 0.12 s | 2026-10-12 |
