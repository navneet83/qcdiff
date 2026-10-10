# qcdiff

Semantic diff for OpenQASM 3 circuits: a fast, human-readable report of **what
changed** between two quantum programs, at the operation level instead of the
text level. Built for CI pipelines and code review.

```text
$ qcdiff old.qasm new.qasm
--- old.qasm
+++ new.qasm
qubits: 3 -> 3 | bits: 3 -> 3 | 2q-gates: 2 -> 2 | gate-defs: 0 -> 0
ops: 8 -> 8 | +0 -0 ~1

  h q[0]
  cx q[0],q[1]
  cx q[1],q[2]
~ rz q[2] (0.5) => (0.75)
  barrier q[0],q[1],q[2]
  measure q[0] -> c[0]
  measure q[1] -> c[1]
  measure q[2] -> c[2]
```

Textual diff is useless for circuits: equivalent programs serialize with
different whitespace, comments, and declaration order, and transpilers rewrite
files wholesale. `qcdiff` parses both files with
[Qiskit's OpenQASM 3 parser](https://github.com/Qiskit/openqasm3_parser),
extracts a normalized operation stream, and aligns the two streams so that:

- comment, whitespace, and declaration-order changes are **not** differences;
- an inserted or removed gate shows as one `+`/`-` line;
- a parameter-only change shows as one `~` line with old and new values;
- summary counters (qubit/bit counts, two-qubit gate count, op totals) give
  the one-glance answer.

## What qcdiff is not

`qcdiff` tells you *what changed*, not *whether the circuits are equivalent*.
Two circuits can differ textually and still implement the same unitary; for
formal equivalence checking use
[mqt-qcec](https://github.com/munich-quantum-toolkit/qcec). Think `diff`
versus a checksum: complementary tools.

## Install

```sh
cargo install --path .
```

## Usage

```sh
qcdiff old.qasm new.qasm            # full diff
qcdiff old.qasm new.qasm --quiet    # summary only (CI logs)
qcdiff old.qasm new.qasm --no-color
```

Exit codes: `0` streams identical, `1` differences found, `2` parse or usage
error — so it drops straight into CI:

```yaml
# GitHub Actions: fail when the transpiled circuit drifts from the checked-in
# reference (e.g. after a qiskit upgrade).
- name: Circuit regression check
  run: qcdiff reference/ghz.qasm build/ghz.qasm --quiet
```

## Limitations (v0, honest list)

- **OpenQASM 3 only.** OpenQASM 2 files are rejected with exit 2; convert
  first (`qiskit.qasm3.dumps(QuantumCircuit.from_qasm_file(...))`).
- **Control flow is opaque.** `if`/`for`/`while`/`switch` blocks participate
  as single units keyed on the condition, with the body content-hashed: a
  body edit reports as one modified unit, not a nested diff.
- **Gate definitions** (`gate my_gate ...`) likewise diff as named units, not
  body-by-body.
- **No commutation awareness.** Reordering two commuting gates reports as a
  change. Future work.
- **Register renames are changes.** Renaming `q` to `qr` with identical
  structure reports every touching op as changed; structural matching under
  renaming is future work.
- **No line numbers.** The semantic layer of `openqasm3_parser` 0.7 does not
  carry source spans, so the diff shows stream order, not file lines.
- Index expressions and a few exotic expression forms are compared by content
  hash rather than pretty-printed.

## How it works

1. Parse both files with `oq3_semantics` (syntax + semantic analysis; any
   parse or semantic error aborts with the parser's diagnostics).
2. Lower the program to a linear stream of operations — gate calls (with
   modifiers folded into the name), measures, resets, barriers, delays,
   opaque control-flow units — each as `(kind, name, operands, params)` with
   canonical, whitespace-free rendering of operands and parameter
   expressions.
3. Align the streams with a patience diff over `(kind, name, operands)` keys,
   then compare parameters within aligned pairs: equal → unchanged, different
   → `~` modified.

## License

Apache-2.0
