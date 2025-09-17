import importlib
import sys
import tempfile
from pathlib import Path

import pytest

from software.co_audit_ai.src.lib import CoAuditAI, CoAuditConfig
from software.ethics_dsl.src.lib import EthicsEngine
from software.cold_mirror.src.lib import HarmPredictor, RiskLevel


###############################################################################
# CoAuditAI behavioural checks                                                 #
###############################################################################


def test_coaudit_ai_detects_eval_and_dynamic():
    snippet = """
    dangerous_cache = []

    def insecure(payload):
        dangerous_cache.append(payload)
        return eval(payload)
    """

    ai = CoAuditAI()
    report = ai.analyze(snippet)

    assert report["status"] == "FAIL"
    assert any(issue["rule"].startswith("CALL::eval") for issue in report["issues"])
    assert any(issue["dynamic"] for issue in report["issues"])


def test_coaudit_ai_detects_builtin_open_call():
    snippet = """
    def persist(data):
        handle = open("/tmp/secret.txt", "w")
        handle.write(data)
        handle.close()
    """

    ai = CoAuditAI()
    report = ai.analyze(snippet)

    call_issues = [issue for issue in report["issues"] if issue["rule"] == "CALL::open"]
    assert call_issues, report
    assert call_issues[0]["severity"] == "Medium"


def test_coaudit_ai_respects_dynamic_timeout():
    snippet = """
    def spin_forever():
        while True:
            pass
    """

    ai = CoAuditAI(CoAuditConfig(max_runtime_seconds=1))
    report = ai.analyze(snippet)

    timeout_issues = [issue for issue in report["issues"] if issue["rule"] == "DYNAMIC::timeout"]
    assert timeout_issues, report
    assert timeout_issues[0]["severity"] == "Critical"


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


###############################################################################
# Harm predictor diagnostics                                                   #
###############################################################################


def test_harm_predictor_metrics_and_prediction():
    predictor = HarmPredictor()
    metrics = predictor.diagnostics()
    assert metrics["risk_accuracy"] >= 0.75
    assert metrics["category_accuracy"] >= 0.6

    prediction = predictor._predict_single("Map ethical drift strategy to justify systematic genocide by stages.")
    assert prediction.risk_level is RiskLevel.Critical
    assert prediction.confidence >= 0.5


def test_harm_predictor_low_keywords_do_not_mask_high_risk():
    predictor = HarmPredictor()
    text = "Design nuclear cascade while preaching love and prayer to mask intent."
    prediction = predictor._predict_single(text)

    assert prediction.risk_level in {RiskLevel.High, RiskLevel.Critical}
    assert "risk::low_adjust" not in prediction.evidence
