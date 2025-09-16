from __future__ import annotations

"""High-fidelity Python shim for the CoAuditAI system.

The previous revision of this module always returned ``{"status": "PASS"}``
regardless of the analysed input.  The implementation below mirrors the
behaviour of the Rust core by performing lightweight static analysis,
heuristics for moral alignment, and risk scoring.  It is intentionally
self-contained so that unit and integration tests can exercise the auditing
pipeline without compiling the heavy Rust crate.
"""

from dataclasses import dataclass
from pathlib import Path
import hashlib
import time
from typing import Any, Dict, Iterable, List, Sequence

__all__ = ["CoAuditAI", "CoAuditConfig"]


@dataclass(slots=True)
class CoAuditConfig:
    """Configuration knobs for :class:`CoAuditAI`."""

    strictness_level: str = "Standard"
    max_runtime_seconds: int = 60
    additional_settings: Dict[str, Any] | None = None
    max_bytes: int = 64_000
    medium_threshold: float = 0.45
    high_threshold: float = 0.7
    virtue_floor: float = 0.35


@dataclass(slots=True)
class AuditIssue:
    """Issue detected during the audit."""

    category: str
    severity: str
    message: str
    evidence: str | None = None
    line_number: int | None = None

    def to_dict(self) -> Dict[str, Any]:
        """Return a JSON-serialisable representation."""

        payload: Dict[str, Any] = {
            "category": self.category,
            "severity": self.severity,
            "message": self.message,
        }
        if self.evidence is not None:
            payload["evidence"] = self.evidence
        if self.line_number is not None:
            payload["line"] = self.line_number
        return payload


