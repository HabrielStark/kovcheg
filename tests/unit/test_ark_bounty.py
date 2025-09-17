import pytest

from dao_contracts.ark_bounty_sim import ArkBountySimulator


def test_ark_bounty_simulator_flow():
    sim = ArkBountySimulator()
    sim.grant_role(sim.ADMIN, "admin2")
    sim.grant_role(sim.REVIEWER, "auditor")
    sim.deposit(200)
    bounty_id = sim.create_bounty("Investigate optics", reward=60, deadline=10)
    submission_id = sim.submit(bounty_id, "hunter1", "uri://finding")
    sim.review(bounty_id, approve=True, score=95, actor="auditor")
    sim.payout(bounty_id, "hunter1")
    assert sim.treasury_balance == 140
    assert sim.locked_rewards == 0
    assert sim.formal_verify_state()


def test_ark_bounty_stress_test_keeps_invariants():
    sim = ArkBountySimulator()
    sim.stress_test(rounds=25)
    assert sim.formal_verify_state()
