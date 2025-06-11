import json
from datetime import datetime, timedelta
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')


def generate_scenario(episodes: int = 100_000) -> dict:
    scenario = {
        "id": "NT-02",
        "description": "Ethical-Drift Time-Bomb",
        "episodes": episodes,
        "generated": datetime.utcnow().isoformat() + "Z",
    }
    # outcomes placeholder
    scenario["detected"] = False  # to be filled by integration harness
    return scenario


def main():
    data = generate_scenario()
    OUTPUT.write_text(json.dumps(data, indent=2))
    print(f"NT-02 vector generated → {OUTPUT}")


if __name__ == "__main__":
    main() 