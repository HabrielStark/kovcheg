from __future__ import annotations

"""Cold Mirror harm prediction model training utilities.

This module provides a lightweight but *trained* text classification stack for
Cold Mirror.  It implements a Laplace-smoothed multinomial Naive Bayes model
with per-class statistics, evaluation helpers and deterministic dataset
handling.  The goal is to replace the previous keyword heuristics with a real
model that can be validated in unit tests and in integration flows.
"""

from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Tuple
import hashlib
import json
import math
import re
import statistics

_DATASET_PATH = Path(__file__).resolve().parent / "dataset" / "harm_training_data.json"
_TOKEN_RE = re.compile(r"[a-zA-Z_]+")


@dataclass(frozen=True)
class ModelMetrics:
    """Container summarising training/evaluation metrics."""

    dataset_hash: str
    train_size: int
    test_size: int
    risk_accuracy: float
    category_accuracy: float
    vocabulary_size: int
    mean_document_length: float


class NaiveBayesTextClassifier:
    """Simple multinomial Naive Bayes text classifier.

    The implementation keeps full token statistics so we can surface
    explanations (token likelihood contributions) for predictions.  Training is
    deterministic and requires no external dependencies beyond the standard
    library.
    """

    def __init__(self, alpha: float = 1.0):
        if alpha <= 0:
            raise ValueError("alpha must be positive for Laplace smoothing")
        self.alpha = alpha
        self.vocabulary: Dict[str, int] = {}
        self._class_totals: Dict[str, int] = {}
        self._token_totals: Dict[str, int] = {}
        self._token_counts: Dict[str, Dict[str, int]] = {}
        self._document_counts: Dict[str, int] = {}
        self._total_documents = 0

    # ------------------------------------------------------------------
    @staticmethod
    def _tokenise(text: str) -> List[str]:
        tokens = _TOKEN_RE.findall(text.lower())
        return tokens if tokens else [""]

    def fit(self, texts: Sequence[str], labels: Sequence[str]) -> None:
        if len(texts) != len(labels):
            raise ValueError("texts and labels must have the same length")
        if not texts:
            raise ValueError("training data is empty")

        self.vocabulary.clear()
        self._class_totals.clear()
        self._token_totals.clear()
        self._token_counts.clear()
        self._document_counts.clear()
        self._total_documents = 0

        for text, label in zip(texts, labels):
            tokens = self._tokenise(text)
            class_counts = self._token_counts.setdefault(label, {})
            self._document_counts[label] = self._document_counts.get(label, 0) + 1
            self._total_documents += 1

            for token in tokens:
                if token not in self.vocabulary:
                    self.vocabulary[token] = len(self.vocabulary)
                class_counts[token] = class_counts.get(token, 0) + 1
                self._token_totals[label] = self._token_totals.get(label, 0) + 1

        for label, token_counts in self._token_counts.items():
            self._class_totals[label] = sum(token_counts.values())

    # ------------------------------------------------------------------
    def _log_prior(self, label: str) -> float:
        doc_count = self._document_counts.get(label, 0)
        if doc_count == 0:
            return float("-inf")
        return math.log(doc_count / self._total_documents)

    def _log_likelihood(self, label: str, token: str) -> float:
        token_count = self._token_counts.get(label, {}).get(token, 0)
        total = self._class_totals.get(label, 0)
        denom = total + self.alpha * len(self.vocabulary)
        return math.log((token_count + self.alpha) / denom)

    def predict_log_proba(self, text: str) -> Dict[str, float]:
        tokens = self._tokenise(text)
        log_probs: Dict[str, float] = {}
        for label in self._document_counts:
            log_prob = self._log_prior(label)
            for token in tokens:
                log_prob += self._log_likelihood(label, token)
            log_probs[label] = log_prob
        return log_probs

    def predict(self, text: str) -> str:
        log_probs = self.predict_log_proba(text)
        if not log_probs:
            raise RuntimeError("classifier is not trained")
        return max(log_probs.items(), key=lambda item: item[1])[0]

    def predict_with_confidence(self, text: str) -> Tuple[str, float, Dict[str, float]]:
        log_probs = self.predict_log_proba(text)
        if not log_probs:
            raise RuntimeError("classifier is not trained")
        max_log = max(log_probs.values())
        exp_scores = {label: math.exp(score - max_log) for label, score in log_probs.items()}
        normaliser = sum(exp_scores.values()) or 1.0
        probabilities = {label: value / normaliser for label, value in exp_scores.items()}
        best_label, best_prob = max(probabilities.items(), key=lambda item: item[1])
        token_contributions = self.explain(text, best_label)
        return best_label, best_prob, token_contributions

    def explain(self, text: str, label: str, top_k: int = 3) -> Dict[str, float]:
        tokens = self._tokenise(text)
        contributions: Dict[str, float] = {}
        total = self._class_totals.get(label, 0) + self.alpha * len(self.vocabulary)
        if total == 0:
            return contributions
        for token in tokens:
            numerator = self._token_counts.get(label, {}).get(token, 0) + self.alpha
            contributions[token] = numerator / total
        # Return top_k tokens sorted by contribution
        top_tokens = sorted(contributions.items(), key=lambda item: item[1], reverse=True)[:top_k]
        return dict(top_tokens)

    def score(self, texts: Sequence[str], labels: Sequence[str]) -> float:
        if not texts:
            return 0.0
        correct = 0
        for text, expected in zip(texts, labels):
            predicted = self.predict(text)
            if predicted == expected:
                correct += 1
        return correct / len(texts)

    @property
    def vocabulary_size(self) -> int:
        return len(self.vocabulary)


# ---------------------------------------------------------------------------


def _load_dataset() -> List[Dict[str, str]]:
    if not _DATASET_PATH.exists():
        raise FileNotFoundError(f"Cold Mirror dataset missing at {_DATASET_PATH}")
    with _DATASET_PATH.open("r", encoding="utf-8") as f:
        dataset: List[Dict[str, str]] = json.load(f)
    return dataset


def _split_dataset(dataset: Sequence[Dict[str, str]], test_ratio: float = 0.2) -> Tuple[List[Dict[str, str]], List[Dict[str, str]]]:
    if not 0 < test_ratio < 1:
        raise ValueError("test_ratio must be in (0, 1)")

    # Deterministic stratified split by risk label
    by_label: Dict[str, List[Dict[str, str]]] = {}
    for item in dataset:
        label = item["risk"].lower()
        by_label.setdefault(label, []).append(item)

    train, test = [], []
    for items in by_label.values():
        items_sorted = sorted(items, key=lambda entry: hashlib.sha1(entry["text"].encode("utf-8")).hexdigest())
        cutoff = max(1, int(len(items_sorted) * (1 - test_ratio)))
        train.extend(items_sorted[:cutoff])
        test.extend(items_sorted[cutoff:])
    return train, test


def _dataset_hash(dataset: Sequence[Dict[str, str]]) -> str:
    digest = hashlib.sha256()
    for item in dataset:
        digest.update(item["text"].encode("utf-8"))
        digest.update(item["risk"].encode("utf-8"))
        digest.update(item["category"].encode("utf-8"))
    return digest.hexdigest()


def _mean_document_length(dataset: Sequence[Dict[str, str]]) -> float:
    lengths = [len(NaiveBayesTextClassifier._tokenise(entry["text"])) for entry in dataset]
    return statistics.fmean(lengths) if lengths else 0.0


__all__ = [
    "ModelMetrics",
    "NaiveBayesTextClassifier",
    "_load_dataset",
    "_split_dataset",
    "_dataset_hash",
]
