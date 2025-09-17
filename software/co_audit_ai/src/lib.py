from __future__ import annotations

"""Static and dynamic security analysis for CoAuditAI."""

import ast
import importlib.abc
import importlib.util
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from types import ModuleType
from typing import Any, Dict, List, Optional, Sequence, Set, Tuple

from software.co_audit_ai.fuzzer import FuzzerConfig, FuzzReport, PythonFuzzer

__all__ = [
    "CoAuditAI",
    "CoAuditConfig",
    "AuditReport",
    "AuditFinding",
    "FindingSeverity",
]


class FindingSeverity(Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"

    @property
    def weight(self) -> int:
        return {
            FindingSeverity.LOW: 1,
            FindingSeverity.MEDIUM: 3,
            FindingSeverity.HIGH: 7,
            FindingSeverity.CRITICAL: 12,
        }[self]


@dataclass
class AuditFinding:
    kind: str
    severity: FindingSeverity
    message: str
    location: Tuple[str, int]
    evidence: Dict[str, Any] = field(default_factory=dict)


@dataclass
class FuzzerOutcome:
    entry_point: str
    report: FuzzReport


@dataclass
class AuditReport:
    subject: Path
    findings: List[AuditFinding]
    call_graph: Dict[str, Set[str]]
    fuzz_outcomes: List[FuzzerOutcome]
    risk_score: float

    def summary(self) -> Dict[str, Any]:
        return {
            "subject": str(self.subject),
            "risk_score": self.risk_score,
            "findings": [
                {
                    "kind": finding.kind,
                    "severity": finding.severity.value,
                    "message": finding.message,
                    "location": {
                        "function": finding.location[0],
                        "lineno": finding.location[1],
                    },
                    "evidence": finding.evidence,
                }
                for finding in self.findings
            ],
            "fuzzing": [
                {
                    "entry_point": outcome.entry_point,
                    "total_cases": outcome.report.total_cases,
                    "exceptions": outcome.report.exceptions,
                    "findings": [finding.issue for finding in outcome.report.findings],
                }
                for outcome in self.fuzz_outcomes
            ],
        }

    @property
    def risk_band(self) -> str:
        if self.risk_score >= 40:
            return "critical"
        if self.risk_score >= 25:
            return "high"
        if self.risk_score >= 12:
            return "elevated"
        if self.risk_score >= 5:
            return "medium"
        return "low"


@dataclass
class CoAuditConfig:
    strictness_level: str = "Standard"
    max_runtime_seconds: int = 60
    entry_points: Sequence[str] = ()
    enable_fuzzer: bool = True
    max_call_depth: int = 4
    fuzzer_seed: int = 2024
    allowed_sinks: Sequence[str] = ()


class CoAuditAI:
    def __init__(self, config: Optional[CoAuditConfig] = None) -> None:
        self.config = config or CoAuditConfig()

    def analyze(self, subject: str | Path | ModuleType) -> AuditReport:
        source_path = self._resolve_subject(subject)
        module_ast = ast.parse(source_path.read_text(encoding="utf-8"), filename=str(source_path))
        call_graph = _CallGraphBuilder().build(module_ast)
        analyzer = _TaintAnalyzer(self.config, call_graph)
        findings = analyzer.run(module_ast)
        fuzz_outcomes: List[FuzzerOutcome] = []
        if self.config.enable_fuzzer:
            fuzz_outcomes = self._execute_fuzzers(source_path, call_graph)
        risk_score = self._score(findings, fuzz_outcomes)
        return AuditReport(
            subject=source_path,
            findings=findings,
            call_graph=call_graph,
            fuzz_outcomes=fuzz_outcomes,
            risk_score=risk_score,
        )

    # ------------------------------------------------------------------
    def _resolve_subject(self, subject: str | Path | ModuleType) -> Path:
        if isinstance(subject, ModuleType):
            if not hasattr(subject, "__file__") or subject.__file__ is None:
                raise ValueError("Module lacks __file__ attribute")
            return Path(subject.__file__).resolve()
        path = Path(subject).resolve()
        if not path.exists():
            raise FileNotFoundError(path)
        return path

    def _execute_fuzzers(self, path: Path, call_graph: Dict[str, Set[str]]) -> List[FuzzerOutcome]:
        entry_points = list(self.config.entry_points) or self._default_entry_points(call_graph)
        module = self._load_module(path)
        outcomes: List[FuzzerOutcome] = []
        for name in entry_points:
            target = getattr(module, name, None)
            if callable(target):
                config = FuzzerConfig(seed=self.config.fuzzer_seed)
                fuzzer = PythonFuzzer(target, config=config)
                report = fuzzer.run()
                if report.has_findings():
                    outcomes.append(FuzzerOutcome(entry_point=name, report=report))
        return outcomes

    def _default_entry_points(self, call_graph: Dict[str, Set[str]]) -> List[str]:
        candidates = [name for name in call_graph if name.startswith("check_") or name.startswith("handle_")]
        if not candidates:
            candidates = [name for name in call_graph if not name.startswith("_")]
        return candidates[:5]

    def _load_module(self, path: Path) -> ModuleType:
        spec = importlib.util.spec_from_file_location(f"coaudit_target_{path.stem}", str(path))
        if spec is None or spec.loader is None:
            raise RuntimeError(f"Unable to import module from {path}")
        module = importlib.util.module_from_spec(spec)
        loader = spec.loader
        assert isinstance(loader, importlib.abc.Loader)  # type: ignore[attr-defined]
        loader.exec_module(module)  # type: ignore[attr-defined]
        return module

    def _score(self, findings: Sequence[AuditFinding], fuzz_outcomes: Sequence[FuzzerOutcome]) -> float:
        base = sum(finding.severity.weight for finding in findings)
        fuzz_penalty = sum(4 + 3 * len(outcome.report.findings) + 2 * len(outcome.report.exceptions) for outcome in fuzz_outcomes)
        if self.config.strictness_level.lower() == "strict":
            base *= 1.3
        elif self.config.strictness_level.lower() == "lenient":
            base *= 0.7
        return min(base + fuzz_penalty, 100.0)


# ======================================================================
# Static analysis helpers
# ======================================================================


class _CallGraphBuilder(ast.NodeVisitor):
    def __init__(self) -> None:
        self.graph: Dict[str, Set[str]] = {}
        self._current_function: Optional[str] = None

    def build(self, module: ast.AST) -> Dict[str, Set[str]]:
        self.visit(module)
        return self.graph

    def visit_FunctionDef(self, node: ast.FunctionDef) -> Any:
        previous = self._current_function
        self._current_function = node.name
        self.graph.setdefault(node.name, set())
        self.generic_visit(node)
        self._current_function = previous

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> Any:
        self.visit_FunctionDef(node)

    def visit_Call(self, node: ast.Call) -> Any:
        if self._current_function is not None:
            callee = self._resolve_name(node.func)
            if callee is not None:
                self.graph.setdefault(self._current_function, set()).add(callee)
        self.generic_visit(node)

    @staticmethod
    def _resolve_name(node: ast.AST) -> Optional[str]:
        if isinstance(node, ast.Name):
            return node.id
        if isinstance(node, ast.Attribute):
            return node.attr
        return None


class _TaintAnalyzer(ast.NodeVisitor):
    DANGEROUS_SINKS: Dict[str, str] = {
        "eval": "Dynamic evaluation of untrusted data",
        "exec": "Execution of untrusted code",
        "system": "Shell execution",
        "popen": "Subprocess invocation",
        "run": "Subprocess invocation",
        "load": "Unsafe YAML/serialization load",
        "loads": "Unsafe deserialization",
    }
    SANITIZERS = {"escape", "sanitize", "quote", "clean", "safe_load"}

    def __init__(self, config: CoAuditConfig, call_graph: Dict[str, Set[str]]) -> None:
        self.config = config
        self.call_graph = call_graph
        self.findings: List[AuditFinding] = []
        self._current_function: Optional[str] = None
        self._untrusted_functions: Set[str] = set()

    def run(self, module_ast: ast.AST) -> List[AuditFinding]:
        self.visit(module_ast)
        return self.findings

    # ------------------------------------------------------------------
    def visit_FunctionDef(self, node: ast.FunctionDef) -> Any:
        previous = self._current_function
        self._current_function = node.name
        if self._looks_untrusted(node):
            self._untrusted_functions.add(node.name)
        self.generic_visit(node)
        self._current_function = previous

    visit_AsyncFunctionDef = visit_FunctionDef

    def visit_Call(self, node: ast.Call) -> Any:
        if self._current_function is None:
            self.generic_visit(node)
            return

        callee_name = self._extract_callee_name(node)
        if callee_name is None:
            self.generic_visit(node)
            return

        if callee_name in self.config.allowed_sinks:
            self.generic_visit(node)
            return

        if callee_name.lower().startswith("input"):
            self._untrusted_functions.add(self._current_function)

        if callee_name.lower() in self.DANGEROUS_SINKS:
            if not self._call_is_sanitized(node):
                message = self.DANGEROUS_SINKS[callee_name.lower()]
                path = self._find_tainted_path(self._current_function)
                severity = FindingSeverity.CRITICAL if path else FindingSeverity.HIGH
                evidence = {"call": ast.unparse(node)} if hasattr(ast, "unparse") else {}
                if path:
                    evidence["call_chain"] = path
                self.findings.append(
                    AuditFinding(
                        kind="dangerous_sink",
                        severity=severity,
                        message=message,
                        location=(self._current_function, node.lineno),
                        evidence=evidence,
                    )
                )
        self.generic_visit(node)

    def visit_Try(self, node: ast.Try) -> Any:
        if self._current_function is None:
            self.generic_visit(node)
            return
        for handler in node.handlers:
            if isinstance(handler.body, list) and all(isinstance(stmt, ast.Pass) for stmt in handler.body):
                self.findings.append(
                    AuditFinding(
                        kind="suppressed_exception",
                        severity=FindingSeverity.MEDIUM,
                        message="Exception swallowed with bare pass",
                        location=(self._current_function, handler.lineno),
                        evidence={},
                    )
                )
        self.generic_visit(node)

    # ------------------------------------------------------------------
    def _looks_untrusted(self, node: ast.FunctionDef) -> bool:
        name = node.name.lower()
        suspicious_keywords = ("input", "request", "user", "payload", "external")
        if any(keyword in name for keyword in suspicious_keywords):
            return True
        return any(
            isinstance(arg.annotation, ast.Name) and arg.annotation.id.lower() == "untrusted"
            for arg in node.args.args
            if arg.annotation is not None
        )

    def _extract_callee_name(self, node: ast.Call) -> Optional[str]:
        if isinstance(node.func, ast.Name):
            return node.func.id
        if isinstance(node.func, ast.Attribute):
            return node.func.attr
        return None

    def _call_is_sanitized(self, node: ast.Call) -> bool:
        for arg in node.args:
            if isinstance(arg, ast.Call):
                callee = self._extract_callee_name(arg)
                if callee and any(keyword in callee.lower() for keyword in self.SANITIZERS):
                    return True
        return False

    def _find_tainted_path(self, sink_function: str) -> Optional[List[str]]:
        queue: List[Tuple[str, List[str]]] = [(source, [source]) for source in self._untrusted_functions]
        visited: Set[str] = set()
        while queue:
            current, path = queue.pop(0)
            if current == sink_function:
                return path
            for callee in self.call_graph.get(current, set()):
                if callee in visited:
                    continue
                visited.add(callee)
                queue.append((callee, path + [callee]))
        return None

