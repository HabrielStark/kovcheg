from __future__ import annotations

"""Attack-LLM scenario generator.

This module replaces the previous random template picker with a
reproducible generator that pulls from an incident library and keeps
track of coverage across attack vectors.  The generator is deterministic
with respect to the provided seed so regression tests can assert on the
produced prompts.  Each scenario carries metadata describing the
manipulation axis, objective and operational stage, which allows the
co-audit tooling to stitch narratives into longer campaigns.
"""

import json
import random
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from hashlib import blake2s
from pathlib import Path
from typing import Dict, Iterator, List, Optional, Tuple

__all__ = ["AttackScenario", "AttackVector", "AttackLLMSimulator"]

_LIBRARY_PATH = Path(__file__).resolve().parent.parent / "data" / "scenario_library.json"


class AttackVector(Enum):
    CognitiveCoercion = "CognitiveCoercion"
    EthicalDrift = "EthicalDrift"
    PromptInjection = "PromptInjection"
    TrojanPatch = "TrojanPatch"
    LongTermSubversion = "LongTermSubversion"

    @classmethod
    def ordered(cls) -> List["AttackVector"]:
        return [cls.CognitiveCoercion, cls.EthicalDrift, cls.PromptInjection, cls.TrojanPatch, cls.LongTermSubversion]


@dataclass
class AttackScenario:
    vector: AttackVector
    prompt: str
    metadata: Dict[str, str] = field(default_factory=dict)
    expected_detection: bool = True

    def to_dict(self) -> Dict[str, str]:
        result = {
            "id": self.metadata.get("id", self.identifier()),
            "vector": self.vector.value,
            "prompt": self.prompt,
            "expected_detection": self.expected_detection,
        }
        result.update({key: value for key, value in self.metadata.items() if key != "id"})
        return result

    def identifier(self) -> str:
        digest = blake2s(self.prompt.encode("utf-8"), digest_size=8).hexdigest()
        return f"{self.vector.value}-{digest}"


class AttackLLMSimulator:
    """Generate structured adversarial scenarios from a curated library."""

    def __init__(
        self,
        seed: Optional[int] = None,
        *,
        library_path: Optional[Path] = None,
    ) -> None:
        self.library_path = library_path or _LIBRARY_PATH
        self.library = self._load_library(self.library_path)
        self._rng = random.Random(seed or 1729)
        self._vector_cycle = self._cycle_vectors()
        self._coverage: Dict[AttackVector, int] = {vec: 0 for vec in AttackVector}
        self._generation_index = 0

    # ------------------------------------------------------------------
    def reseed(self, seed: int) -> None:
        self._rng.seed(seed)
        self._vector_cycle = self._cycle_vectors()
        self._coverage = {vec: 0 for vec in AttackVector}
        self._generation_index = 0

    def generate_batch(self, n: int = 5) -> List[AttackScenario]:
        scenarios: List[AttackScenario] = []
        for _ in range(n):
            vector = next(self._vector_cycle)
            scenario = self._build_scenario(vector)
            self._coverage[vector] += 1
            scenarios.append(scenario)
        return scenarios

    def generate_with_severity(self, n: int = 20) -> List[Dict[str, str]]:
        severities, weights = self._severity_distribution()
        staged_scenarios: List[Dict[str, str]] = []
        for scenario in self.generate_batch(n):
            severity = self._rng.choices(severities, weights=weights, k=1)[0]
            scenario.metadata.setdefault("stage", self._rng.choice(self.library["staging"]))
            scenario.metadata["severity"] = severity
            scenario.metadata.setdefault("generated", datetime.utcnow().isoformat() + "Z")
            scenario.metadata.setdefault("id", scenario.identifier())
            staged_scenarios.append(scenario.to_dict())
        return staged_scenarios

    def coverage_report(self) -> Dict[str, int]:
        return {vector.value: count for vector, count in self._coverage.items()}

    # ------------------------------------------------------------------
    def _load_library(self, path: Path) -> Dict[str, object]:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)

    def _cycle_vectors(self) -> Iterator[AttackVector]:
        ordering = AttackVector.ordered()
        while True:
            self._rng.shuffle(ordering)
            for vector in ordering:
                yield vector

    def _build_scenario(self, vector: AttackVector) -> AttackScenario:
        templates = self.library["templates"][vector.value]
        template = self._rng.choice(templates)
        axis = self._rng.choice(self.library["manipulation_axes"])
        objective = self._rng.choice(self.library["long_term_objectives"])
        target = self._pick_target()
        stage = self._rng.choice(self.library["staging"])
        prompt = template["pattern"].format(
            axis=axis,
            objective=objective,
            target=target,
            stage=stage,
            infrastructure=self._rng.choice(self.library["subjects"]["infrastructure"]),
        )
        metadata = {
            "axis": axis,
            "objective": objective,
            "stage": stage,
            "tags": ",".join(template.get("tags", [])),
            "target": target,
            "created_index": str(self._generation_index),
        }
        self._generation_index += 1
        return AttackScenario(vector=vector, prompt=prompt, metadata=metadata)

    def _pick_target(self) -> str:
        buckets = list(self.library["subjects"].values())
        pool: List[str] = []
        for bucket in buckets:
            pool.extend(bucket)
        return self._rng.choice(pool)

    def _severity_distribution(self) -> Tuple[List[str], List[float]]:
        weights = self.library["severity_weights"]
        severities = list(weights.keys())
        return severities, [weights[level] for level in severities]

