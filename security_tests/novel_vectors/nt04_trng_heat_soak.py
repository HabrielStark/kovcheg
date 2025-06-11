import json
from datetime import datetime
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')

def generate(hours: int = 190):
    return {
        "id": "NT-04",
        "description": "TRNG-Degeneracy Heat-Soak",
        "duration_hours": hours,
        "temperature_C": 105,
        "test_interval_h": 2,
        "generated": datetime.utcnow().isoformat() + "Z",
    }

if __name__ == "__main__":
    OUTPUT.write_text(json.dumps(generate(), indent=2))
    print(f"NT-04 vector generated → {OUTPUT}") 