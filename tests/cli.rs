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
