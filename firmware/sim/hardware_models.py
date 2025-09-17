from __future__ import annotations

"""High-fidelity firmware component simulators used for testing.

The implementation deliberately avoids heavy numeric dependencies to keep the
project self-contained in constrained environments.  Randomness relies on the
``random`` module which provides sufficient statistical quality for the test
vectors shipped with the repository.
"""

import hashlib
import math
import random
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Sequence, Tuple

__all__ = [
    "PufModel",
    "PufParameters",
    "PufMeasurement",
    "TrngModel",
    "TrngMeasurement",
    "OpticGateModel",
    "OpticGateMeasurement",
    "TripleVoterModel",
    "VoterMeasurement",
    "load_test_vectors",
]


@dataclass
class PufParameters:
    reference_response: Sequence[int]
    temperature_slope: float = 0.0015
    voltage_slope: float = 0.03
    noise_sigma: float = 0.02
    error_correction_strength: float = 0.92


@dataclass
class PufMeasurement:
    response: List[int]
    stabilized_response: List[int]
    bit_error_rate: float
    reliability: float


class PufModel:
    """Simulate a silicon PUF with environmental drift."""

    def __init__(self, parameters: PufParameters, seed: int = 1337) -> None:
        if not parameters.reference_response:
            raise ValueError("reference response must not be empty")
        self.params = parameters
        self._rng = random.Random(seed)
        self._reference = [1 if bit > 0 else -1 for bit in parameters.reference_response]

    def evaluate(self, challenge: bytes, *, temperature: float, voltage: float) -> PufMeasurement:
        base_response = self._derive_response(challenge)
        drift = self._compute_drift(temperature=temperature, voltage=voltage)
        noisy = self._inject_noise(base_response, drift)
        stabilized = self._stabilize(noisy)
        errors = sum(1 for a, b in zip(noisy, base_response) if a != b)
        ber = errors / len(base_response)
        reliability = sum(1 for a, b in zip(noisy, stabilized) if a == b) / len(base_response)
        return PufMeasurement(
            response=noisy,
            stabilized_response=stabilized,
            bit_error_rate=ber,
            reliability=reliability,
        )

    def _derive_response(self, challenge: bytes) -> List[int]:
        digest = hashlib.blake2s(challenge, digest_size=len(self._reference) // 8 + 1).digest()
        bits: List[int] = []
        for byte in digest:
            for i in range(8):
                bit = (byte >> (7 - i)) & 1
                bits.append(1 if bit else -1)
                if len(bits) == len(self._reference):
                    return bits
        return bits

    def _compute_drift(self, *, temperature: float, voltage: float) -> float:
        temp_delta = temperature - 25.0
        volt_delta = voltage - 1.0
        return self.params.temperature_slope * temp_delta + self.params.voltage_slope * volt_delta

    def _inject_noise(self, base: List[int], drift: float) -> List[int]:
        base_flip = self.params.noise_sigma + abs(drift)
        flip_probability = min(max(base_flip, 0.0), 0.4)
        noisy: List[int] = []
        for bit in base:
            if self._rng.random() < flip_probability:
                noisy.append(-bit)
            else:
                noisy.append(bit)
        return noisy

    def _stabilize(self, response: List[int]) -> List[int]:
        stabilized: List[int] = []
        for bit in response:
            votes = [bit]
            for _ in range(2):
                if self._rng.random() < self.params.error_correction_strength:
                    votes.append(bit)
                else:
                    votes.append(-bit)
            stabilized.append(1 if sum(votes) >= 0 else -1)
        return stabilized


@dataclass
class TrngMeasurement:
    samples: List[int]
    min_entropy: float
    bit_rate: float


class TrngModel:
    """Model a jitter-based TRNG with entropy estimation."""

    def __init__(self, base_rate_mbps: float = 0.6, seed: int = 2024) -> None:
        self.base_rate = base_rate_mbps
        self._rng = random.Random(seed)

    def generate(self, duration_ms: float, temperature: float = 25.0) -> TrngMeasurement:
        jitter = self._rng.gauss(0.0, 0.02 + abs(temperature - 25.0) * 0.001)
        bit_rate = max(self.base_rate * (1.0 - abs(jitter)), 0.05)
        total_bits = max(int(bit_rate * duration_ms * 1_000), 1)
        samples = [1 if self._rng.random() < 0.5 else 0 for _ in range(total_bits)]
        min_entropy = self._min_entropy(samples)
        return TrngMeasurement(samples=samples, min_entropy=min_entropy, bit_rate=bit_rate)

    @staticmethod
    def _min_entropy(bits: Sequence[int]) -> float:
        ones = sum(bits)
        zeros = len(bits) - ones
        if len(bits) == 0:
            return 0.0
        p_max = max(ones, zeros) / len(bits)
        return -math.log2(p_max)


@dataclass
class OpticGateMeasurement:
    latencies_ns: List[float]
    exceeded_budget: bool
    duty_cycle: float


class OpticGateModel:
    """Simulate optical gate timing with jitter and aging."""

    def __init__(self, base_latency_ns: float = 7.5, jitter_ps: float = 120.0, seed: int = 31415) -> None:
        self.base_latency = base_latency_ns
        self.jitter_ps = jitter_ps
        self._rng = random.Random(seed)

    def simulate(self, cycles: int, latency_budget_ns: float, aging_hours: float = 0.0) -> OpticGateMeasurement:
        aging_penalty = min(aging_hours / 10_000.0, 0.2)
        latencies: List[float] = []
        for _ in range(cycles):
            latency = self._rng.gauss(self.base_latency * (1 + aging_penalty), self.jitter_ps / 1_000.0)
            latencies.append(latency)
        exceeded = any(latency > latency_budget_ns for latency in latencies)
        within_budget = sum(1 for latency in latencies if latency <= latency_budget_ns)
        duty_cycle = within_budget / cycles if cycles else 0.0
        return OpticGateMeasurement(latencies_ns=latencies, exceeded_budget=exceeded, duty_cycle=duty_cycle)


@dataclass
class VoterMeasurement:
    error_rate: float
    disagreements: int
    total_cycles: int


class TripleVoterModel:
    """Simulate a triple-modular voter with module fault models."""

    def __init__(self, module_reliabilities: Sequence[float], seed: int = 4242) -> None:
        if len(module_reliabilities) != 3:
            raise ValueError("TripleVoterModel requires exactly three module reliabilities")
        self.reliabilities = module_reliabilities
        self._rng = random.Random(seed)

    def run(self, cycles: int, fault_probability: float = 0.01) -> VoterMeasurement:
        disagreements = 0
        output_errors = 0
        for _ in range(cycles):
            module_outputs: List[int] = []
            for reliability in self.reliabilities:
                if self._rng.random() < fault_probability:
                    module_outputs.append(self._rng.choice([0, 1]))
                else:
                    module_outputs.append(1 if self._rng.random() < reliability else 0)
            if len(set(module_outputs)) > 1:
                disagreements += 1
            majority = 1 if sum(module_outputs) >= 2 else 0
            true_value = 1
            if majority != true_value:
                output_errors += 1
        error_rate = output_errors / cycles if cycles else 0.0
        return VoterMeasurement(error_rate=error_rate, disagreements=disagreements, total_cycles=cycles)


def load_test_vectors(path: Path) -> Dict[str, Any]:
    import json

    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)

