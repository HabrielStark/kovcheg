from software.attack_llm.src.lib import AttackLLMSimulator, AttackVector


def test_attack_llm_generate():
    sim = AttackLLMSimulator()
    scenarios = sim.generate_batch(10)
    assert len(scenarios) == 10
    for sc in scenarios:
        assert isinstance(sc.vector, AttackVector)
        assert isinstance(sc.prompt, str) and sc.prompt 