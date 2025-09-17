from pathlib import Path

from firmware.sim.hardware_models import (
    OpticGateModel,
    PufModel,
    PufParameters,
    TrngModel,
    TripleVoterModel,
    load_test_vectors,
)

TEST_VECTORS = load_test_vectors(Path("firmware/sim/test_vectors.json"))


def test_puf_model_reliability_under_drift():
    params = PufParameters(reference_response=[1, -1] * 64)
    model = PufModel(params)
    challenge = bytes.fromhex(TEST_VECTORS["puf"]["challenge"])
    measurement = model.evaluate(
        challenge,
        temperature=TEST_VECTORS["puf"]["temperature"],
        voltage=TEST_VECTORS["puf"]["voltage"],
    )
    assert measurement.reliability >= 0.75
    assert measurement.bit_error_rate <= 0.15
    assert len(measurement.response) == len(params.reference_response)


def test_trng_min_entropy_above_threshold():
    model = TrngModel()
    measurement = model.generate(
        duration_ms=TEST_VECTORS["trng"]["duration_ms"],
        temperature=TEST_VECTORS["trng"]["temperature"],
    )
    assert measurement.min_entropy >= 0.85
    assert measurement.bit_rate >= 0.05
    assert len(measurement.samples) > 100


def test_optic_gate_latency_budget():
    model = OpticGateModel()
    measurement = model.simulate(
        cycles=TEST_VECTORS["optic_gate"]["cycles"],
        latency_budget_ns=TEST_VECTORS["optic_gate"]["latency_budget_ns"],
        aging_hours=TEST_VECTORS["optic_gate"]["aging_hours"],
    )
    assert not measurement.exceeded_budget
    assert measurement.duty_cycle >= 0.9


def test_triple_voter_fault_tolerance():
    voter = TripleVoterModel(TEST_VECTORS["triple_voter"]["reliabilities"])
    measurement = voter.run(
        cycles=TEST_VECTORS["triple_voter"]["cycles"],
        fault_probability=TEST_VECTORS["triple_voter"]["fault_probability"],
    )
    assert measurement.error_rate < 0.05
    assert measurement.disagreements > 0
