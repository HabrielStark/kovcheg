import random
import time


def measure_entropy(duration_sec: float = 1.0) -> float:
    """Mock entropy measurement returning bits/sec.
    Replace with hardware TRNG reading via serial.
    """
    start = time.time()
    # simulate bitstream capture
    bits_captured = int(600_000 * duration_sec * random.uniform(0.95, 1.05))
    time.sleep(duration_sec * 0.1)  # simulate processing delay
    rate = bits_captured / duration_sec
    return rate


if __name__ == "__main__":
    rate = measure_entropy(1.0)
    print(f"Entropy rate: {rate:.0f} bps") 