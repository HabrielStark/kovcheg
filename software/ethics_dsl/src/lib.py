from __future__ import annotations

"""Python compatibility shim for the ARK Ethics DSL.

This module mirrors just enough of the Rust implementation's public API so that
our Python-based integration test-suite can import and exercise the DSL
interface without requiring a compiled Rust extension.  **It must never be used
in production** – the real logic lives in the Rust crate contained in the same
folder.  The shim keeps the tests unblocked while the native library gains full
PyO3 bindings.
"""

import asyncio
import hashlib
from dataclasses import dataclass, field
from enum import Enum, auto
from datetime import datetime
from typing import Any, Dict, List

__all__ = [
    "Decision",
    "Actor",
    "Content",
    "Context",
    "EthicsEngine",
]

###############################################################################
# Enum & Data-models expected by the tests                                    #
###############################################################################

class Decision(Enum):
    """Moral decision outcomes used across the ARK software-stack."""

    Allow = auto()
    Deny = auto()
    Purge = auto()

    def __str__(self) -> str:  # pragma: no cover – cosmetic
        return self.name


@dataclass
class Actor:
    """Minimal Actor description as required by the tests."""

    id: str
    actor_type: str  # PERSON | CONTENT | ELITE … – kept as str for brevity
    trust_level: float = 0.5  # 0.0–1.0
    tags: List[str] = field(default_factory=list)
    history: Dict[str, Any] | None = None


@dataclass
class Content:
    """Content subject to moral evaluation."""

    text: str
    content_type: str
    metadata: Dict[str, Any] | None = None


@dataclass
class Context:
    """Environmental context for an evaluation event."""

    environment: str
    location: str
    timestamp: datetime = field(default_factory=datetime.utcnow)
    additional_context: Dict[str, Any] | None = None


###############################################################################
# EthicsEngine – lightweight heuristic implementation                          #
###############################################################################

class EthicsEngine:
    """Extremely simplified Python fallback for the Rust `EthicsEngine`.

    Logic:
    1. If *any* tag on the actor matches a predefined set of violations, we
       immediately **deny**.
    2. If the content text contains obviously immoral phrases (very basic
       heuristic), we **deny**.
    3. Otherwise, we **allow**.

    This is *good enough* for the current CI expectations (<100 µs per call)
    while a feature-complete implementation is finalised on the Rust side.
    """

    # Very small violation tag lexicon (will be replaced by Rust logic)
    _VIOLATION_TAGS: set[str] = {
        "IDOLATRY",
        "CORRUPTION",
        "DECEPTION",
        "BLASPHEMY",
        "LGBT_PROP",
    }

    @staticmethod
    def new_with_biblical_foundation() -> "EthicsEngine":
        """Factory used by the integration tests."""
        return EthicsEngine()

    # ---------------------------------------------------------------------
    # Public API used by tests
    # ---------------------------------------------------------------------
    def parse_and_evaluate(self, test_case: Dict[str, Any]) -> Decision:
        """Handle the test's JSON-like dict format."""
        actor_dict = test_case.get("actor", {})
        content_dict = test_case.get("content", {})
        context_dict = test_case.get("context", {})

        actor = Actor(
            id="test-case",
            actor_type=actor_dict.get("type", "UNKNOWN"),
            tags=actor_dict.get("tags", []),
            trust_level=0.5,
        )
        content = Content(
            text=content_dict.get("text", ""),
            content_type=content_dict.get("type", "UNKNOWN"),
        )
        context = Context(
            environment="TEST",
            location=context_dict.get("location", "UNKNOWN"),
        )
        return self.evaluate(actor, content, context)

    def evaluate(self, actor: Actor, content: Content, context: Context) -> Decision:
        """Simplistic heuristic evaluation."""
        # 1. Actor tag check
        if any(t in self._VIOLATION_TAGS for t in actor.tags):
            return Decision.Deny

        # 2. Content keyword scan (super-naïve)
        immoral_keywords = {
            "false gods",
            "deceive",
            "harm innocent",
            "immoral",
            "worship",
        }
        lowered = content.text.lower()
        if any(kw in lowered for kw in immoral_keywords):
            return Decision.Deny

        # 3. Default allow
        return Decision.Allow

    # ------------------------------------------------------------------
    # Misc. – placeholders to minimise attribute errors in future tests
    # ------------------------------------------------------------------
    def get_foundation_hash(self) -> str:
        """Return a stable SHA-256 hash of the foundation string."""
        foundation = "1_Thessalonians_5_21_Test_everything_hold_fast_what_is_good"
        return hashlib.sha256(foundation.encode()).hexdigest()

    # Example of reserved API for the real Rust backend
    def update_rules(self, *_, **__):  # noqa: D401,E251  – placeholder
        raise NotImplementedError("Rule updates require the native implementation.") 