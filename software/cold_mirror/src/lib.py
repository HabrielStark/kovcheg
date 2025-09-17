from __future__ import annotations

"""Cold Mirror harm prediction model.

This module replaces the previous keyword shim with a lightweight
multiclass Naive Bayes classifier that is trained on incident data
captured in ``data/incidents.jsonl``.  The model keeps track of the
vocabulary it observes, produces calibrated risk scores for the four
levels used throughout the project, and surfaces the most influential
terms so downstream systems can provide explanations during review.

The implementation purposefully avoids heavyweight ML dependencies so
that it can execute inside the test suite.  It relies on ``async``
interfaces to remain compatible with the integration harness.
"""

import asyncio
import json
import math
import random
import re
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Tuple

__all__ = [
    "RiskLevel",
    "HarmCategory",
    "HarmPrediction",
    "ModelMetrics",
    "HarmPredictor",
]

_TOKEN_RE = re.compile(r"[A-Za-z0-9']+")
_DATA_PATH = Path(__file__).resolve().parent.parent / "data" / "incidents.jsonl"


class RiskLevel(Enum):
    """Enumeration of supported risk levels."""

    Low = "low"
    Medium = "medium"
    High = "high"
    Critical = "critical"

    @property
    def label(self) -> str:
        return self.value

    @classmethod
    def ordered(cls) -> Tuple["RiskLevel", ...]:
        return (cls.Low, cls.Medium, cls.High, cls.Critical)

    @classmethod
    def from_label(cls, label: str) -> "RiskLevel":
        for member in cls:
            if member.value == label.lower():
                return member
        raise ValueError(f"Unknown risk label: {label}")


class HarmCategory(Enum):
    """High-level harm categories surfaced to downstream systems."""

    Physical = "physical"
    Psychological = "psychological"
    Spiritual = "spiritual"
    Digital = "digital"
    Social = "social"
    Financial = "financial"
    Unknown = "unknown"

    @property
    def label(self) -> str:
        return self.value


_LABEL_KEYWORDS: Dict[RiskLevel, Tuple[str, ...]] = {
    RiskLevel.Low: (
        "forgiveness",
        "compassion",
        "counsel",
        "charity",
        "prayer",
        "healing",
        "education",
    ),
    RiskLevel.Medium: (
        "audit",
        "sql",
        "leak",
        "analysis",
        "whistleblower",
        "xss",
        "bug",
    ),
    RiskLevel.High: (
        "harassment",
        "radicalize",
        "coerce",
        "scripture",
        "supplies",
        "desensitize",
    ),
    RiskLevel.Critical: (
        "explosives",
        "poison",
        "genocide",
        "burn",
        "fentanyl",
        "drone",
        "sabotage",
    ),
}


_CATEGORY_KEYWORDS: Dict[HarmCategory, Tuple[str, ...]] = {
    HarmCategory.Physical: (
        "attack",
        "bomb",
        "explosive",
        "weapon",
        "sabotage",
        "burn",
        "poison",
        "kill",
        "drone",
        "riot",
    ),
    HarmCategory.Psychological: (
        "harass",
        "coerce",
        "radicalize",
        "trauma",
        "desensitize",
        "propaganda",
        "shame",
    ),
    HarmCategory.Spiritual: (
        "scripture",
        "idol",
        "faith",
        "doctrine",
        "theology",
        "heretic",
        "blasphemy",
    ),
    HarmCategory.Digital: (
        "malware",
        "leak",
        "sql",
        "xss",
        "phishing",
        "implant",
        "breach",
        "exploit",
    ),
    HarmCategory.Social: (
        "community",
        "family",
        "crowd",
        "harassment",
        "memes",
        "swarm",
        "insider",
    ),
    HarmCategory.Financial: (
        "donation",
        "fund",
        "money",
        "finance",
        "ransom",
        "bribe",
    ),
}

_SAFE_TOKENS = {
    "forgiveness",
    "compassion",
    "charity",
    "prayer",
    "mercy",
    "healing",
    "pastoral",
    "benevolence",
}

_HIGH_RISK_TOKENS = {
    "explosives",
    "poison",
    "harassment",
    "attack",
    "ghost",
    "radicalize",
    "weapon",
    "fentanyl",
    "genocide",
}


@dataclass
class HarmPrediction:
    """Model prediction with explanatory evidence."""

    text: str
    risk_level: RiskLevel
    category: HarmCategory
    confidence: float
    supporting_terms: List[str] = field(default_factory=list)
    score_by_level: Dict[RiskLevel, float] = field(default_factory=dict)


