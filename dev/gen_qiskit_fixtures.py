"""Generate realistic before/after OpenQASM 3 pairs for qcdiff battle-testing."""
import os
from qiskit import QuantumCircuit, transpile
from qiskit.circuit.library import QFTGate, efficient_su2
from qiskit.providers.fake_provider import GenericBackendV2
from qiskit import qasm3

OUT = "/tmp/qk/fixtures"
os.makedirs(OUT, exist_ok=True)

def dump(circ, name):
    with open(f"{OUT}/{name}.qasm", "w") as f:
        f.write(qasm3.dumps(circ))
    print(name, "->", circ.size(), "ops, depth", circ.depth())

backend = GenericBackendV2(num_qubits=8, seed=7)

# Base circuit: QFT-5 with measurements
qft = QuantumCircuit(5, 5)
qft.append(QFTGate(5), range(5))
qft.measure(range(5), range(5))

# Case A: optimization level 1 vs 3 (same seed)
dump(transpile(qft, backend, optimization_level=1, seed_transpiler=42), "qft_o1")
dump(transpile(qft, backend, optimization_level=3, seed_transpiler=42), "qft_o3")

# Case B: same everything, different transpiler seed (layout/routing drift)
dump(transpile(qft, backend, optimization_level=2, seed_transpiler=1), "qft_seed1")
dump(transpile(qft, backend, optimization_level=2, seed_transpiler=2), "qft_seed2")

# Case C: determinism — same seed twice (MUST be identical => exit 0)
dump(transpile(qft, backend, optimization_level=2, seed_transpiler=9), "qft_det_a")
dump(transpile(qft, backend, optimization_level=2, seed_transpiler=9), "qft_det_b")

# Case D: CI regression case — ansatz with one parameter bound differently
ansatz = efficient_su2(4, reps=2)
params_a = [0.1 * i for i in range(len(ansatz.parameters))]
params_b = list(params_a)
params_b[7] += 0.05  # one parameter tweaked
dump(ansatz.assign_parameters(params_a), "ansatz_a")
dump(ansatz.assign_parameters(params_b), "ansatz_b")

print("done")
