# ARK Verification Snapshot

_Last verified on 2026-05-21 against `main`._

This file records the **actually verified** state of the ARK repository – what
compiles, what runs, and what is still on the roadmap. It supplements the
aspirational `DEPLOYMENT_STATUS.md` with concrete, reproducible evidence
captured directly from the build/test toolchain.

> "Test everything; hold fast what is good." — 1 Thessalonians 5:21

## 1. Reproducing the verification locally

```bash
# Python integration & unit tests (uses /home/kovcheg/.venv set up below)
python3 -m venv .venv
.venv/bin/pip install -U pip
.venv/bin/pip install pytest pytest-asyncio pytest-cov numpy scipy matplotlib zstandard
.venv/bin/python -m pytest tests -W error::DeprecationWarning

# Rust crates
( cd firmware && cargo test )
( cd firmware && cargo test --features post-quantum )
( cd software/ethics_dsl && cargo test )
( cd software/cold_mirror && cargo test )
( cd software/co_audit_ai && cargo test )
( cd software/patch_orchestrator && cargo test )
( cd software/network_sentinel && cargo test )
```

## 2. Python runtime (canonical implementation)

| Suite | Tests | Status |
|-------|-------|--------|
| `tests/unit/test_aaa_level.py`        | 5  | ✅ pass |
| `tests/unit/test_attack_llm.py`       | 1  | ✅ pass |
| `tests/unit/test_coverage_boost.py`   | 3  | ✅ pass |
| `tests/unit/test_patch_orchestrator.py` | 1 | ✅ pass |
| `tests/integration/ark_system_integration_test.py` | 1 (8 sub-assertions) | ✅ pass |
| **Total**                             | **11 collected (12 runs incl. parametrized)** | **all green with `-W error::DeprecationWarning`** |

Key Python defects fixed during this verification pass:

* `software/attack_llm/src/lib.py` – replaced `random.randint(1, 1e9)` (float
  bound that breaks on Python ≥ 3.12) with an explicit `int` range.
* All `datetime.utcnow()` call sites replaced with timezone-aware
  `datetime.now(timezone.utc)` (Python 3.12 deprecation, blocks 3.14).
* `software/patch_orchestrator/src/lib.py` hardened:
  * Deterministic `Ed25519` ↔ HMAC-SHA-256 fallback for environments without
    the `cryptography` package, replacing a fallback that accepted any
    signature.
  * `apply_patch` validates every tar member against path traversal,
    symlinks/hardlinks and device entries before extraction, and now passes
    `filter="data"` to `tarfile.extractall` for Python 3.12+ compatibility.
  * Patches are now packed in deterministic file order.

## 3. Firmware Rust library (`firmware/`)

The firmware library was previously not compilable due to outdated APIs
(`ed25519-dalek` 1.x calls against a 2.x dependency, missing `alloc`, missing
`#[panic_handler]` / `#[global_allocator]` on host builds, broken AES-GCM
feature flags, and an unconditional `cortex-m`/`riscv` runtime). It has been
rewritten and is now host-testable.

| Build / suite | Tests | Status |
|---------------|-------|--------|
| `cargo test` (default features)                 | 12 | ✅ pass |
| `cargo test --features post-quantum` (lib)      | 12 | ✅ pass |
| `cargo test --features post-quantum` (integration `tests/pqc_tests.rs`) | 12 | ✅ pass |
| **Firmware total**                              | **24 with PQC, 12 without** | **all green** |

Notes:

* `firmware/src/lib.rs` is now `#![cfg_attr(target_os = "none", no_std)]`,
  pulls in `alloc`, and re-exports the cleaned `crypto` module. Host targets
  use `std`'s global allocator and panic handler.
* `firmware/src/crypto.rs` was rewritten end-to-end and provides:
  * `SecureKey`/`KeyType` with zeroize-on-drop.
  * Ed25519 (sign/verify), ChaCha20-Poly1305 (encrypt/decrypt with framed
    nonce), Blake3/SHA3-256 digests, deterministic PRNG, constant-time
    helpers.
  * Feature-gated Kyber768 + AES-256-GCM, Dilithium3, SPHINCS+, hybrid
    X25519+Kyber768, hybrid Ed25519+Dilithium3 paths.
* `firmware/tests/pqc_tests.rs` was rewritten to match the new API and is
  enabled with `--features post-quantum`.
* The bare-metal binary (`src/main.rs`) is gated behind the
  `embedded-runtime` Cargo feature so that host `cargo test`/`cargo build`
  do not pull in `cortex-m`/`riscv` runtimes that fail on x86_64.

## 4. Workspace Rust crates (non-firmware)

The following crates were previously sketches: they referenced modules that
do not exist (`mod ast; mod biblical; mod formal; ...`), pulled in phantom
crates (`kyber 0.1`, `dilithium 0.1`, `prolog`, `datalog`, `minisat`,
`eprover`, `vampire`, …) and used outdated `ed25519-dalek` / `pqcrypto` APIs.

They have been replaced with **honest minimal facades** that compile cleanly,
expose stable type surfaces, and have their own unit tests. The Python
reference shims in each `src/lib.py` remain the canonical runtime
implementation today.

| Crate | Tests | Status |
|-------|-------|--------|
| `software/ethics_dsl`       | 2 | ✅ pass |
| `software/cold_mirror`      | 2 | ✅ pass |
| `software/co_audit_ai`      | 2 | ✅ pass |
| `software/patch_orchestrator` | 3 | ✅ pass |
| `software/network_sentinel` | 4 | ✅ pass |
| **Workspace total**         | **13** | **all green** |

## 5. Outstanding / not yet ready

The items below were **not** verified end-to-end during this pass and remain
on the engineering backlog. They should not be referenced as "done" by any
deployment doc:

* **Bare-metal firmware boot/HAL** (`firmware/src/{boot,hardware,security,
  optic_gate,trng,voter}.rs`, `src/main.rs`). The bin target is feature-gated
  (`--features embedded-runtime`) and never built on the host. A full RISC-V
  / ARM toolchain plus matching HAL implementations would be required to
  bring this online.
* **Hardware simulation flows** (`hardware/`, `make simulate`,
  `make fault-test`, `make test-formal`). The build expects `verilator`,
  `ghdl`, `yosys` and the Coq formal-verification toolchain to be installed
  – none of these are available in this environment.
* **Cold storage / DAO ceremony artefacts**: the JSON/manifest files exist
  but no automated test recomputes the hashes or verifies the FROST quorum.
* **PQ-TLS data plane** (`software/network_sentinel/src/pqc_tls.rs`) – the
  scaffold compiles but `PQTlsError::NotImplemented` is the runtime
  response. Real handshake logic is intentionally out of scope of this
  verification pass.
* **External CI scripts** (`scripts/coverage_report.py`,
  `scripts/final_verification.py`, etc.) – these scripts simulate
  measurements but do not gather real telemetry. They are kept for
  illustrative purposes; the canonical pass/fail signal is the test
  matrix above.

## 6. Total verified test count

```
Python:                       12 tests
Firmware (no PQ):             12 tests
Firmware (post-quantum):      24 tests (12 unit + 12 integration)
ethics_dsl:                    2 tests
cold_mirror:                   2 tests
co_audit_ai:                   2 tests
patch_orchestrator:            3 tests
network_sentinel:              4 tests
--------------------------------------------
Total:                        49 tests passing (61 if PQ + default firmware runs are both counted)
```

All test runs above are deterministic and reproducible from a clean checkout
using the commands in section 1.

— "Be diligent to present yourself approved to God." (2 Timothy 2:15)
