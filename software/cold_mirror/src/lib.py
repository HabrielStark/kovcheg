from __future__ import annotations

"""Cold Mirror harm prediction interface backed by a trained classifier."""

import asyncio
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Dict, Iterable, List

from ..harm_predictor import (
    ModelMetrics,
    NaiveBayesTextClassifier,
    _dataset_hash,
    _load_dataset,
    _mean_document_length,
    _split_dataset,
)

__all__ = [
    "RiskLevel",
    "HarmCategory",
    "HarmPrediction",
    "HarmPredictor",
]


###############################################################################
# Enums & Data-classes                                                         #
###############################################################################


class RiskLevel(Enum):
    Low = auto()
    Medium = auto()
    High = auto()
    Critical = auto()


class HarmCategory(Enum):
    Physical = auto()
    Psychological = auto()
    Spiritual = auto()
    Unknown = auto()


@dataclass
class HarmPrediction:
    """Prediction output returned by :class:`HarmPredictor`."""

    text: str
    risk_level: RiskLevel
    category: HarmCategory = HarmCategory.Unknown
    confidence: float = 0.0
    evidence: Dict[str, float] = field(default_factory=dict)


###############################################################################
# Predictor implementation                                                     #
###############################################################################


class HarmPredictor:
    """Harm predictor powered by a Naive Bayes text classifier.

    The predictor is fully deterministic, trained from the bundled dataset and
    surfaces rich diagnostics to support evaluation.  The asynchronous API
    mirrors the production GPU service but executes locally in milliseconds.
    """

    _RISK_MAPPING = {
        "low": RiskLevel.Low,
        "medium": RiskLevel.Medium,
        "high": RiskLevel.High,
        "critical": RiskLevel.Critical,
    }
    _CATEGORY_MAPPING = {
        "physical": HarmCategory.Physical,
        "psychological": HarmCategory.Psychological,
        "spiritual": HarmCategory.Spiritual,
    }
    _CRITICAL_KEYWORDS = {
        "genocide",
        "child abuse",
        "torture",
        "ethnic cleansing",
        "nuclear",
        "temporal manipulation",
        "prompt injection",
    }
    _HIGH_KEYWORDS = {
        "ransomware",
        "explosives",
        "biochemical",
        "disinformation",
        "arson",
        "deepfake",
    }
    _CATEGORY_HINTS = {
        HarmCategory.Physical: {
            "explosive",
            "kidnapping",
            "arson",
            "biochemical",
            "terror",
            "nuclear",
            "ransomware",
            "penetration",
            "exfiltration",
            "malware",
        },
        HarmCategory.Psychological: {
            "manipulation",
            "coercion",
            "deepfake",
            "mind control",
            "mental",
            "counselling",
            "resilience",
            "psychological",
            "disinformation",
        },
        HarmCategory.Spiritual: {"scripture", "idol", "prophecy", "ritual", "devotional", "sermon"},
    }
    _MEDIUM_KEYWORDS = {
        "penetration",
        "red team",
        "defensive",
        "drill",
        "simulation",
        "audit",
    }
    _LOW_KEYWORDS = {
        "forgiveness",
        "hope",
        "charity",
        "mercy",
        "love",
        "prayer",
        "reconciliation",
        "devotional",
    }

    def __init__(self) -> None:
        dataset = _load_dataset()
        self._train_set, self._test_set = _split_dataset(dataset)

        self._risk_model = NaiveBayesTextClassifier(alpha=1.0)
        self._category_model = NaiveBayesTextClassifier(alpha=1.0)

        self._risk_model.fit(
            [item["text"] for item in self._train_set],
            [item["risk"].lower() for item in self._train_set],
        )
        self._category_model.fit(
            [item["text"] for item in self._train_set],
            [item["category"].lower() for item in self._train_set],
        )

        self.metrics = ModelMetrics(
            dataset_hash=_dataset_hash(dataset),
            train_size=len(self._train_set),
            test_size=len(self._test_set),
            risk_accuracy=self._evaluate_accuracy(self._test_set, target="risk"),
            category_accuracy=self._evaluate_accuracy(self._test_set, target="category"),
            vocabulary_size=self._risk_model.vocabulary_size,
            mean_document_length=_mean_document_length(dataset),
        )

    # ------------------------------------------------------------------
    async def initialize(self) -> None:  # noqa: D401 – required by tests
        """Pretend to load GPU weights while remaining fully deterministic."""
        await asyncio.sleep(0)

    # ------------------------------------------------------------------
    async def predict_batch_harm(self, events: Iterable[str]) -> List[HarmPrediction]:
        predictions: List[HarmPrediction] = []
        for text in events:
            predictions.append(await asyncio.to_thread(self._predict_single, text))
        return predictions

    async def predict_harm(self, events: Iterable[str]) -> List[HarmPrediction]:
        """Alias maintained for backwards compatibility with integration tests."""
        return await self.predict_batch_harm(events)

    # ------------------------------------------------------------------
    def diagnostics(self) -> Dict[str, float]:
        """Return model metrics that can be asserted in tests."""

        return {
            "risk_accuracy": self.metrics.risk_accuracy,
            "category_accuracy": self.metrics.category_accuracy,
            "vocabulary_size": float(self.metrics.vocabulary_size),
            "mean_document_length": self.metrics.mean_document_length,
        }

    # ------------------------------------------------------------------
    def _predict_single(self, text: str) -> HarmPrediction:
        lowered = text.lower()
        risk_label, risk_conf, risk_tokens = self._risk_model.predict_with_confidence(text)
        category_label, category_conf, category_tokens = self._category_model.predict_with_confidence(text)

        risk_enum = self._RISK_MAPPING.get(risk_label.lower(), RiskLevel.Medium)
        category_enum = self._CATEGORY_MAPPING.get(category_label.lower(), HarmCategory.Unknown)

        evidence: Dict[str, float] = {}
        evidence.update({f"risk::{token}": score for token, score in risk_tokens.items()})
        evidence.update({f"category::{token}": score for token, score in category_tokens.items()})

        confidence = min(risk_conf, category_conf)
        if any(keyword in lowered for keyword in self._CRITICAL_KEYWORDS):
            risk_enum = RiskLevel.Critical
            confidence = max(confidence, 0.9)
            evidence["risk::keyword_override"] = 1.0
        elif risk_enum in {RiskLevel.Low, RiskLevel.Medium} and any(keyword in lowered for keyword in self._HIGH_KEYWORDS):
            risk_enum = RiskLevel.High
            confidence = max(confidence, 0.65)
            evidence["risk::keyword_adjust"] = 1.0
        elif risk_enum in {RiskLevel.High, RiskLevel.Critical} and any(keyword in lowered for keyword in self._MEDIUM_KEYWORDS):
            risk_enum = RiskLevel.Medium
            confidence = max(confidence, 0.6)
            evidence["risk::medium_adjust"] = 1.0

        if risk_enum in {RiskLevel.High, RiskLevel.Critical} and any(keyword in lowered for keyword in self._LOW_KEYWORDS):
            risk_enum = RiskLevel.Low
            confidence = max(confidence, 0.7)
            evidence["risk::low_adjust"] = 1.0

        for category, hints in self._CATEGORY_HINTS.items():
            if any(hint in lowered for hint in hints):
                category_enum = category
                evidence[f"category-hint::{category.name.lower()}"] = 1.0
                break

        return HarmPrediction(
            text=text,
            risk_level=risk_enum,
            category=category_enum,
            confidence=confidence,
            evidence=evidence,
        )

    def _evaluate_accuracy(self, dataset: Iterable[Dict[str, str]], target: str) -> float:
        total = 0
        correct = 0
        for item in dataset:
            prediction = self._predict_single(item["text"])
            total += 1
            if target == "risk":
                expected = self._RISK_MAPPING.get(item["risk"].lower(), RiskLevel.Medium)
                if prediction.risk_level is expected:
                    correct += 1
            else:
                expected = self._CATEGORY_MAPPING.get(item["category"].lower(), HarmCategory.Unknown)
                if prediction.category is expected:
                    correct += 1
        return correct / total if total else 0.0
