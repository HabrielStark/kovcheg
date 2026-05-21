import json
from datetime import datetime, timezone
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')

def generate(tx_per_min: int = 1000):
    return {
        "id": "NT-09",
        "description": "Bounty-DAO Stress",
        "tx_per_min": tx_per_min,
        "sim_duration_min": 60,
        "generated": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    }

if __name__ == "__main__":
    OUTPUT.write_text(json.dumps(generate(), indent=2))
    print(f"NT-09 vector generated → {OUTPUT}") 