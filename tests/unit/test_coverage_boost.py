import importlib
import sys
import tempfile
from pathlib import Path

import pytest

from software.co_audit_ai.src.lib import CoAuditAI
from software.ethics_dsl.src.lib import EthicsEngine, Decision
from software.cold_mirror.src.lib import HarmPredictor, RiskLevel


###############################################################################
# CoAuditAI simple call – covers analyze() lines                              #
###############################################################################

def test_coaudit_analyze_pass():
    ai = CoAuditAI()
    result = ai.analyze("dummy-subject")
    assert result["status"] == "PASS"


###############################################################################
# EthicsEngine misc helper coverage                                           #
###############################################################################

def test_ethics_foundation_hash_and_update_rules():
    eng = EthicsEngine.new_with_biblical_foundation()
    h = eng.get_foundation_hash()
    assert len(h) == 64  # SHA-256 hex length

    # update_rules is placeholder – ensure it raises
    with pytest.raises(NotImplementedError):
        eng.update_rules()


###############################################################################
# PatchOrchestrator fallback path coverage (no zstandard / cryptography)       #
###############################################################################

@pytest.mark.skipif(sys.platform.startswith("win") and sys.maxsize > 2**32, reason="CI perf only")
def test_patch_orchestrator_fallback_paths(monkeypatch):
    """Reload module without zstandard/cryptography to execute fallback code."""
    # Backup original modules
    original_zstd = sys.modules.get("zstandard")
    original_crypto = {name: sys.modules.get(name) for name in list(sys.modules) if name.startswith("cryptography")}

    with monkeypatch.context() as m:
        m.setitem(sys.modules, "zstandard", None)
        # Remove cryptography modules
        for k in list(sys.modules):
            if k.startswith("cryptography"):
                del sys.modules[k]
        # Reload module
        if "software.patch_orchestrator.src.lib" in sys.modules:
            del sys.modules["software.patch_orchestrator.src.lib"]
        po = importlib.import_module("software.patch_orchestrator.src.lib")

        assert po._use_zstd is False
        assert po._has_crypto is False

        # Use fallback generate_keypair
        priv, pub = po.PatchOrchestrator.generate_keypair()
        assert isinstance(priv, bytes) and isinstance(pub, bytes)

        # Exercise fallback compression/decompression logic
        data = b"ark-test-data"
        compressed = po.PatchOrchestrator._compress_bytes(data)
        assert isinstance(compressed, bytes) and compressed != data
        decompressed = po.PatchOrchestrator._decompress_bytes(compressed)
        assert decompressed == data

        # End-to-end patch cycle under fallback environment
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            src = base / "src"
            dst = base / "dst"
            src.mkdir()
            dst.mkdir()
            (src / "file.txt").write_text("x")
            patch_path = base / "upd.arkpatch"

            cfg = po.OrchestratorConfig(
                patch_directory=base,
                staging_directory=base / "stage",
                backup_directory=base / "bak",
                max_patch_size=1024,
                verification_timeout=1,
                auto_apply_threshold="Low",
                require_biblical_justification=False,
                signing_keys={},
            )
            orch = po.PatchOrchestrator(cfg)
            orch.create_patch_from_dir(src, patch_path)
            assert orch.verify_patch(patch_path)
            # apply_patch is async; run quickly
            import asyncio as _a
            _a.run(orch.apply_patch(patch_path, dst))
            assert (dst / "file.txt").exists()

    # Restore original modules
    if original_zstd is not None:
        sys.modules["zstandard"] = original_zstd
    else:
        sys.modules.pop("zstandard", None)
    sys.modules.update(original_crypto)


def test_cold_mirror_low_biblical_path():
    predictor = HarmPredictor()
    # Use synchronous internal call for deterministic path
    pred = predictor._predict_single_enhanced("We should harm but with righteousness and love")
    assert pred.risk_level is RiskLevel.Low 