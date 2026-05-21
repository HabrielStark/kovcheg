//! ARK Co-Audit AI CLI entry point.
//!
//! The runtime implementation lives in the Python shim today; this binary
//! exists so the Rust workspace builds without unresolved references.

fn main() {
    println!(
        "ARK Co-Audit AI v{} — native CLI placeholder.\n\
         Use the Python reference implementation in software/co_audit_ai/src/lib.py.",
        co_audit_ai::CO_AUDIT_VERSION
    );
}
