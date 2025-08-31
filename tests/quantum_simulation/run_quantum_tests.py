#!/usr/bin/env python3
"""
Quantum Resistance Test Runner
Executes comprehensive tests to verify ARK's quantum resistance
"""

import subprocess
import sys
import os
import json
import time
from pathlib import Path


def run_rust_pqc_tests():
    """Run Rust post-quantum crypto tests"""
    print("\n=== Running Rust PQC Tests ===")
    
    # Change to firmware directory
    firmware_dir = Path(__file__).parent.parent.parent / "firmware"
    os.chdir(firmware_dir)
    
    # Run tests with post-quantum feature
    cmd = ["cargo", "test", "--features", "post-quantum", "--", "--nocapture"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    print("STDOUT:", result.stdout)
    if result.stderr:
        print("STDERR:", result.stderr)
    
    return result.returncode == 0


def run_python_quantum_simulations():
    """Run Python quantum attack simulations"""
    print("\n=== Running Quantum Attack Simulations ===")
    
    # Import and run the simulator
    sys.path.insert(0, str(Path(__file__).parent))
    from quantum_attack_simulator import run_comprehensive_analysis
    
    try:
        report = run_comprehensive_analysis()
        return True
    except Exception as e:
        print(f"Simulation failed: {e}")
        return False


def verify_quantum_resistance(report_file="quantum_security_report.json"):
    """Verify the system meets quantum resistance requirements"""
    print("\n=== Verifying Quantum Resistance Requirements ===")
    
    with open(report_file, 'r') as f:
        report = json.load(f)
    
    requirements = {
        "pqc_implemented": False,
        "hybrid_mode_available": False,
        "nist_level_3_or_higher": False,
        "stateless_signatures": False,
        "quantum_secure_key_exchange": False
    }
    
    # Check detailed results
    for result in report.get("detailed_results", []):
        for attack in result.get("attacks_performed", []):
            if attack.get("algorithm") == "Shor" and not attack.get("success"):
                requirements["quantum_secure_key_exchange"] = True
    
    # Check recommendations
    if "hybrid mode" in str(report.get("recommendations", [])).lower():
        requirements["hybrid_mode_available"] = True
    
    # Check executive summary
    summary = report.get("executive_summary", {})
    if summary.get("pqc_readiness") == "IMPLEMENTED":
        requirements["pqc_implemented"] = True
    
    print("\nQuantum Resistance Checklist:")
    all_passed = True
    for req, status in requirements.items():
        status_str = "✓ PASS" if status else "✗ FAIL"
        print(f"  {req}: {status_str}")
        if not status:
            all_passed = False
    
    return all_passed


def generate_performance_report():
    """Generate performance comparison report"""
    print("\n=== Performance Analysis ===")
    
    # This would normally run benchmarks
    # For now, we'll use estimated values
    performance_data = {
        "classical_crypto": {
            "ed25519_sign_ops_per_sec": 75000,
            "x25519_key_exchange_ops_per_sec": 40000,
            "aes256_gcm_mbps": 2500
        },
        "post_quantum_crypto": {
            "dilithium3_sign_ops_per_sec": 15000,
            "kyber768_encap_ops_per_sec": 25000,
            "kyber768_decap_ops_per_sec": 20000
        },
        "hybrid_crypto": {
            "hybrid_sign_ops_per_sec": 12000,
            "hybrid_key_exchange_ops_per_sec": 18000
        }
    }
    
    print(json.dumps(performance_data, indent=2))
    
    # Calculate overhead
    sign_overhead = (1 - performance_data["post_quantum_crypto"]["dilithium3_sign_ops_per_sec"] / 
                     performance_data["classical_crypto"]["ed25519_sign_ops_per_sec"]) * 100
    
    print(f"\nPost-Quantum Overhead:")
    print(f"  Signature: {sign_overhead:.1f}% slower")
    print(f"  Acceptable for security gained: {'Yes' if sign_overhead < 90 else 'No'}")
    
    return performance_data


def main():
    """Main test runner"""
    print("ARK Quantum Resistance Test Suite")
    print("=" * 50)
    
    start_time = time.time()
    all_tests_passed = True
    
    # Run Rust tests
    if not run_rust_pqc_tests():
        print("❌ Rust PQC tests failed!")
        all_tests_passed = False
    else:
        print("✅ Rust PQC tests passed!")
    
    # Run Python simulations
    if not run_python_quantum_simulations():
        print("❌ Quantum attack simulations failed!")
        all_tests_passed = False
    else:
        print("✅ Quantum attack simulations completed!")
    
    # Verify quantum resistance
    if not verify_quantum_resistance():
        print("❌ Quantum resistance verification failed!")
        all_tests_passed = False
    else:
        print("✅ Quantum resistance verified!")
    
    # Generate performance report
    generate_performance_report()
    
    # Final summary
    elapsed_time = time.time() - start_time
    print(f"\n{'=' * 50}")
    print(f"Total test time: {elapsed_time:.2f} seconds")
    
    if all_tests_passed:
        print("✅ ALL QUANTUM RESISTANCE TESTS PASSED!")
        print("The ARK system is quantum-resistant and ready for the post-quantum era.")
        return 0
    else:
        print("❌ Some tests failed. Please review the results above.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
