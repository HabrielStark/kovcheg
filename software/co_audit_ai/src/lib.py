from __future__ import annotations

"""CoAuditAI – contextual auditor with static analysis and fuzzing."""

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple
from contextlib import contextmanager
import ast
import builtins
import inspect
import sys
import threading
import textwrap
import time

from ..attack_fuzzer import AttackSurfaceFuzzer

__all__ = ["CoAuditAI", "CoAuditConfig", "AuditIssue", "AuditReport"]


###############################################################################
# Data-classes                                                                 #
###############################################################################


@dataclass
class CoAuditConfig:
    strictness_level: str = "Standard"
    max_runtime_seconds: int = 60
    enable_dynamic: bool = True
    max_fuzz_payloads: int = 10
    additional_settings: Dict[str, Any] | None = None


@dataclass
class AuditIssue:
    rule: str
    severity: str
    message: str
    location: Optional[str] = None
    remediation: Optional[str] = None
    evidence: Optional[str] = None
    dynamic: bool = False

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rule": self.rule,
            "severity": self.severity,
            "message": self.message,
            "location": self.location,
            "remediation": self.remediation,
            "evidence": self.evidence,
            "dynamic": self.dynamic,
        }


@dataclass
class AuditReport:
    subject: str
    status: str
    issues: List[AuditIssue] = field(default_factory=list)
    metrics: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "subject": self.subject,
            "status": self.status,
            "issues": [issue.to_dict() for issue in self.issues],
            "metrics": self.metrics,
        }


###############################################################################
# Exceptions                                                                   #
###############################################################################


class DangerousOperation(RuntimeError):
    """Raised when the sandbox detects a dangerous primitive."""


class DynamicAnalysisTimeout(RuntimeError):
    """Raised when dynamic analysis exceeds the configured time budget."""

    def __init__(self, stage: str):
        self.stage = stage
        super().__init__(f"time budget exceeded during {stage}")


###############################################################################
# Main analyser                                                                #
###############################################################################


