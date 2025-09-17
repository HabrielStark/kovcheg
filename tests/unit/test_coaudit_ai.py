import textwrap
from pathlib import Path

from software.co_audit_ai.src.lib import CoAuditAI, CoAuditConfig, FindingSeverity


def _write_module(tmp_path: Path, code: str) -> Path:
    module_path = tmp_path / "audit_target.py"
    module_path.write_text(textwrap.dedent(code), encoding="utf-8")
    return module_path


def test_coaudit_detects_dangerous_sink(tmp_path):
    code = """
    import os

    def user_input_handler(user_payload: str):
        return user_payload

    def execute_payload(payload: str):
        return eval(payload)

    def orchestrator(payload: str):
        command = user_input_handler(payload)
        return os.system(command)
    """
    module = _write_module(tmp_path, code)
    config = CoAuditConfig(entry_points=["execute_payload", "orchestrator"], fuzzer_seed=7)
    report = CoAuditAI(config=config).analyze(module)
    assert any(f.kind == "dangerous_sink" for f in report.findings)
    assert report.risk_score >= 12
    assert report.fuzz_outcomes
    assert "execute_payload" in report.call_graph
    summary = report.summary()
    assert summary["findings"][0]["severity"] in {"high", "critical"}


def test_coaudit_respects_allowed_sinks(tmp_path):
    code = """
    def sanitize(value: str) -> str:
        return value.replace(" ", "")

    def render_template(user_input: str) -> str:
        safe = sanitize(user_input)
        return f"<div>{safe}</div>"
    """
    module = _write_module(tmp_path, code)
    config = CoAuditConfig(entry_points=["render_template"], allowed_sinks=("render_template",), enable_fuzzer=False)
    report = CoAuditAI(config=config).analyze(module)
    assert not report.findings
    assert report.risk_score == 0