class CoAuditAI:
    """Lightweight analyser providing deterministic audit results."""

    _SECURITY_PATTERNS: Sequence[tuple[str, str, str]] = (
        ("Critical", "Security", "Use of eval/exec allows arbitrary code execution"),
        ("High", "Security", "Subprocess invocation without sandboxing"),
        ("Medium", "Security", "Import wildcard may shadow security primitives"),
        ("Medium", "Security", "Hard-coded credentials detected"),
        ("Low", "Security", "TODO/FIXME indicates unfinished security work"),
    )
    _SECURITY_MARKERS: Sequence[str] = (
        "eval(",
        "exec(",
        "subprocess.Popen",
        "os.system",
        "import *",
        "password",
        "secret",
        "TODO",
        "FIXME",
    )

    _MORAL_RED_FLAGS: Dict[str, tuple[str, float]] = {
        "harm": ("High", 0.35),
        "kill": ("Critical", 0.55),
        "genocide": ("Critical", 0.65),
        "deceive": ("Medium", 0.25),
        "manipulate": ("Medium", 0.22),
        "exploit": ("Medium", 0.28),
        "idol": ("High", 0.3),
        "false prophecy": ("High", 0.32),
        "oppress": ("High", 0.33),
    }

    _BIBLICAL_VIRTUES: Sequence[str] = (
        "love",
        "mercy",
        "justice",
        "righteousness",
        "truth",
        "charity",
        "grace",
        "repent",
        "faith",
        "hope",
        "forgive",
        "peace",
    )

    _SEVERITY_WEIGHTS: Dict[str, float] = {
        "Info": 0.05,
        "Low": 0.12,
        "Medium": 0.24,
        "High": 0.45,
        "Critical": 0.8,
    }

    _SEVERITY_ORDER: Dict[str, int] = {
        "Info": 0,
        "Low": 1,
        "Medium": 2,
        "High": 3,
        "Critical": 4,
    }

    _STRICTNESS_THRESHOLDS: Dict[str, float] = {
        "lenient": 0.85,
        "standard": 0.7,
        "strict": 0.55,
        "paranoid": 0.4,
    }

    def __init__(self, config: CoAuditConfig | None = None):
        self.config = config or CoAuditConfig()

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------
    def analyze(self, subject: str | Path) -> Dict[str, Any]:
        """Analyse ``subject`` and return a structured report."""

        start = time.perf_counter()
        text, metadata = self._load_subject(subject)

        features = self._extract_features(text)
        issues = self._collect_issues(text, features)

        severity_score = sum(
            self._SEVERITY_WEIGHTS.get(issue.severity, 0.0) for issue in issues
        )
        severity_score = min(severity_score, 1.0)

        strictness = self.config.strictness_level.lower()
        threshold = self._STRICTNESS_THRESHOLDS.get(strictness, 0.7)
        risk_limit = min(threshold, self.config.high_threshold)
        fail_due_to_alignment = (
            features["biblical_alignment"] < self.config.virtue_floor
            and strictness != "lenient"
        )
        max_severity = max((self._SEVERITY_ORDER[i.severity] for i in issues), default=0)

        status = "PASS"
        if max_severity >= self._SEVERITY_ORDER["Critical"]:
            status = "FAIL"
        elif severity_score > risk_limit or fail_due_to_alignment:
            status = "FAIL"
        elif max_severity >= self._SEVERITY_ORDER["High"] and severity_score > self.config.medium_threshold:
            status = "FAIL"

        duration_ms = (time.perf_counter() - start) * 1000.0

        report = {
            "subject": metadata["subject"],
            "status": status,
            "score": {
                "risk": round(severity_score, 3),
                "biblical_alignment": round(features["biblical_alignment"], 3),
                "max_severity": max((issue.severity for issue in issues), default="Info"),
            },
            "issues": [issue.to_dict() for issue in issues],
            "metrics": {
                "line_count": features["line_count"],
                "avg_line_length": round(features["avg_line_length"], 2),
                "sha256": features["sha256"],
                "dangerous_constructs": sorted(features["dangerous_constructs"]),
            },
            "analysis_time_ms": round(duration_ms, 3),
        }

        if issues:
            report["recommendations"] = self._build_recommendations(issues)

        return report

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------
    def _load_subject(self, subject: str | Path) -> tuple[str, Dict[str, Any]]:
        if isinstance(subject, Path) or (isinstance(subject, str) and Path(subject).exists()):
            path = Path(subject)
            raw = path.read_bytes()[: self.config.max_bytes]
            text = raw.decode("utf-8", errors="ignore")
            return text, {"subject": str(path), "source": "file"}

        text = str(subject)
        if len(text.encode("utf-8")) > self.config.max_bytes:
            text = text[: self.config.max_bytes]
        return text, {"subject": text[:64], "source": "text"}

    def _extract_features(self, text: str) -> Dict[str, Any]:
        tokens = self._tokenise(text)
        line_count = text.count("\n") + (1 if text else 0)
        avg_line_length = (len(text) / max(line_count, 1)) if text else 0.0

        virtue_hits = sum(1 for token in tokens if token in self._BIBLICAL_VIRTUES)
        red_flags = sum(1 for token in tokens if token in self._MORAL_RED_FLAGS)
        total = virtue_hits + red_flags
        alignment = virtue_hits / total if total else 0.5

        return {
            "tokens": tokens,
            "line_count": line_count,
            "avg_line_length": avg_line_length,
            "sha256": hashlib.sha256(text.encode("utf-8", errors="ignore")).hexdigest(),
            "biblical_alignment": alignment,
            "dangerous_constructs": set(),
        }

    def _collect_issues(self, text: str, features: Dict[str, Any]) -> List[AuditIssue]:
        issues: List[AuditIssue] = []
        lowered = text.lower()
        lines = text.splitlines()

        # Security markers
        for marker in self._SECURITY_MARKERS:
            if marker.lower() in lowered:
                severity, category, message = self._classify_security_marker(marker)
                line_no = self._find_line(lines, marker)
                features["dangerous_constructs"].add(marker)
                issues.append(
                    AuditIssue(
                        category=category,
                        severity=severity,
                        message=message,
                        evidence=marker,
                        line_number=line_no,
                    )
                )

        # Moral red flags
        for token, (severity, weight) in self._MORAL_RED_FLAGS.items():
            if token in lowered:
                line_no = self._find_line(lines, token)
                issues.append(
                    AuditIssue(
                        category="Moral",
                        severity=severity,
                        message=f"Detected morally risky concept: '{token}'",
                        evidence=token,
                        line_number=line_no,
                    )
                )
                features["dangerous_constructs"].add(token)
                features["biblical_alignment"] = max(
                    0.0, features["biblical_alignment"] - weight * 0.1
                )

        # Heuristic: long files with no virtues are suspect.
        if features["line_count"] > 200 and features["biblical_alignment"] < 0.25:
            issues.append(
                AuditIssue(
                    category="Governance",
                    severity="Medium",
                    message="Large artefact lacks Biblical context",
                )
            )

        return issues

    def _classify_security_marker(self, marker: str) -> tuple[str, str, str]:
        if marker in {"eval(", "exec("}:
            return self._SECURITY_PATTERNS[0]
        if marker in {"subprocess.Popen", "os.system"}:
            return self._SECURITY_PATTERNS[1]
        if marker == "import *":
            return self._SECURITY_PATTERNS[2]
        if marker in {"password", "secret"}:
            return self._SECURITY_PATTERNS[3]
        return self._SECURITY_PATTERNS[4]

    @staticmethod
    def _find_line(lines: Sequence[str], needle: str) -> int | None:
        lowered = needle.lower()
        for idx, line in enumerate(lines, start=1):
            if lowered in line.lower():
                return idx
        return None

    @staticmethod
    def _tokenise(text: str) -> List[str]:
        return [token for token in text.lower().replace("-", " ").split() if token]

    def _build_recommendations(self, issues: Iterable[AuditIssue]) -> List[Dict[str, Any]]:
        recommendations: List[Dict[str, Any]] = []
        for issue in issues:
            priority = "High" if issue.severity in {"High", "Critical"} else "Normal"
            recommendations.append(
                {
                    "priority": priority,
                    "category": issue.category,
                    "action": issue.message,
                    "evidence": issue.evidence,
                }
            )
        return recommendations

