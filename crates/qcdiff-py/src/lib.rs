//! Python bindings for qcdiff: semantic diff for OpenQASM 3 circuits.

use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use qcdiff_core as core;

create_exception!(
    qcdiff,
    QasmError,
    PyValueError,
    "The source could not be parsed as OpenQASM 3."
);

/// Counts describing one program.
#[pyclass(get_all, frozen)]
#[derive(Clone)]
struct Summary {
    qubits: usize,
    bits: usize,
    total_ops: usize,
    two_qubit_gates: usize,
    gate_defs: usize,
}

#[pymethods]
impl Summary {
    fn __repr__(&self) -> String {
        format!(
            "Summary(qubits={}, bits={}, total_ops={}, two_qubit_gates={}, gate_defs={})",
            self.qubits, self.bits, self.total_ops, self.two_qubit_gates, self.gate_defs
        )
    }
}

impl From<&core::Summary> for Summary {
    fn from(s: &core::Summary) -> Summary {
        Summary {
            qubits: s.qubits,
            bits: s.bits,
            total_ops: s.total_ops,
            two_qubit_gates: s.two_qubit_gates,
            gate_defs: s.gate_defs,
        }
    }
}

/// One aligned line of the diff. `change` is "same", "added", "removed",
/// or "modified"; `new_params` is set for "modified" lines.
#[pyclass(get_all, frozen)]
#[derive(Clone)]
struct Line {
    change: String,
    text: String,
    new_params: Option<String>,
}

#[pymethods]
impl Line {
    fn __repr__(&self) -> String {
        format!("Line({:?}, {:?})", self.change, self.text)
    }

    fn __str__(&self) -> String {
        let marker = match self.change.as_str() {
            "added" => "+",
            "removed" => "-",
            "modified" => "~",
            _ => " ",
        };
        match (self.change.as_str(), self.new_params.as_deref()) {
            ("modified", Some("") | None) => format!("{marker} {} => none", self.text),
            ("modified", Some(np)) => format!("{marker} {} => ({np})", self.text),
            _ => format!("{marker} {}", self.text),
        }
    }
}

/// Result of diffing two circuits.
#[pyclass(frozen)]
struct Report {
    #[pyo3(get)]
    added: usize,
    #[pyo3(get)]
    removed: usize,
    #[pyo3(get)]
    modified: usize,
    #[pyo3(get)]
    old_summary: Summary,
    #[pyo3(get)]
    new_summary: Summary,
    lines: Vec<Line>,
}

#[pymethods]
impl Report {
    /// True when the operation streams differ.
    fn has_changes(&self) -> bool {
        self.added + self.removed + self.modified > 0
    }

    /// Truthiness follows has_changes(), so `assert not qcdiff.diff_files(a, b)`
    /// reads as "assert the circuits did not drift".
    fn __bool__(&self) -> bool {
        self.has_changes()
    }

    #[getter]
    fn lines(&self) -> Vec<Line> {
        self.lines.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Report(added={}, removed={}, modified={})",
            self.added, self.removed, self.modified
        )
    }

    /// The same text the qcdiff CLI prints, without the file header.
    fn __str__(&self) -> String {
        let a = &self.old_summary;
        let b = &self.new_summary;
        let mut out = format!(
            "qubits: {} -> {} | bits: {} -> {} | 2q-gates: {} -> {} | gate-defs: {} -> {}\n\
             ops: {} -> {} | +{} -{} ~{}\n",
            a.qubits,
            b.qubits,
            a.bits,
            b.bits,
            a.two_qubit_gates,
            b.two_qubit_gates,
            a.gate_defs,
            b.gate_defs,
            a.total_ops,
            b.total_ops,
            self.added,
            self.removed,
            self.modified
        );
        if self.has_changes() {
            out.push('\n');
            for line in &self.lines {
                out.push_str(&line.__str__());
                out.push('\n');
            }
        }
        out
    }
}

fn extract(source: &str, name: &str) -> PyResult<core::Extracted> {
    core::extract_source(source, Some(name)).map_err(|e| match e {
        core::Error::Qasm2 => QasmError::new_err(format!(
            "{name} declares OPENQASM 2.0, which qcdiff does not support; \
             convert with qiskit.qasm3.dumps(QuantumCircuit.from_qasm_file(...))"
        )),
        core::Error::Parse(errors) => {
            // The parser's diagnostics only render to stdout; emit them there
            // so pytest captures them next to the raised error.
            errors.print();
            QasmError::new_err(format!(
                "{name} has parse or semantic errors (diagnostics on stdout)"
            ))
        }
    })
}

fn report(old: &core::Extracted, new: &core::Extracted) -> Report {
    let result = core::diff(old, new);
    Report {
        added: result.added,
        removed: result.removed,
        modified: result.modified,
        old_summary: Summary::from(&old.summary),
        new_summary: Summary::from(&new.summary),
        lines: result
            .lines
            .into_iter()
            .map(|l| Line {
                change: match l.change {
                    core::Change::Same => "same",
                    core::Change::Added => "added",
                    core::Change::Removed => "removed",
                    core::Change::Modified => "modified",
                }
                .to_string(),
                text: l.text,
                new_params: l.new_params,
            })
            .collect(),
    }
}

/// Diff two OpenQASM 3 programs given as strings.
#[pyfunction]
#[pyo3(signature = (old, new, old_name="old", new_name="new"))]
fn diff_sources(old: &str, new: &str, old_name: &str, new_name: &str) -> PyResult<Report> {
    Ok(report(&extract(old, old_name)?, &extract(new, new_name)?))
}

/// Diff two OpenQASM 3 files.
#[pyfunction]
fn diff_files(old_path: &str, new_path: &str) -> PyResult<Report> {
    let read = |p: &str| {
        std::fs::read_to_string(p)
            .map_err(|e| PyValueError::new_err(format!("cannot read {p}: {e}")))
    };
    Ok(report(
        &extract(&read(old_path)?, old_path)?,
        &extract(&read(new_path)?, new_path)?,
    ))
}

#[pymodule]
fn qcdiff(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(diff_sources, m)?)?;
    m.add_function(wrap_pyfunction!(diff_files, m)?)?;
    m.add_class::<Report>()?;
    m.add_class::<Summary>()?;
    m.add_class::<Line>()?;
    m.add("QasmError", py.get_type::<QasmError>())?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
