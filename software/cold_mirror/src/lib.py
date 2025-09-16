from __future__ import annotations

"""Asynchronous shim around :mod:`software.cold_mirror.harm_predictor`.

The original file contained a toy implementation that relied on ``random``
choices and printed debug messages to stdout.  Tests only require a lightweight
async facade so we delegate the heavy lifting to the deterministic core model
implemented in :mod:`software.cold_mirror.harm_predictor`.
"""

import asyncio
from typing import Sequence

from software.cold_mirror.harm_predictor import (
    HarmCategory,
    HarmPrediction,
    HarmPredictor as _CorePredictor,
    RiskLevel,
)

__all__ = [
    "RiskLevel",
    "HarmCategory",
    "HarmPrediction",
    "HarmPredictor",
]


class HarmPredictor:
    """Async-compatible wrapper around the deterministic harm model."""

    def __init__(self, calibration_profile: str = "ark-v1") -> None:
        self._core = _CorePredictor(calibration_profile)
        self.agi_defense_active = True
        self.biblical_filter_active = True
        self.quantum_protection_level = 0.9999
        self._initialised = False

    async def initialize(self) -> None:
        """Simulate asynchronous model initialisation."""

        if not self._initialised:
            # Yield to the event loop so that callers exercising timing-sensitive
            # code keep their structure, then warm up the deterministic model.
            await asyncio.sleep(0)
            self._core.warmup()
            self._initialised = True

    async def predict_batch_harm(self, events: Sequence[str]) -> list[HarmPrediction]:
        """Predict harm for a batch of events."""

        if not self._initialised:
            await self.initialize()

        return [self._core.predict(text) for text in events]

    async def predict_harm(self, events: Sequence[str]) -> list[HarmPrediction]:
        """Alias used by some integration flows."""

        return await self.predict_batch_harm(events)

    # ------------------------------------------------------------------
    # Compatibility helpers for existing unit tests
    # ------------------------------------------------------------------
    def _predict_single_enhanced(self, text: str) -> HarmPrediction:
        """Direct synchronous access used by coverage-focused tests."""

        if not self._initialised:
            self._core.warmup()
            self._initialised = True

        return self._core.predict(text)

