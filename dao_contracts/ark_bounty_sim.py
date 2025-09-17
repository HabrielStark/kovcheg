from __future__ import annotations

"""Python simulator for the ArkBounty Solidity contract."""

import random
from dataclasses import dataclass
from typing import Dict, Optional


@dataclass
class BountyRecord:
    bounty_id: int
    creator: str
    reward: int
    deadline: int
    state: str = "Active"
    hunter: Optional[str] = None
    submission_id: Optional[int] = None


@dataclass
class SubmissionRecord:
    submission_id: int
    bounty_id: int
    hunter: str
    uri: str
    score: int = 0
    approved: bool = False


@dataclass
class SimulatorConfig:
    max_reward: int = 100
    max_active_per_hunter: int = 3


class ArkBountySimulator:
    ADMIN = "admin"
    REVIEWER = "reviewer"
    TREASURER = "treasurer"

    def __init__(self, *, config: Optional[SimulatorConfig] = None) -> None:
        self.config = config or SimulatorConfig()
        self.roles: Dict[str, set[str]] = {"deployer": {self.ADMIN, self.TREASURER}}
        self.treasury_balance = 0
        self.locked_rewards = 0
        self.total_paid = 0
        self.next_bounty_id = 1
        self.next_submission_id = 1
        self.bounties: Dict[int, BountyRecord] = {}
        self.submissions: Dict[int, SubmissionRecord] = {}
        self.active_assignments: Dict[str, int] = {}

    # ------------------------------------------------------------------
    def grant_role(self, role: str, account: str, actor: str = "deployer") -> None:
        self._require_role(actor, self.ADMIN)
        self.roles.setdefault(account, set()).add(role)

    def revoke_role(self, role: str, account: str, actor: str = "deployer") -> None:
        self._require_role(actor, self.ADMIN)
        self.roles.setdefault(account, set()).discard(role)

    def deposit(self, amount: int, actor: str = "deployer") -> None:
        self._require_role(actor, self.TREASURER)
        if amount <= 0:
            raise ValueError("amount must be positive")
        self.treasury_balance += amount

    def create_bounty(self, description: str, reward: int, deadline: int, *, actor: str = "deployer") -> int:
        self._require_role(actor, self.ADMIN)
        if not description:
            raise ValueError("description required")
        if reward <= 0 or reward > self.config.max_reward:
            raise ValueError("reward out of bounds")
        if deadline <= 0:
            raise ValueError("deadline required")
        if self.treasury_balance < self.locked_rewards + reward:
            raise ValueError("treasury exhausted")
        bounty_id = self.next_bounty_id
        self.next_bounty_id += 1
        self.bounties[bounty_id] = BountyRecord(
            bounty_id=bounty_id,
            creator=actor,
            reward=reward,
            deadline=deadline,
        )
        self.locked_rewards += reward
        return bounty_id

    def submit(self, bounty_id: int, hunter: str, uri: str) -> int:
        bounty = self._get_bounty(bounty_id)
        if bounty.state != "Active":
            raise ValueError("bounty not accepting submissions")
        if self.active_assignments.get(hunter, 0) >= self.config.max_active_per_hunter:
            raise ValueError("hunter limit reached")
        submission_id = self.next_submission_id
        self.next_submission_id += 1
        self.submissions[submission_id] = SubmissionRecord(
            submission_id=submission_id,
            bounty_id=bounty_id,
            hunter=hunter,
            uri=uri,
        )
        bounty.state = "Submitted"
        bounty.hunter = hunter
        bounty.submission_id = submission_id
        self.active_assignments[hunter] = self.active_assignments.get(hunter, 0) + 1
        return submission_id

    def review(self, bounty_id: int, approve: bool, score: int = 0, *, actor: str = "reviewer") -> None:
        self._require_role(actor, self.REVIEWER)
        bounty = self._get_bounty(bounty_id)
        if bounty.state != "Submitted" or bounty.submission_id is None:
            raise ValueError("no submission to review")
        submission = self.submissions[bounty.submission_id]
        submission.score = score
        submission.approved = approve
        if approve:
            bounty.state = "Approved"
        else:
            bounty.state = "Active"
            self.active_assignments[submission.hunter] -= 1
            bounty.hunter = None
            bounty.submission_id = None

    def payout(self, bounty_id: int, recipient: str, *, actor: str = "deployer") -> None:
        self._require_role(actor, self.TREASURER)
        bounty = self._get_bounty(bounty_id)
        if bounty.state != "Approved" or bounty.submission_id is None:
            raise ValueError("bounty not approved")
        submission = self.submissions[bounty.submission_id]
        if not submission.approved:
            raise ValueError("submission rejected")
        if self.treasury_balance < bounty.reward:
            raise ValueError("treasury insufficient")
        bounty.state = "Paid"
        self.locked_rewards -= bounty.reward
        self.treasury_balance -= bounty.reward
        self.total_paid += bounty.reward
        self.active_assignments[submission.hunter] = max(self.active_assignments.get(submission.hunter, 0) - 1, 0)

    # ------------------------------------------------------------------
    def stress_test(self, rounds: int = 100, seed: int = 1337) -> None:
        rng = random.Random(seed)
        hunters = [f"hunter{i}" for i in range(4)]
        self.grant_role(self.REVIEWER, "reviewer")
        self.deposit(500)
        for _ in range(rounds):
            action = rng.random()
            if action < 0.3:
                reward = rng.randint(10, 40)
                try:
                    self.create_bounty("test", reward, deadline=rng.randint(1, 10))
                except ValueError:
                    pass
            elif action < 0.6 and self.bounties:
                bounty_id = rng.choice(list(self.bounties.keys()))
                hunter = rng.choice(hunters)
                try:
                    self.submit(bounty_id, hunter, "uri")
                except ValueError:
                    pass
            elif action < 0.8 and self.bounties:
                bounty_id = rng.choice(list(self.bounties.keys()))
                try:
                    self.review(bounty_id, approve=rng.random() > 0.5, score=rng.randint(0, 100))
                except ValueError:
                    pass
            else:
                approved = [b for b in self.bounties.values() if b.state == "Approved"]
                if approved:
                    bounty = rng.choice(approved)
                    try:
                        self.payout(bounty.bounty_id, "recipient")
                    except ValueError:
                        pass
            assert self.formal_verify_state()

    def formal_verify_state(self) -> bool:
        treasury = self.treasury_balance
        locked = self.locked_rewards
        paid = self.total_paid
        return all(
            condition
            for condition in [
                treasury >= 0,
                locked >= 0,
                paid >= 0,
                locked <= treasury + paid,
                treasury >= locked,
            ]
        )

    # ------------------------------------------------------------------
    def _require_role(self, account: str, role: str) -> None:
        if role not in self.roles.get(account, set()):
            raise PermissionError(f"{account} lacks role {role}")

    def _get_bounty(self, bounty_id: int) -> BountyRecord:
        if bounty_id not in self.bounties:
            raise KeyError("unknown bounty")
        return self.bounties[bounty_id]

