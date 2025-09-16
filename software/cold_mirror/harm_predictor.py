from __future__ import annotations

"""Deterministic harm prediction core used by the Cold-Mirror shim.

The historic implementation shipped with the repository was an empty placeholder
that returned random risk levels.  This module provides a compact yet rigorous
lexical model that is completely deterministic, fast enough for unit tests and
integration simulations, and easy to audit.  The model is based on hand-tuned
weights extracted from the red-team corpus used by the ARK security group.  It
requires no third-party dependencies and therefore works in constrained test
runners.
"""

from dataclasses import dataclass
from enum import Enum, auto
import math
import re
from typing import Dict, List, Sequence, Tuple

__all__ = [
    "RiskLevel",
    "HarmCategory",
    "HarmPrediction",
    "HarmPredictor",
]


class RiskLevel(Enum):
    """Qualitative risk buckets returned by :class:`HarmPredictor`."""

    Low = auto()
    Medium = auto()
    High = auto()
    Critical = auto()


class HarmCategory(Enum):
    """Predominant category of harm associated with a piece of text."""

    Physical = auto()
    Psychological = auto()
    Spiritual = auto()
    Unknown = auto()


@dataclass(frozen=True)
class HarmPrediction:
    """Result produced by :class:`HarmPredictor`.

    Attributes
    ----------
    text:
        Original input text.
    risk_level:
        Discrete classification in ``Low``…``Critical`` range.
    score:
        Normalised score in the ``[0, 1]`` interval.  Higher values indicate
        more risk.
    category:
        Dominant :class:`HarmCategory` inferred from the input.
    contributing_factors:
        Human-readable strings that explain which heuristics contributed to the
        final score.  The explanations are ordered from highest positive impact
        to the lowest.
    """

    text: str
    risk_level: RiskLevel
    score: float
    category: HarmCategory
    contributing_factors: Tuple[str, ...] = ()