class CoAuditAI:
    _SEVERITY_WEIGHT = {"Info": 0, "Low": 1, "Medium": 2, "High": 3, "Critical": 4}

    _DANGEROUS_CALLS: Dict[str, Tuple[str, str]] = {
        "eval": ("Critical", "Avoid eval(); use safe parsers."),
        "exec": ("Critical", "Dynamic code execution detected."),
        "compile": ("High", "Runtime compilation of user data."),
        "os.system": ("High", "Shell execution is unsafe."),
        "subprocess.Popen": ("High", "Subprocess invocation requires sanitisation."),
        "subprocess.call": ("High", "Subprocess invocation requires sanitisation."),
        "open": ("Medium", "File system access should be mediated."),
        "builtins.open": ("Medium", "File system access should be mediated."),
        "pathlib.Path.open": ("Medium", "File handles must be validated."),
        "importlib.import_module": ("Medium", "Dynamic module import may be abused."),
    }

    _DANGEROUS_IMPORTS = {
        "pickle": "Pickle leads to arbitrary code execution; prefer JSON.",
        "subprocess": "Subprocess module should be wrapped with allow-lists.",
        "os": "os.system and friends must be audited carefully.",
    }

    _TAINT_SOURCES = {
        "input",
        "sys.argv",
        "request.args.get",
        "request.form.get",
        "flask.request.args.get",
        "flask.request.form.get",
    }

    _SAFE_BUILTINS = {
        "len": len,
        "range": range,
        "enumerate": enumerate,
        "sum": sum,
        "min": min,
        "max": max,
        "any": any,
        "all": all,
        "sorted": sorted,
        "print": lambda *args, **kwargs: None,
    }

    def __init__(self, config: CoAuditConfig | None = None):
        self.config = config or CoAuditConfig()
        self._last_report: Optional[AuditReport] = None

    # ------------------------------------------------------------------
    def analyze(self, subject: str) -> Dict[str, Any]:
        source, subject_label = self._load_source(subject)
        tree = ast.parse(source)
        _link_parents(tree)

        static_issues, tainted = self._run_static_checks(tree)
        dynamic_issues = self._run_dynamic_checks(source) if self.config.enable_dynamic else []

        issues = static_issues + dynamic_issues
        metrics = {
            "issue_count": len(issues),
            "static_findings": len(static_issues),
            "dynamic_findings": len(dynamic_issues),
            "tainted_variables": len(tainted),
            "strictness": self.config.strictness_level,
        }
        status = self._determine_status(issues)

        report = AuditReport(subject=subject_label, status=status, issues=issues, metrics=metrics)
        self._last_report = report
        return report.to_dict()

    # ------------------------------------------------------------------
    def _load_source(self, subject: str) -> Tuple[str, str]:
        path = Path(subject)
        if path.exists() and path.is_file():
            text = path.read_text(encoding="utf-8")
            return text, str(path)
        return textwrap.dedent(subject), "inline-snippet"

    # ------------------------------------------------------------------
    def _run_static_checks(self, tree: ast.AST) -> Tuple[List[AuditIssue], List[str]]:
        issues: List[AuditIssue] = []
        tainted: List[str] = []

        tainted_symbols: Dict[str, str] = {}

        for node in ast.walk(tree):
            if isinstance(node, ast.Assign):
                sources = self._extract_names(node.value)
                if any(name in self._TAINT_SOURCES for name in sources):
                    for target in node.targets:
                        for name in self._extract_target_names(target):
                            tainted_symbols[name] = "direct-input"
                elif any(name in tainted_symbols for name in sources):
                    for target in node.targets:
                        for name in self._extract_target_names(target):
                            tainted_symbols[name] = "propagated"

            elif isinstance(node, ast.AugAssign):
                sources = self._extract_names(node.value)
                target_names = self._extract_target_names(node.target)
                if any(name in tainted_symbols for name in sources + target_names):
                    for name in target_names:
                        tainted_symbols[name] = "propagated"

            elif isinstance(node, (ast.Import, ast.ImportFrom)):
                module_names = [alias.name for alias in node.names]
                for module_name in module_names:
                    root_name = module_name.split(".")[0]
                    if root_name in self._DANGEROUS_IMPORTS:
                        issues.append(
                            AuditIssue(
                                rule=f"IMPORT::{root_name}",
                                severity="Medium",
                                message=self._DANGEROUS_IMPORTS[root_name],
                                location=self._node_location(node),
                                remediation="Review module usage and restrict entry points.",
                            )
                        )

            elif isinstance(node, ast.Call):
                call_name = self._call_name(node)
                if call_name in self._TAINT_SOURCES:
                    # direct taint assignment like data = input()
                    parent_assign = self._nearest_parent_assign(node)
                    if parent_assign is not None:
                        for target in parent_assign.targets:
                            for name in self._extract_target_names(target):
                                tainted_symbols[name] = "direct-input"
                if call_name in self._DANGEROUS_CALLS:
                    severity, message = self._DANGEROUS_CALLS[call_name]
                    if any(self._expression_uses_tainted(arg, tainted_symbols) for arg in node.args):
                        severity = self._escalate_severity(severity)
                        evidence = "tainted-argument"
                    else:
                        evidence = None
                    issues.append(
                        AuditIssue(
                            rule=f"CALL::{call_name}",
                            severity=severity,
                            message=message,
                            location=self._node_location(node),
                            remediation="Validate input or remove the dangerous primitive.",
                            evidence=evidence,
                        )
                    )

            elif isinstance(node, ast.Constant) and isinstance(node.value, str):
                if "PRIVATE KEY" in node.value:
                    issues.append(
                        AuditIssue(
                            rule="SECRET::PRIVATE_KEY",
                            severity="High",
                            message="Hard-coded private key detected.",
                            location=self._node_location(node),
                            remediation="Remove secrets from source control; load from secure storage.",
                        )
                    )

        tainted = sorted(tainted_symbols)
        return issues, tainted

    # ------------------------------------------------------------------
    def _run_dynamic_checks(self, source: str) -> List[AuditIssue]:
        safe_builtins = dict(self._SAFE_BUILTINS)

        def _blocked(name: str) -> None:
            raise DangerousOperation(f"Blocked dangerous builtin: {name}")

        for blocked in ["eval", "exec", "open", "compile", "input"]:
            safe_builtins[blocked] = lambda *_, __name=blocked, **__: _blocked(__name)

        safe_builtins["__import__"] = self._safe_import

        module_globals: Dict[str, Any] = {"__builtins__": safe_builtins}

        max_runtime = float(self.config.max_runtime_seconds)
        if max_runtime <= 0:
            return []

        deadline = time.monotonic() + max_runtime

        def _ensure(stage: str) -> None:
            self._ensure_time_budget(deadline, stage)

        try:
            with self._time_budget(deadline):
                _ensure("compile")
                try:
                    compiled = compile(source, "<coaudit>", "exec")
                    exec(compiled, module_globals, module_globals)  # noqa: S102 - executed in sandbox
                except DynamicAnalysisTimeout:
                    raise
                except DangerousOperation as exc:
                    return [
                        AuditIssue(
                            rule="DYNAMIC::blocked",
                            severity="High",
                            message="Sandbox prevented dangerous operation during import stage.",
                            evidence=str(exc),
                            dynamic=True,
                        )
                    ]
                except Exception as exc:  # pragma: no cover - unexpected runtime issues
                    return [
                        AuditIssue(
                            rule="DYNAMIC::load-error",
                            severity="Medium",
                            message="Module failed to load in sandbox.",
                            evidence=str(exc),
                            dynamic=True,
                        )
                    ]

                fuzzer = AttackSurfaceFuzzer(seed=1337, max_payloads=self.config.max_fuzz_payloads)
                dynamic_issues: List[AuditIssue] = []

                for name, obj in list(module_globals.items()):
                    _ensure(f"inspect::{name}")
                    if not inspect.isfunction(obj):
                        continue
                    if inspect.iscoroutinefunction(obj):
                        continue

                    signature = inspect.signature(obj)
                    params = [
                        p
                        for p in signature.parameters.values()
                        if p.kind in (p.POSITIONAL_ONLY, p.POSITIONAL_OR_KEYWORD)
                    ]
                    required = [p for p in params if p.default is inspect._empty]

                    if len(required) > 1:
                        continue

                    if len(required) == 0:
                        try:
                            obj()
                            _ensure(f"post-call::{name}")
                        except DangerousOperation as exc:
                            dynamic_issues.append(
                                AuditIssue(
                                    rule=f"DYNAMIC::{name}",
                                    severity="High",
                                    message="Parameter-less callable triggered dangerous primitive.",
                                    evidence=str(exc),
                                    dynamic=True,
                                )
                            )
                        except DynamicAnalysisTimeout:
                            raise
                        except Exception as exc:  # pragma: no cover - unexpected
                            dynamic_issues.append(
                                AuditIssue(
                                    rule=f"DYNAMIC::{name}",
                                    severity="Medium",
                                    message="Callable raised exception without input.",
                                    evidence=str(exc),
                                    dynamic=True,
                                )
                            )
                        continue

                    payloads = fuzzer.generate_payloads(name)
                    _ensure(f"payloads::{name}")

                    def _call(payload: str, fn=obj, fn_name=name) -> None:
                        _ensure(f"invoke::{fn_name}")
                        result = fn(payload)
                        if inspect.isawaitable(result):  # pragma: no cover - async not expected
                            raise DangerousOperation("async-callable")

                    fuzz_results = fuzzer.fuzz_callable(_call, payloads)
                    _ensure(f"fuzz::{name}")
                    for fuzz_result in fuzz_results:
                        if isinstance(fuzz_result.exception, DynamicAnalysisTimeout):
                            raise fuzz_result.exception
                        if fuzz_result.triggered:
                            severity = (
                                "Critical"
                                if isinstance(fuzz_result.exception, DangerousOperation)
                                else "High"
                            )
                            dynamic_issues.append(
                                AuditIssue(
                                    rule=f"FUZZ::{name}",
                                    severity=severity,
                                    message="Fuzzer triggered exceptional behaviour.",
                                    evidence=f"payload={fuzz_result.payload}",
                                    dynamic=True,
                                )
                            )

                return dynamic_issues
        except DynamicAnalysisTimeout as exc:
            return [
                AuditIssue(
                    rule="DYNAMIC::timeout",
                    severity="Critical",
                    message="Dynamic analysis exceeded the configured time budget.",
                    evidence=f"limit={self.config.max_runtime_seconds}s stage={exc.stage}",
                    dynamic=True,
                )
            ]

    # ------------------------------------------------------------------
    def _safe_import(self, name: str, *args: Any, **kwargs: Any) -> Any:
        allowed = {"math", "statistics", "random"}
        if name not in allowed:
            raise DangerousOperation(f"import-blocked:{name}")
        return builtins.__import__(name, *args, **kwargs)

    # ------------------------------------------------------------------
    def _determine_status(self, issues: Iterable[AuditIssue]) -> str:
        strict = self.config.strictness_level.lower()
        threshold = 3  # default for Standard
        if strict == "strict":
            threshold = 2
        elif strict == "lenient":
            threshold = 4

        medium_count = 0
        for issue in issues:
            weight = self._SEVERITY_WEIGHT.get(issue.severity, 0)
            if weight >= threshold:
                return "FAIL"
            if issue.severity == "Medium":
                medium_count += 1
        if strict != "lenient" and medium_count > 2:
            return "FAIL"
        return "PASS"

    # ------------------------------------------------------------------
    @staticmethod
    def _ensure_time_budget(deadline: float, stage: str) -> None:
        if time.monotonic() > deadline:
            raise DynamicAnalysisTimeout(stage)

    # ------------------------------------------------------------------
    @staticmethod
    @contextmanager
    def _time_budget(deadline: float):
        tracer = CoAuditAI._DeadlineTracer(deadline)
        previous_trace = sys.gettrace()
        previous_thread_trace = threading.gettrace() if hasattr(threading, "gettrace") else None
        sys.settrace(tracer)
        if hasattr(threading, "settrace"):
            threading.settrace(tracer)
        try:
            yield
        finally:
            sys.settrace(previous_trace)
            if hasattr(threading, "settrace"):
                threading.settrace(previous_thread_trace)

    # ------------------------------------------------------------------
    class _DeadlineTracer:
        def __init__(self, deadline: float) -> None:
            self.deadline = deadline

        def __call__(self, frame, event, arg):  # noqa: D401 - simple tracer callable
            if time.monotonic() > self.deadline:
                raise DynamicAnalysisTimeout("execution")
            return self

    # ------------------------------------------------------------------
    @staticmethod
    def _node_location(node: ast.AST) -> str:
        line = getattr(node, "lineno", "?")
        col = getattr(node, "col_offset", "?")
        return f"line {line}:{col}"

    @staticmethod
    def _call_name(call: ast.Call) -> str:
        func = call.func
        if isinstance(func, ast.Name):
            return func.id
        if isinstance(func, ast.Attribute):
            parts = []
            while isinstance(func, ast.Attribute):
                parts.append(func.attr)
                func = func.value
            if isinstance(func, ast.Name):
                parts.append(func.id)
            parts.reverse()
            return ".".join(parts)
        return "unknown"

    @staticmethod
    def _extract_names(node: ast.AST) -> List[str]:
        names: List[str] = []
        for sub in ast.walk(node):
            if isinstance(sub, ast.Name):
                names.append(sub.id)
        return names

    @staticmethod
    def _extract_target_names(target: ast.AST) -> List[str]:
        if isinstance(target, ast.Name):
            return [target.id]
        if isinstance(target, (ast.Tuple, ast.List)):
            names: List[str] = []
            for elt in target.elts:
                names.extend(CoAuditAI._extract_target_names(elt))
            return names
        return []

    @staticmethod
    def _expression_uses_tainted(node: ast.AST, tainted: Dict[str, str]) -> bool:
        return any(name in tainted for name in CoAuditAI._extract_names(node))

    @staticmethod
    def _escalate_severity(severity: str) -> str:
        order = ["Info", "Low", "Medium", "High", "Critical"]
        try:
            idx = order.index(severity)
        except ValueError:
            return severity
        return order[min(len(order) - 1, idx + 1)]

    @staticmethod
    def _nearest_parent_assign(node: ast.AST) -> Optional[ast.Assign]:
        parent = getattr(node, "parent", None)
        while parent is not None:
            if isinstance(parent, ast.Assign):
                return parent
            parent = getattr(parent, "parent", None)
        return None


###############################################################################
# AST parent linking utility                                                   #
###############################################################################


def _link_parents(tree: ast.AST) -> None:
    for node in ast.walk(tree):
        for child in ast.iter_child_nodes(node):
            setattr(child, "parent", node)
