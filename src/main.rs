mod diff;
mod stream;

use clap::Parser;
use diff::{Change, DiffResult};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use stream::{Extracted, Summary};

/// Semantic diff for OpenQASM 3 circuits: reports what changed between two
/// programs at the operation level, for CI and code review. Not an
/// equivalence checker — for formal equivalence use mqt-qcec.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Old circuit file (OpenQASM 3)
    old: PathBuf,
    /// New circuit file (OpenQASM 3)
    new: PathBuf,
    /// Print the summary only (for CI logs)
    #[arg(short, long)]
    quiet: bool,
    /// Disable colored output
    #[arg(long)]
    no_color: bool,
}

const EXIT_DIFFER: u8 = 1;
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let color = !cli.no_color && std::io::stdout().is_terminal();

    let old = match load(&cli.old) {
        Ok(x) => x,
        Err(code) => return code,
    };
    let new = match load(&cli.new) {
        Ok(x) => x,
        Err(code) => return code,
    };

    print_summary(&cli.old, &old.summary, &cli.new, &new.summary);

    let result = diff::diff(&old, &new);
    println!(
        "ops: {} -> {} | +{} -{} ~{}",
        old.summary.total_ops, new.summary.total_ops, result.added, result.removed, result.modified
    );

    if !cli.quiet && result.has_changes() {
        println!();
        print_diff(&result, color);
    }

    if result.has_changes() {
        ExitCode::from(EXIT_DIFFER)
    } else {
        ExitCode::SUCCESS
    }
}

fn load(path: &Path) -> Result<Extracted, ExitCode> {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", path.display());
            return Err(ExitCode::from(EXIT_ERROR));
        }
    };

    // Honest scope: OpenQASM 2 has different grammar and semantics.
    if is_qasm2(&source) {
        eprintln!(
            "error: {} declares OPENQASM 2.0, which qcdiff does not support yet.\n\
             Convert it to OpenQASM 3 first, e.g. in Python:\n\
             qiskit.qasm3.dumps(qiskit.QuantumCircuit.from_qasm_file(\"{}\"))",
            path.display(),
            path.display()
        );
        return Err(ExitCode::from(EXIT_ERROR));
    }

    let search: Option<&[&str]> = None;
    let parsed = oq3_semantics::syntax_to_semantics::parse_source_string(
        source,
        Some(path.to_string_lossy().as_ref()),
        search,
    );
    if parsed.any_errors() {
        eprintln!("error: {} has parse or semantic errors:", path.display());
        parsed.print_errors();
        return Err(ExitCode::from(EXIT_ERROR));
    }

    Ok(stream::extract(parsed.program(), parsed.symbol_table()))
}

/// True when the version statement declares major version 2. Decided on the
/// parsed-out version line rather than substring search over the whole file.
fn is_qasm2(source: &str) -> bool {
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if let Some(rest) = t.strip_prefix("OPENQASM") {
            let version = rest.trim_start().trim_end_matches(';').trim();
            return version.starts_with('2');
        }
        // First effective statement is not a version declaration: OQ3 allows
        // omitting it; treat as OQ3.
        return false;
    }
    false
}

fn print_summary(path_a: &Path, a: &Summary, path_b: &Path, b: &Summary) {
    println!("--- {}", path_a.display());
    println!("+++ {}", path_b.display());
    println!(
        "qubits: {} -> {} | bits: {} -> {} | 2q-gates: {} -> {} | gate-defs: {} -> {}",
        a.qubits,
        b.qubits,
        a.bits,
        b.bits,
        a.two_qubit_gates,
        b.two_qubit_gates,
        a.gate_defs,
        b.gate_defs
    );
}

fn print_diff(result: &DiffResult, color: bool) {
    let (red, green, yellow, dim, reset) = if color {
        ("\x1b[31m", "\x1b[32m", "\x1b[33m", "\x1b[2m", "\x1b[0m")
    } else {
        ("", "", "", "", "")
    };
    for line in &result.lines {
        match line.change {
            Change::Same => println!("{dim}  {}{reset}", line.text),
            Change::Removed => println!("{red}- {}{reset}", line.text),
            Change::Added => println!("{green}+ {}{reset}", line.text),
            Change::Modified => println!(
                "{yellow}~ {} => ({}){reset}",
                line.text,
                line.new_params.as_deref().unwrap_or("(none)")
            ),
        }
    }
}