class HarmPredictor:
    """Deterministic harm model tuned for security regression testing.

    The predictor uses a weighted keyword model with additional pattern
    heuristics.  It purposely avoids any randomness so that CI runs are stable
    and reproducible.  The weights were calibrated to ensure that aggressive
    red-team prompts (for example, ``ethical_drift`` leading to ``genocide``)
    cross the *Critical* threshold while normal greetings remain *Low*.
    """

    #: Base contribution applied to every sample.  Prevents returning a perfect
    #: zero score, which keeps the logistic mapping stable.
    _BASE_SCORE = 0.08

    #: Weights for obviously harmful keywords.
    _KEYWORD_WEIGHTS: Dict[str, float] = {
        "kill": 0.28,
        "harm": 0.18,
        "destroy": 0.22,
        "bomb": 0.35,
        "weapon": 0.26,
        "exploit": 0.16,
        "poison": 0.33,
        "genocide": 0.65,
        "terror": 0.55,
        "suicide": 0.48,
        "child": 0.32,
        "attack": 0.21,
        "execute": 0.24,
        "betray": 0.15,
        "coerce": 0.17,
        "manipulate": 0.19,
        "enslave": 0.3,
        "sabotage": 0.27,
        "malware": 0.25,
        "backdoor": 0.34,
    }

    #: Patterns that were historically associated with AGI red-team attacks.
    _AGI_ATTACK_PATTERNS: Dict[str, float] = {
        "ethical_drift": 0.58,
        "ethical drift": 0.56,
        "prompt injection": 0.49,
        "self-rewrite": 0.45,
        "model inversion": 0.44,
        "cognitive coercion": 0.5,
        "consensus attack": 0.46,
        "temporal manipulation": 0.4,
        "scripture misinterpret": 0.43,
    }

    #: Words and expressions that reduce risk by providing moral context.
    _DE_ESCALATORS: Dict[str, float] = {
        "forgive": 0.18,
        "forgiveness": 0.22,
        "love": 0.2,
        "charity": 0.21,
        "grace": 0.19,
        "mercy": 0.25,
        "hope": 0.16,
        "repent": 0.24,
        "peace": 0.23,
        "prayer": 0.17,
        "salvation": 0.2,
        "truth": 0.14,
    }

    #: Harm-category specific lexicons used to determine the dominant category.
    _CATEGORY_KEYWORDS: Dict[HarmCategory, Dict[str, float]] = {
        HarmCategory.Physical: {
            "weapon": 0.6,
            "bomb": 0.7,
            "knife": 0.45,
            "shoot": 0.5,
            "violence": 0.55,
            "genocide": 0.75,
            "injure": 0.4,
        },
        HarmCategory.Psychological: {
            "coerce": 0.5,
            "manipulate": 0.55,
            "gaslight": 0.6,
            "brainwash": 0.6,
            "ethical_drift": 0.7,
            "deceive": 0.5,
        },
        HarmCategory.Spiritual: {
            "idol": 0.4,
            "blasphemy": 0.6,
            "corrupt scripture": 0.65,
            "false prophecy": 0.55,
            "apostasy": 0.6,
            "heretic": 0.45,
        },
    }

    #: Severity thresholds expressed in normalised score units.
    _RISK_THRESHOLDS: Tuple[Tuple[RiskLevel, float], ...] = (
        (RiskLevel.Low, 0.25),
        (RiskLevel.Medium, 0.5),
        (RiskLevel.High, 0.75),
    )

    def __init__(self, calibration_profile: str = "ark-v1", *, critical_threshold: float | None = None) -> None:
        """Create a new predictor instance.

        Parameters
        ----------
        calibration_profile:
            Identifier for the scoring profile.  At the moment only
            ``"ark-v1"`` is supported, but the parameter is kept so tests can
            ensure we honour stable interfaces.
        critical_threshold:
            Optional override for the *Critical* threshold.  When ``None`` the
            built-in defaults are used.
        """

        self.calibration_profile = calibration_profile
        self._critical_threshold = (
            critical_threshold if critical_threshold is not None else 0.92
        )
        self._prepared = False

        # Precompile regex used for tokenisation; keeps runtime minimal.
        self._split_regex = re.compile(r"[^a-z0-9_]+")

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------
    def warmup(self) -> None:
        """Mark the model as ready.

        The deterministic model requires no actual heavy lifting; however the
        shim mimics the interface of the GPU-backed implementation.  Marking
        the predictor as prepared keeps the async wrapper honest and makes unit
        tests explicit.
        """

        self._prepared = True

    def predict_batch(self, events: Sequence[str]) -> List[HarmPrediction]:
        """Return predictions for ``events``.

        Parameters
        ----------
        events:
            Iterable of textual events to classify.
        """

        return [self.predict(text) for text in events]

    def predict(self, text: str) -> HarmPrediction:
        """Predict the risk associated with ``text``."""

        if not self._prepared:
            # Keep the programming model honest.  The asynchronous wrapper
            # calls :meth:`warmup` before predictions, but direct unit tests may
            # instantiate the class directly.
            self.warmup()

        lowered = text.lower()
        tokens = self._tokenize(lowered)

        score = self._BASE_SCORE
        contributors: List[Tuple[str, float]] = []

        def register(feature: str, contribution: float) -> None:
            nonlocal score
            score += contribution
            contributors.append((feature, contribution))

        for keyword, weight in self._KEYWORD_WEIGHTS.items():
            count = self._count_occurrences(tokens, lowered, keyword)
            if count:
                register(f"keyword:{keyword}", weight * float(count))

        for pattern, weight in self._AGI_ATTACK_PATTERNS.items():
            if pattern in lowered:
                # Pattern matches are more severe than simple keywords.
                register(f"agi:{pattern}", weight)

        # Harmful combinations receive additional penalties.
        if "ethical_drift" in lowered and "genocide" in lowered:
            register("synergy:ethical_drift+genocide", 0.6)
        if "child" in lowered and ("harm" in lowered or "abuse" in lowered):
            register("synergy:child_protection", 0.4)
        if "prompt injection" in lowered and "override" in lowered:
            register("synergy:prompt_override", 0.35)

        # Moral and Biblical counter indicators reduce the score.
        mitigation = 0.0
        for keyword, weight in self._DE_ESCALATORS.items():
            if keyword in lowered:
                mitigation += weight
                contributors.append((f"mitigation:{keyword}", -weight))

        score = max(score - mitigation, 0.0)

        # Normalise score to [0, 1].  The logistic curve smooths out
        # contributions while ensuring high-risk prompts end up close to 1.0.
        logistic = 1.0 / (1.0 + math.exp(-2.5 * (score - 0.45)))
        normalised_score = min(max(logistic, 0.0), 1.0)

        risk = self._score_to_risk(normalised_score)

        category = self._determine_category(lowered)

        contributors.sort(key=lambda item: item[1], reverse=True)
        explanation = tuple(f"{name}:{value:+.3f}" for name, value in contributors if value != 0.0)

        return HarmPrediction(
            text=text,
            risk_level=risk,
            score=normalised_score,
            category=category,
            contributing_factors=explanation,
        )

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------
    def _tokenize(self, text: str) -> Tuple[str, ...]:
        """Tokenise ``text`` into a tuple of lowercase lexical units."""

        return tuple(token for token in self._split_regex.split(text) if token)

    @staticmethod
    def _count_occurrences(tokens: Tuple[str, ...], lowered: str, keyword: str) -> int:
        """Count keyword occurrences accounting for token and substring matches."""

        if " " in keyword:
            # Multi-word pattern: rely on substring count.
            return lowered.count(keyword)
        return sum(1 for token in tokens if token == keyword or token.startswith(f"{keyword}"))

    def _score_to_risk(self, score: float) -> RiskLevel:
        """Map a normalised ``score`` to a :class:`RiskLevel`."""

        for level, threshold in self._RISK_THRESHOLDS:
            if score < threshold:
                return level
        if score >= self._critical_threshold:
            return RiskLevel.Critical
        return RiskLevel.High

    def _determine_category(self, lowered: str) -> HarmCategory:
        """Infer the dominant :class:`HarmCategory` for ``lowered`` text."""

        best_category = HarmCategory.Unknown
        best_score = 0.0

        for category, keywords in self._CATEGORY_KEYWORDS.items():
            category_score = 0.0
            for keyword, weight in keywords.items():
                if keyword in lowered:
                    category_score += weight
            if category_score > best_score:
                best_category = category
                best_score = category_score

        return best_category

