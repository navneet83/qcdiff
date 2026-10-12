"""Binding tests; fixtures are the CLI crate's committed QASM files."""

import pathlib

import pytest

import qcdiff

ROOT = pathlib.Path(__file__).resolve().parents[2] / "qcdiff" / "tests"
GOLDEN = ROOT / "golden"
FIXTURES = ROOT / "fixtures"


def test_identical_transpile_outputs_have_no_changes():
    r = qcdiff.diff_files(str(GOLDEN / "qft_det_a.qasm"), str(GOLDEN / "qft_det_b.qasm"))
    assert not r.has_changes()
    assert not r  # __bool__ follows has_changes


def test_one_param_tweak_is_one_modification():
    r = qcdiff.diff_files(str(GOLDEN / "ansatz_a.qasm"), str(GOLDEN / "ansatz_b.qasm"))
    assert (r.added, r.removed, r.modified) == (0, 0, 1)
    mods = [line for line in r.lines if line.change == "modified"]
    assert len(mods) == 1
    assert mods[0].new_params


def test_optimization_level_diff_summary():
    r = qcdiff.diff_files(str(GOLDEN / "qft_o1.qasm"), str(GOLDEN / "qft_o3.qasm"))
    assert (r.added, r.removed, r.modified) == (40, 48, 6)
    assert r.old_summary.two_qubit_gates == 26
    assert r.new_summary.two_qubit_gates == 20


def test_diff_sources_and_report_text():
    old = (FIXTURES / "base.qasm").read_text()
    new = (FIXTURES / "param_change.qasm").read_text()
    r = qcdiff.diff_sources(old, new)
    text = str(r)
    assert text.startswith("qubits:")
    assert "~ rz" in text


def test_qasm2_raises():
    with pytest.raises(qcdiff.QasmError, match="OPENQASM 2.0"):
        qcdiff.diff_files(str(FIXTURES / "base.qasm"), str(FIXTURES / "qasm2.qasm"))


def test_parse_error_raises():
    with pytest.raises(qcdiff.QasmError, match="parse or semantic errors"):
        qcdiff.diff_files(str(FIXTURES / "base.qasm"), str(FIXTURES / "malformed.qasm"))
