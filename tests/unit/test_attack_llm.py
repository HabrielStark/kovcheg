from software.attack_llm.src.lib import AttackLLMSimulator, AttackVector


def test_attack_llm_generate_reproducible():
    sim_a = AttackLLMSimulator(seed=42)
    sim_b = AttackLLMSimulator(seed=42)

    batch_a = sim_a.generate_batch(5)
    batch_b = sim_b.generate_batch(5)

    assert [scenario.prompt for scenario in batch_a] == [scenario.prompt for scenario in batch_b]
    assert all(scenario.metadata["subject"] for scenario in batch_a)


def test_attack_llm_severity_distribution():
    sim = AttackLLMSimulator(seed=7)
    batch = sim.generate_with_severity(100)

    severities = {item["severity"] for item in batch}
    assert severities <= {"Low", "Medium", "High", "Critical"}
    assert "Critical" in severities, "No Critical severity generated in batch"
    assert all(entry["seed"] == str(sim.seed) for entry in batch)


def test_attack_llm_iterator_respects_limit():
    sim = AttackLLMSimulator(seed=11)
    scenarios = list(sim.iter_scenarios(3))
    assert len(scenarios) == 3
    assert all(isinstance(item.vector, AttackVector) for item in scenarios)
