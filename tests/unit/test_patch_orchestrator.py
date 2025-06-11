import tempfile
from pathlib import Path

from software.patch_orchestrator.src.lib import (
    OrchestratorConfig,
    PatchOrchestrator,
)


async def _make_dummy_files(root: Path, count: int = 3) -> None:
    for i in range(count):
        (root / f"file_{i}.txt").write_text(f"dummy-content-{i}\n")


def test_patch_pack_verify_apply(asyncio_executor):
    """End-to-end sanity check of the Python PatchOrchestrator shim."""

    with tempfile.TemporaryDirectory() as temp_dir:
        base = Path(temp_dir)
        src = base / "src"
        dst = base / "dst"
        patch_path = base / "update.arkpatch"
        for p in (src, dst):
            p.mkdir(parents=True, exist_ok=True)

        # Create dummy payload
        asyncio_executor(_make_dummy_files, src)

        cfg = OrchestratorConfig(
            patch_directory=base,
            staging_directory=base / "staging",
            backup_directory=base / "backup",
            max_patch_size=1024 * 1024,
            verification_timeout=1,
            auto_apply_threshold="Low",
            require_biblical_justification=False,
            signing_keys={},
        )
        orchestrator = PatchOrchestrator(cfg)

        # Ensure keys & package
        orchestrator.ensure_keys()
        orchestrator.create_patch_from_dir(src, patch_path)

        # Verify signature
        assert orchestrator.verify_patch(patch_path), "Signature check failed"

        # Apply
        asyncio_executor(orchestrator.apply_patch, patch_path, dst)

        # Files must appear in destination
        for i in range(3):
            assert (dst / f"file_{i}.txt").exists(), "Patched file missing" 