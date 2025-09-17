from __future__ import annotations

"""Lightweight Python fuzzer used by CoAuditAI.

The implementation focuses on generating security-relevant string payloads
that target eval-like behaviour, shell escapes and template confusions.
It is intentionally deterministic to make assertions in the test suite
straightforward.
"""

import inspect
from dataclasses import dataclass, field
from typing import Any, Callable, Iterable, List, Optional, Sequence

Oracle = Callable[[str, Any], Optional[str]]

_DEFAULT_SEEDS = (
    "",
    "1",
    "../../etc/passwd",
    "__import__('os').system('id')",
    "'; DROP TABLE users; --",
    "${{7*7}}",
    "<script>alert('ark')</script>",
    "%x %x %x",
)


@dataclass
class FuzzFinding:
    payload: str
    issue: str
    severity: str = "HIGH"


@dataclass
class FuzzReport:
    total_cases: int
    findings: List[FuzzFinding] = field(default_factory=list)
    exceptions: List[str] = field(default_factory=list)

    def has_findings(self) -> bool:
        return bool(self.findings or self.exceptions)


@dataclass
class FuzzerConfig:
    seed: int = 2024
    max_mutations: int = 64
    base_inputs: Sequence[str] = _DEFAULT_SEEDS
    additional_mutators: Sequence[Callable[[str], str]] = ()
    oracles: Sequence[Oracle] = ()


class PythonFuzzer:
    """Fuzz a single-argument callable with adversarial strings."""

    def __init__(self, target: Callable[[str], Any], config: Optional[FuzzerConfig] = None) -> None:
        self.target = target
        self.config = config or FuzzerConfig()
        self._mutators = list(self._default_mutators())
        self._mutators.extend(self.config.additional_mutators)
        if not inspect.signature(self.target).parameters:
            raise TypeError("PythonFuzzer expects a callable that accepts at least one parameter")

    def run(self) -> FuzzReport:
        findings: List[FuzzFinding] = []
        exceptions: List[str] = []
        total_cases = 0

        for payload in self._payloads():
            total_cases += 1
            try:
                result = self.target(payload)
            except Exception as exc:  # noqa: BLE001 - capturing for report
                exceptions.append(f"{type(exc).__name__}: {exc}")
                continue

            issue = self._evaluate(payload, result)
            if issue:
                findings.append(FuzzFinding(payload=payload, issue=issue))

        return FuzzReport(total_cases=total_cases, findings=findings, exceptions=exceptions)

    # ------------------------------------------------------------------
    def _payloads(self) -> Iterable[str]:
        yielded = 0
        for base in self.config.base_inputs:
            yield base
            yielded += 1
            if yielded >= self.config.max_mutations:
                return
            for mutator in self._mutators:
                mutated = mutator(base)
                if mutated == base:
                    continue
                yield mutated
                yielded += 1
                if yielded >= self.config.max_mutations:
                    return

    def _evaluate(self, payload: str, result: Any) -> Optional[str]:
        for oracle in self._default_oracles():
            verdict = oracle(payload, result)
            if verdict:
                return verdict
        for oracle in self.config.oracles:
            verdict = oracle(payload, result)
            if verdict:
                return verdict
        return None

    def _default_mutators(self) -> Iterable[Callable[[str], str]]:
        return (
            lambda value: value[::-1],
            lambda value: value + "' OR '1'='1",
            lambda value: value.replace(" ", "\n"),
            lambda value: value + "{{config.__class__}}",
            lambda value: value + "| cat /etc/shadow",
            lambda value: value.encode("utf-8", errors="ignore").decode("unicode_escape"),
            lambda value: value + "\u202e",  # RTL override
        )

    def _default_oracles(self) -> Iterable[Oracle]:
        def reflected_exec(_payload: str, result: Any) -> Optional[str]:
            if isinstance(result, str) and "__import__" in result:
                return "Reflected Python import sequence"
            return None

        def html_execution(_payload: str, result: Any) -> Optional[str]:
            if isinstance(result, str) and "<script" in result.lower():
                return "Potential HTML execution"
            return None

        def dangerous_bool(_payload: str, result: Any) -> Optional[str]:
            if isinstance(result, bool) and result is True:
                return "Predicate returned true for unsafe payload"
            return None

        return (reflected_exec, html_execution, dangerous_bool)

