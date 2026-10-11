use assert_cmd::Command;
use predicates::prelude::*;

fn qcdiff() -> Command {
    Command::cargo_bin("qcdiff").unwrap()
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn identical_files_exit_zero() {
    qcdiff()
        .args([fixture("base.qasm"), fixture("identical.qasm")])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("+0 -0 ~0"));
}

#[test]
fn same_file_twice_exits_zero() {
    qcdiff()
        .args([fixture("base.qasm"), fixture("base.qasm")])
        .assert()
        .code(0);
}

#[test]
fn reordered_declarations_and_whitespace_are_not_changes() {
    qcdiff()
        .args([fixture("base.qasm"), fixture("reordered_decls.qasm")])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("+0 -0 ~0"));
}

#[test]
fn inserted_gate_detected_exactly_once() {
    qcdiff()
        .args([
            fixture("base.qasm"),
            fixture("inserted_gate.qasm"),
            "--no-color".into(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("+1 -0 ~0"))
        .stdout(predicate::str::contains("+ x q[1]"));
}

#[test]
fn removed_gate_detected() {
    // Reverse direction of the insert fixture: the x gate is a removal.
    qcdiff()
        .args([
            fixture("inserted_gate.qasm"),
            fixture("base.qasm"),
            "--no-color".into(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("+0 -1 ~0"))
        .stdout(predicate::str::contains("- x q[1]"));
}

#[test]
fn parameter_change_is_modification_not_insert_delete() {
    qcdiff()
        .args([
            fixture("base.qasm"),
            fixture("param_change.qasm"),
            "--no-color".into(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("+0 -0 ~1"))
        .stdout(predicate::str::contains("~ rz q[2] (0.5) => (0.75)"));
}

#[test]
fn quiet_suppresses_the_line_diff() {
    qcdiff()
        .args([
            fixture("base.qasm"),
            fixture("param_change.qasm"),
            "--quiet".into(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("~ rz").not());
}

#[test]
fn malformed_file_exits_two() {
    qcdiff()
        .args([fixture("base.qasm"), fixture("malformed.qasm")])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("errors"));
}

#[test]
fn qasm2_is_rejected_with_guidance() {
    qcdiff()
        .args([fixture("base.qasm"), fixture("qasm2.qasm")])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("OPENQASM 2.0"))
        .stderr(predicate::str::contains("qiskit"));
}

#[test]
fn missing_file_exits_two() {
    qcdiff()
        .args([fixture("base.qasm"), fixture("does_not_exist.qasm")])
        .assert()
        .code(2);
}

#[test]
fn shifted_declaration_does_not_change_control_flow_hash() {
    qcdiff()
        .args([
            fixture("ctrl_flow.qasm"),
            fixture("ctrl_flow_shifted_decl.qasm"),
        ])
        .assert()
        .code(0);
}

#[test]
fn control_flow_body_edit_reports_as_modified() {
    qcdiff()
        .args([
            fixture("ctrl_flow.qasm"),
            fixture("ctrl_flow_body_edit.qasm"),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("~ if"));
}

#[test]
fn removed_params_render_as_none() {
    qcdiff()
        .args([fixture("param_change.qasm"), fixture("base.qasm")])
        .assert()
        .stdout(predicate::str::contains("((none))").not());
}

// Golden fixtures: real `qiskit.transpile()` output (qiskit 2.5.2), committed
// so the suite exercises physical-qubit operands, pi expressions, and
// stdgates includes without needing Python. Regenerate with
// dev/gen_qiskit_fixtures.py.
fn golden(name: &str) -> String {
    format!("tests/golden/{name}.qasm")
}

#[test]
fn golden_same_transpile_seed_is_identical() {
    qcdiff()
        .args([golden("qft_det_a"), golden("qft_det_b")])
        .assert()
        .code(0);
}

#[test]
fn golden_one_param_tweak_is_one_modification() {
    qcdiff()
        .args([golden("ansatz_a"), golden("ansatz_b")])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("+0 -0 ~1"));
}

#[test]
fn golden_optimization_levels_diff_summarizes() {
    qcdiff()
        .args([golden("qft_o1"), golden("qft_o3")])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("2q-gates: 26 -> 20"))
        .stdout(predicate::str::contains("+40 -48 ~6"));
}
