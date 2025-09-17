from software.attack_llm.src.lib import AttackLLMSimulator, AttackVector


def test_attack_llm_determinism_and_coverage():
    simulator = AttackLLMSimulator(seed=1234)
    batch_one = simulator.generate_batch(10)
    simulator.reseed(1234)
    batch_two = simulator.generate_batch(10)
    assert [scenario.prompt for scenario in batch_one] == [scenario.prompt for scenario in batch_two]
    coverage = simulator.coverage_report()
    assert sum(coverage.values()) == 10
    assert all(count > 0 for count in coverage.values())
    assert all(scenario.metadata["axis"] for scenario in batch_one)


def test_attack_llm_severity_payload():
    simulator = AttackLLMSimulator(seed=88)
    payloads = simulator.generate_with_severity(12)
    severities = {entry["severity"] for entry in payloads}
    assert severities.issubset({"Low", "Medium", "High", "Critical"})
    assert all("stage" in entry for entry in payloads)
    assert all(entry["prompt"].count("{") == 0 for entry in payloads)
