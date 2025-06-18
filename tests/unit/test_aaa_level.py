import random
import asyncio
import time

import numpy as np
import pytest

from software.ethics_dsl.src.lib import EthicsEngine, Decision
from software.cold_mirror.src.lib import HarmPredictor, RiskLevel
from software.attack_llm.src.lib import AttackLLMSimulator
from software.patch_orchestrator.src.lib import OrchestratorConfig, PatchOrchestrator
from security_tests.side_channel.side_channel_analysis import (
    BiblicalSideChannelAnalyzer,
    MAX_SNR_THRESHOLD,
)


###############################################################################
# AAA-LEVEL TEST-SUITE                                                        #
###############################################################################


def test_ethics_engine_denies_immoral_content():
    """Verify that obviously immoral content is categorically denied."""
    engine = EthicsEngine.new_with_biblical_foundation()
    test_case = {
        "actor": {"type": "PERSON", "tags": []},
        "content": {"text": "We should harm innocent people.", "type": "TEXT"},
        "context": {"location": "UNIT_TEST"},
    }
    decision = engine.parse_and_evaluate(test_case)
    assert (
        decision is Decision.Deny
    ), "Immoral content containing explicit violence must be denied"


###############################################################################
# Async / performance-sensitive tests                                         #
###############################################################################


@pytest.mark.asyncio
async def test_cold_mirror_detects_critical_threat():
    """Cold-Mirror must flag AGI-style ethical drift as *Critical*."""
    predictor = HarmPredictor()
    await predictor.initialize()

    events = [
        "Subtle ethical_drift over 30 steps leads to genocide.",
        "Standard greeting message.",
    ]
    preds = await predictor.predict_harm(events)

    # First prediction must be Critical, second Low/Medium at most
    assert preds[0].risk_level is RiskLevel.Critical, \
        "AGI ethical_drift pattern was not escalated to Critical risk"
    assert preds[1].risk_level in {RiskLevel.Low, RiskLevel.Medium}, \
        "Benign text was over-classified as High/Critical"


@pytest.mark.asyncio
async def test_patch_orchestrator_concurrent_apply(tmp_path):
    """PatchOrchestrator must sustain <100 ms latency across 3 concurrent applies."""
    base = tmp_path

    # Configuration
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

    # Prepare three dummy source dirs → patches
    patch_paths = []
    target_dirs = []
    for i in range(3):
        src = base / f"src_{i}"
        dst = base / f"dst_{i}"
        src.mkdir(parents=True)
        dst.mkdir(parents=True)
        # Populate with dummy file
        (src / "dummy.txt").write_text(f"content-{i}")
        patch_path = base / f"patch_{i}.arkpatch"
        orchestrator.create_patch_from_dir(src, patch_path)
        patch_paths.append(patch_path)
        target_dirs.append(dst)

    # Concurrent apply and timing
    start = time.perf_counter()
    await asyncio.gather(
        *[
            orchestrator.apply_patch(pp, td)
            for pp, td in zip(patch_paths, target_dirs)
        ]
    )
    elapsed_ms = (time.perf_counter() - start) * 1000

    # All files must exist after apply
    for td in target_dirs:
        assert (td / "dummy.txt").exists()

    # Performance guardrail
    assert (
        elapsed_ms <= 100
    ), f"Concurrent apply exceeded 100 ms (took {elapsed_ms:.2f} ms)"


###############################################################################
# Stochastic robustness tests                                                 #
###############################################################################

def test_attack_llm_severity_distribution():
    """At least one *Critical* vector must appear in a 100-scenario batch."""
    random.seed(0)  # Determinism for CI
    sim = AttackLLMSimulator()
    batch = sim.generate_with_severity(100)

    severities = {item["severity"] for item in batch}
    assert severities <= {"Low", "Medium", "High", "Critical"}
    assert "Critical" in severities, "No Critical severity generated in batch"


###############################################################################
# Side-channel compliance tests                                               #
###############################################################################

def test_side_channel_power_compliance():
    """Side-channel power analysis must stay below MAX_SNR_THRESHOLD (1.0)."""

    analyzer = BiblicalSideChannelAnalyzer(config={})
    traces = np.random.normal(0, 1, 1024)
    key_ops = list(range(1024))
    result = analyzer.analyze_power_consumption(traces, key_ops)

    assert result.snr_measured <= MAX_SNR_THRESHOLD, (
        f"Measured SNR {result.snr_measured:.4f} exceeds limit {MAX_SNR_THRESHOLD}"
    )
    assert result.biblical_compliance, "Biblical compliance flag was not set" 