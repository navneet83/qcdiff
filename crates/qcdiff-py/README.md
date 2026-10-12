# qcdiff

Semantic diff for OpenQASM 3 quantum circuits. `qcdiff` compares two QASM programs at the gate level instead of the text level, so comment, whitespace, and declaration-order changes do not count as differences. Built for CI pipelines and code review of transpiler output.

This package is the Python binding of the Rust [qcdiff](https://github.com/navneet83/qcdiff) tool.

## Install

```sh
pip install qcdiff
```

## Usage

```python
import qcdiff

report = qcdiff.diff_files("reference/ghz.qasm", "build/ghz.qasm")
if report.has_changes():
    print(report)          # the same text the CLI prints

# In a pytest regression gate:
assert not qcdiff.diff_files("reference/ghz.qasm", "build/ghz.qasm")
```

With Qiskit circuits, export to QASM 3 first:

```python
from qiskit import qasm3

report = qcdiff.diff_sources(qasm3.dumps(circuit_a), qasm3.dumps(circuit_b))
print(report.added, report.removed, report.modified)
for line in report.lines:
    if line.change != "same":
        print(line)
```

`diff_sources` and `diff_files` raise `qcdiff.QasmError` on OpenQASM 2 input or parse errors.

`qcdiff` reports what changed, not whether the circuits are equivalent; for formal equivalence checking use [mqt-qcec](https://github.com/munich-quantum-toolkit/qcec).

## License

Apache-2.0
