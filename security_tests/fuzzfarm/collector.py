import json, sys
from datetime import datetime, timedelta
from pathlib import Path

THRESHOLD_HOURS = 24
CRITICAL_LEVEL = "Critical"

ROOT = Path("attack_llm_runs")


def load_entries():
    entries = []
    for p in ROOT.glob("*.json"):
        entries.extend(json.loads(p.read_text()))
    return entries


def main():
    if not ROOT.exists():
        print("No Attack-LLM runs yet; skipping check.")
        return

    now = datetime.utcnow()
    fail = False
    for e in load_entries():
        if e["severity"] == CRITICAL_LEVEL and not e.get("patched"):
            ts = datetime.fromisoformat(e["timestamp"].rstrip("Z"))
            if now - ts > timedelta(hours=THRESHOLD_HOURS):
                print(f"❌ Critical exploit {e['id']} unpatched >24h")
                fail = True
    if fail:
        sys.exit(1)
    print("✅ No outstanding critical exploits.")

if __name__ == "__main__":
    main() 