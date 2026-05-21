//! ARK Patch Orchestrator CLI entry point.
//!
//! The runtime implementation lives in the Python shim today; this binary
//! exists so the Rust workspace builds without unresolved references.

fn main() {
    println!(
        "ARK Patch Orchestrator v{} — native CLI placeholder.\n\
         Use the Python reference implementation in software/patch_orchestrator/src/lib.py.",
        patch_orchestrator::PATCH_ORCHESTRATOR_VERSION
    );
}
