import json
from datetime import datetime
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')

def generate(temp_mK: int = 20):
    return {
        "id": "NT-06",
        "description": "Quantum-Coherence Fault",
        "temperature_mK": temp_mK,
        "microwave_impulse": True,
        "generated": datetime.utcnow().isoformat() + "Z",
    }

if __name__ == "__main__":
    OUTPUT.write_text(json.dumps(generate(), indent=2))
    print(f"NT-06 vector generated → {OUTPUT}") 