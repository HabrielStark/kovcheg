import json
from datetime import datetime
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')

def generate(sigma_us: int = 5):
    return {
        "id": "NT-07",
        "description": "Time-Shifted Patch",
        "sigma_us": sigma_us,
        "generated": datetime.utcnow().isoformat() + "Z",
    }

if __name__ == "__main__":
    OUTPUT.write_text(json.dumps(generate(), indent=2))
    print(f"NT-07 vector generated → {OUTPUT}") 