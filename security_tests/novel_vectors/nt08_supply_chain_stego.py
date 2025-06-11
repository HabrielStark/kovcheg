import json
from datetime import datetime
from pathlib import Path

OUTPUT = Path(__file__).with_suffix('.json')

def generate():
    return {
        "id": "NT-08",
        "description": "Supply-Chain Stego (dopant-level)",
        "sem_edx_required": True,
        "generated": datetime.utcnow().isoformat() + "Z",
    }

if __name__ == "__main__":
    OUTPUT.write_text(json.dumps(generate(), indent=2))
    print(f"NT-08 vector generated → {OUTPUT}") 