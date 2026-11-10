# Provisional AST baseline for review

This is a working-tree AST measurement, not an approved reference and not an AST/bytecode comparison. The bytecode backend does not exist yet. Regenerate or explicitly accept the baseline after reviewing the AST changes.

Captured: 2026-11-09T04:44:13.908888+00:00.
Platform: Linux-7.3.0-10-generic-x86_64-with-glibc2.44.
CPU: Intel(R) Core(TM) i5-10210U CPU @ 1.60GHz.
Compiler: rustc 1.98.1 (48a229cea 2026-09-01).
Samples: 7 per workload, after 2 warmup rounds; randomized order with seed 0.

Build: `cargo build -p expressa --release --bin expressa --offline --locked`.
Run: `python3 benchmarks/run.py --engine ast=target/release/expressa --repeat 7 --warmup 2 --output benchmarks/results/ast-review.json`.

| Workload | CPU median ms | CPU min–max ms | Wall median ms | RSS median KiB | RSS min–max KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| arithmetic | 60.967 | 58.393–63.025 | 61.112 | 4820 | 4756–4968 |
| avalie | 4.575 | 4.277–5.301 | 4.664 | 4816 | 4736–4936 |
| closures | 69.956 | 68.360–75.296 | 70.279 | 4808 | 4624–4992 |
| collections | 61.020 | 58.353–65.170 | 61.174 | 4908 | 4772–5004 |
| cycles | 8.232 | 7.564–10.304 | 8.355 | 5140 | 5040–5192 |
| matrix | 2.833 | 2.764–3.412 | 2.931 | 4840 | 4760–5004 |
| startup | 0.995 | 0.978–1.213 | 1.093 | 4900 | 4700–4972 |

CPU is user + system process time. RSS is peak resident memory per interpreter child, collected by the small native helper. Every sample passed its expected-output and exit-status checks. The startup case and other short runs are especially sensitive to noise; these numbers establish neither a speedup nor Pico suitability.

These measurements include startup, parsing, execution, and teardown. They do not isolate heap allocation, live objects, parsing overhead, or VM frame memory. Future execution-only and Pico results must be reported separately.

Executable bytes: 4069736 (not firmware flash usage).

Working source SHA-256: `04a2c5993c577fbd73d9f73f8af64ebed04700a1f70101884adfd4e32ba41eb3`.
AST binary SHA-256: `32ff26b98c199b3baa906f42da8de997d35e2cb7cb44b85cc53c855c827aa5f7`.

See [raw samples and metadata](results/ast-review.json) and [the review-gated comparison protocol](README.md).
