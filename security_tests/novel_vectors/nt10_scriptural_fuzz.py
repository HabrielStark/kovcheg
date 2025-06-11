import json, random
from datetime import datetime
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')

SCRIPTURE_VARIANTS = [
    "Thou shalt not kill",
    "You shall not murder",
    "Не убивай",
    "Не убий",
]

def generate(inputs: int = 1_000_000):
    sample = random.sample(SCRIPTURE_VARIANTS * (inputs // len(SCRIPTURE_VARIANTS)), k=10)
    return {
        "id": "NT-10",
        "description": "Scriptural-Fuzz",
        "inputs": inputs,
        "sample_variants": sample,
        "generated": datetime.utcnow().isoformat() + "Z",
    }

if __name__ == "__main__":
    OUTPUT.write_text(json.dumps(generate(), indent=2))
    print(f"NT-10 vector generated → {OUTPUT}") 