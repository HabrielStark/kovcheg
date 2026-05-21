import json
from datetime import datetime, timezone
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')


def generate():
    data = {
        "id": "NT-03",
        "description": "Photon-Side-Scatter",
        "spectral_range_nm": [400, 1600],
        "generated": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "pass_rule": "Scatter <= 0.01 % full energy",
    }
    return data


def main():
    OUTPUT.write_text(json.dumps(generate(), indent=2))
    print(f"NT-03 vector generated → {OUTPUT}")


if __name__ == "__main__":
    main() 