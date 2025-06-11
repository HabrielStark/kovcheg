import json, random
from datetime import datetime
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')

def generate(crp_samples: int = 1_000_000):
    return {
        "id": "NT-05",
        "description": "PUF-ML-Oracle",
        "crp_samples": crp_samples,
        "model_arch": random.choice(["CNN", "Transformer", "MLP"]),
        "generated": datetime.utcnow().isoformat() + "Z",
    }

if __name__ == "__main__":
    OUTPUT.write_text(json.dumps(generate(), indent=2))
    print(f"NT-05 vector generated → {OUTPUT}") 