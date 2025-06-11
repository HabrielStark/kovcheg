import numpy as np
import random


def measure_latency_ns(samples: int = 1000) -> float:
    """Mock photonic optic-gate latency measurement.
    Replace with Verilator or oscilloscope interface.
    """
    latencies = np.random.normal(loc=7, scale=1, size=samples)  # mean 7 ns ±1
    worst = float(np.max(latencies))
    return worst

if __name__ == "__main__":
    print(f"Worst-case OG latency: {measure_latency_ns():.2f} ns") 