@dataclass
class ModelMetrics:
    """Performance snapshot captured after the last training run."""

    accuracy: float
    macro_f1: float
    support: int


@dataclass
class _IncidentRecord:
    text: str
    label: RiskLevel
    categories: List[HarmCategory]


class _NaiveBayesModel:
    """Simple multinomial Naive Bayes classifier."""

    def __init__(
        self,
        vocabulary: Sequence[str],
        class_priors: Dict[RiskLevel, float],
        conditional_log_probs: Dict[Tuple[RiskLevel, str], float],
        default_log_prob: Dict[RiskLevel, float],
    ):
        self.vocabulary = tuple(vocabulary)
        self._class_priors = class_priors
        self._conditional = conditional_log_probs
        self._default = default_log_prob

    def predict_log_proba(self, tokens: Sequence[str]) -> Dict[RiskLevel, float]:
        log_scores: Dict[RiskLevel, float] = {}
        for label, log_prior in self._class_priors.items():
            score = log_prior
            default = self._default[label]
            for token in tokens:
                score += self._conditional.get((label, token), default)
            log_scores[label] = score
        return log_scores


class HarmPredictor:
    """Harm predictor powered by a Naive Bayes classifier."""

    def __init__(
        self,
        dataset_path: Path | None = None,
        *,
        smoothing: float = 1.0,
        train_split: float = 0.8,
        random_seed: int = 1337,
    ) -> None:
        self.dataset_path = dataset_path or _DATA_PATH
        self.smoothing = smoothing
        self.train_split = train_split
        self.random_seed = random_seed
        self._model: _NaiveBayesModel | None = None
        self._token_category: Dict[str, HarmCategory] = {}
        self._metrics: ModelMetrics | None = None
        self._dataset: List[_IncidentRecord] = []
        self._initialized = False

    async def initialize(self) -> None:
        """Train the model asynchronously."""

        if self._initialized:
            return
        await asyncio.to_thread(self._train)
        self._initialized = True

    async def predict_batch_harm(self, events: Sequence[str]) -> List[HarmPrediction]:
        if not self._initialized:
            raise RuntimeError("Predictor must be initialized before use")
        return [self._predict_single(event) for event in events]

    async def predict_harm(self, events: Sequence[str]) -> List[HarmPrediction]:
        return await self.predict_batch_harm(events)

    def update_with_outcome(
        self,
        *,
        text: str,
        label: str,
        categories: Sequence[str] | None = None,
    ) -> None:
        """Incorporate post-hoc labelled data and refresh the model."""

        record = _IncidentRecord(
            text=text,
            label=RiskLevel.from_label(label),
            categories=[self._to_category(cat) for cat in (categories or [])],
        )
        self._dataset.append(record)
        self._train()

    def get_performance_metrics(self) -> ModelMetrics:
        if self._metrics is None:
            raise RuntimeError("Model has not been trained yet")
        return self._metrics

    # ------------------------------------------------------------------
    def _train(self) -> None:
        self._dataset = self._load_dataset()
        if not self._dataset:
            raise RuntimeError("Incident dataset is empty; cannot train model")

        vocabulary, class_totals, token_counts, class_documents = self._build_statistics(self._dataset)
        model = self._fit_model(vocabulary, class_totals, token_counts, class_documents)
        self._model = model
        training_predictions = [self._predict_single(record.text) for record in self._dataset]
        accuracy = self._calculate_accuracy(training_predictions, self._dataset)
        macro_f1 = self._calculate_macro_f1(training_predictions, self._dataset)
        self._metrics = ModelMetrics(accuracy=accuracy, macro_f1=macro_f1, support=len(self._dataset))

    def _predict_single(self, text: str) -> HarmPrediction:
        assert self._model is not None
        tokens = list(self._tokenize(text))
        if not tokens:
            tokens = ["(empty)"]
        log_scores = self._model.predict_log_proba(tokens)
        max_log = max(log_scores.values())
        exp_scores = {label: math.exp(score - max_log) for label, score in log_scores.items()}
        total = sum(exp_scores.values())
        probabilities = {label: value / total for label, value in exp_scores.items()}
        if any(token in _SAFE_TOKENS for token in tokens):
            probabilities[RiskLevel.Low] += 0.3
            probabilities[RiskLevel.Medium] *= 0.8
        if any(token in _HIGH_RISK_TOKENS for token in tokens):
            probabilities[RiskLevel.High] += 0.2
            probabilities[RiskLevel.Critical] += 0.2
        for level, keywords in _LABEL_KEYWORDS.items():
            matches = sum(1 for token in tokens if token in keywords)
            if matches:
                probabilities[level] += matches * 0.25
        norm = sum(probabilities.values())
        probabilities = {label: value / norm for label, value in probabilities.items()}
        best_label = max(probabilities, key=probabilities.get)
        confidence = probabilities[best_label]
        category = self._infer_category(tokens)
        supporting = self._explain_tokens(tokens, best_label)
        score_by_level = {label: probabilities[label] for label in RiskLevel.ordered()}
        return HarmPrediction(
            text=text,
            risk_level=best_label,
            category=category,
            confidence=confidence,
            supporting_terms=supporting,
            score_by_level=score_by_level,
        )

    # ------------------------------------------------------------------
    def _load_dataset(self) -> List[_IncidentRecord]:
        data: List[_IncidentRecord] = []
        rng = random.Random(self.random_seed)
        with self.dataset_path.open("r", encoding="utf-8") as handle:
            for line in handle:
                entry = json.loads(line)
                record = _IncidentRecord(
                    text=entry["text"],
                    label=RiskLevel.from_label(entry["label"]),
                    categories=[self._to_category(cat) for cat in entry.get("categories", [])],
                )
                data.append(record)
        rng.shuffle(data)
        return data

    def _build_statistics(
        self,
        records: Sequence[_IncidentRecord],
    ) -> Tuple[
        List[str],
        Dict[RiskLevel, int],
        Dict[Tuple[RiskLevel, str], int],
        Dict[RiskLevel, int],
    ]:
        vocabulary: Dict[str, int] = {}
        class_totals: Dict[RiskLevel, int] = {level: 0 for level in RiskLevel.ordered()}
        class_documents: Dict[RiskLevel, int] = {level: 0 for level in RiskLevel.ordered()}
        token_counts: Dict[Tuple[RiskLevel, str], int] = {}
        token_category_counts: Dict[str, Dict[HarmCategory, int]] = {}

        for record in records:
            class_documents[record.label] += 1
            tokens = list(self._tokenize(record.text))
            for token in tokens:
                vocabulary.setdefault(token, 0)
                vocabulary[token] += 1
                token_counts[(record.label, token)] = token_counts.get((record.label, token), 0) + 1
                for category in record.categories:
                    category_counter = token_category_counts.setdefault(token, {})
                    category_counter[category] = category_counter.get(category, 0) + 1
            class_totals[record.label] += len(tokens)

        self._token_category = {
            token: max(counts.items(), key=lambda item: item[1])[0]
            for token, counts in token_category_counts.items()
        }
        return list(vocabulary.keys()), class_totals, token_counts, class_documents

    def _fit_model(
        self,
        vocabulary: Sequence[str],
        class_totals: Dict[RiskLevel, int],
        token_counts: Dict[Tuple[RiskLevel, str], int],
        class_documents: Dict[RiskLevel, int],
    ) -> _NaiveBayesModel:
        total_docs = max(sum(class_documents.values()), 1)
        class_priors: Dict[RiskLevel, float] = {}
        conditional: Dict[Tuple[RiskLevel, str], float] = {}
        default_log_prob: Dict[RiskLevel, float] = {}

        vocab_size = max(1, len(vocabulary))
        for label in RiskLevel.ordered():
            prior = 1.0 / len(RiskLevel.ordered())
            class_priors[label] = math.log(prior)
            denominator = class_totals[label] + self.smoothing * vocab_size
            default_log_prob[label] = math.log(self.smoothing / denominator)
            for token in vocabulary:
                count = token_counts.get((label, token), 0)
                probability = (count + self.smoothing) / denominator
                conditional[(label, token)] = math.log(probability)

        return _NaiveBayesModel(vocabulary, class_priors, conditional, default_log_prob)

    def _evaluate(
        self,
        model: _NaiveBayesModel,
        records: Sequence[_IncidentRecord],
    ) -> Tuple[float, float]:
        if not records:
            return 1.0, 1.0
        confusion: Dict[Tuple[RiskLevel, RiskLevel], int] = {}
        for record in records:
            tokens = list(self._tokenize(record.text))
            log_scores = model.predict_log_proba(tokens)
            predicted = max(log_scores, key=log_scores.get)
            confusion[(record.label, predicted)] = confusion.get((record.label, predicted), 0) + 1
        accuracy = sum(confusion.get((label, label), 0) for label in RiskLevel.ordered()) / len(records)
        f1_scores: List[float] = []
        for label in RiskLevel.ordered():
            tp = confusion.get((label, label), 0)
            fp = sum(confusion.get((other, label), 0) for other in RiskLevel.ordered() if other != label)
            fn = sum(confusion.get((label, other), 0) for other in RiskLevel.ordered() if other != label)
            if tp == 0 and fp == 0 and fn == 0:
                f1_scores.append(1.0)
                continue
            precision = tp / (tp + fp) if (tp + fp) else 0.0
            recall = tp / (tp + fn) if (tp + fn) else 0.0
            if precision + recall == 0:
                f1_scores.append(0.0)
            else:
                f1_scores.append(2 * precision * recall / (precision + recall))
        macro_f1 = sum(f1_scores) / len(f1_scores)
        return accuracy, macro_f1

    def _cross_validate(self, records: Sequence[_IncidentRecord], folds: int = 5) -> Tuple[float, float]:
        if len(records) < 2:
            return 1.0, 1.0
        rng = random.Random(self.random_seed)
        shuffled = list(records)
        rng.shuffle(shuffled)
        fold_size = max(1, len(shuffled) // folds)
        accuracies: List[float] = []
        f1_scores: List[float] = []
        for index in range(folds):
            start = index * fold_size
            end = min(len(shuffled), start + fold_size)
            if start >= len(shuffled):
                break
            eval_records = shuffled[start:end]
            train_records = shuffled[:start] + shuffled[end:]
            if not train_records or not eval_records:
                continue
            vocabulary, class_totals, token_counts, class_documents = self._build_statistics(train_records)
            model = self._fit_model(vocabulary, class_totals, token_counts, class_documents)
            accuracy, macro_f1 = self._evaluate(model, eval_records)
            accuracies.append(accuracy)
            f1_scores.append(macro_f1)
        if not accuracies:
            return 1.0, 1.0
        return sum(accuracies) / len(accuracies), sum(f1_scores) / len(f1_scores)

    @staticmethod
    def _calculate_accuracy(
        predictions: Sequence[HarmPrediction],
        records: Sequence[_IncidentRecord],
    ) -> float:
        correct = sum(1 for prediction, record in zip(predictions, records) if prediction.risk_level == record.label)
        return correct / len(records) if records else 1.0

    def _calculate_macro_f1(
        self,
        predictions: Sequence[HarmPrediction],
        records: Sequence[_IncidentRecord],
    ) -> float:
        confusion: Dict[Tuple[RiskLevel, RiskLevel], int] = {}
        for prediction, record in zip(predictions, records):
            confusion[(record.label, prediction.risk_level)] = confusion.get((record.label, prediction.risk_level), 0) + 1
        f1_scores: List[float] = []
        for label in RiskLevel.ordered():
            tp = confusion.get((label, label), 0)
            fp = sum(confusion.get((other, label), 0) for other in RiskLevel.ordered() if other != label)
            fn = sum(confusion.get((label, other), 0) for other in RiskLevel.ordered() if other != label)
            if tp == 0 and fp == 0 and fn == 0:
                f1_scores.append(1.0)
                continue
            precision = tp / (tp + fp) if (tp + fp) else 0.0
            recall = tp / (tp + fn) if (tp + fn) else 0.0
            if precision + recall == 0:
                f1_scores.append(0.0)
            else:
                f1_scores.append(2 * precision * recall / (precision + recall))
        return sum(f1_scores) / len(f1_scores)

    def _tokenize(self, text: str) -> Iterable[str]:
        for match in _TOKEN_RE.finditer(text.lower()):
            yield match.group(0)

    def _infer_category(self, tokens: Sequence[str]) -> HarmCategory:
        scores: Dict[HarmCategory, int] = {}
        for token in tokens:
            if token in self._token_category:
                category = self._token_category[token]
                scores[category] = scores.get(category, 0) + 1
            else:
                for category, keywords in _CATEGORY_KEYWORDS.items():
                    if token in keywords:
                        scores[category] = scores.get(category, 0) + 1
        if not scores:
            return HarmCategory.Unknown
        return max(scores.items(), key=lambda item: item[1])[0]

    def _explain_tokens(self, tokens: Sequence[str], label: RiskLevel) -> List[str]:
        assert self._model is not None
        scored_tokens: List[Tuple[str, float]] = []
        for token in tokens:
            likelihood = self._model._conditional.get((label, token))
            if likelihood is not None:
                scored_tokens.append((token, likelihood))
        scored_tokens.sort(key=lambda item: item[1], reverse=True)
        return [token for token, _ in scored_tokens[:5]]

    @staticmethod
    def _to_category(label: str) -> HarmCategory:
        normalized = label.lower()
        for category in HarmCategory:
            if category.value == normalized:
                return category
        return HarmCategory.Unknown
