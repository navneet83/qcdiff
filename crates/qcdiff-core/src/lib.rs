//! Semantic diff for OpenQASM 3 circuits.
//!
//! Parses two programs with Qiskit's [`oq3_semantics`], lowers each to a
//! normalized stream of operations, and aligns the streams so that comment,
//! whitespace, and declaration-order changes do not count as differences.
//! This is the library behind the `qcdiff` CLI.
//!
//! ```
//! use qcdiff_core::{diff, extract_source};
//!
//! let old = extract_source(
//!     "OPENQASM 3.0; include \"stdgates.inc\"; qubit q; h q;",
//!     Some("old.qasm"),
//! )
//! .unwrap();
//! let new = extract_source(
//!     "OPENQASM 3.0; include \"stdgates.inc\"; qubit q; x q;",
//!     Some("new.qasm"),
//! )
//! .unwrap();
//!
//! let result = diff(&old, &new);
//! assert!(result.has_changes());
//! ```

mod diff;
mod stream;

pub use diff::{diff, Change, DiffLine, DiffResult};
pub use stream::{extract, Extracted, Op, Summary};

use oq3_semantics::syntax_to_semantics::{parse_source_string, ParseResult};
use oq3_source_file::SourceString;

/// Why a source could not be turned into an operation stream.
pub enum Error {
    /// The version statement declares OpenQASM 2, which has different
    /// grammar and semantics.
    Qasm2,
    /// Parse or semantic errors; print them with [`ParseErrors::print`].
    Parse(ParseErrors),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Qasm2 => write!(f, "OpenQASM 2 is not supported"),
            Error::Parse(_) => write!(f, "parse or semantic errors"),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::error::Error for Error {}

/// Diagnostics from one source, kept so the caller decides when to print.
/// Boxed: the parse result is large and lives on the error path only.
pub struct ParseErrors(Box<ParseResult<SourceString>>);

impl ParseErrors {
    /// Pretty-print the parser's diagnostics.
    pub fn print(&self) {
        self.0.print_errors();
    }
}

/// Parse OpenQASM 3 source and extract its normalized operation stream.
/// `name` is used in diagnostics only.
pub fn extract_source(source: &str, name: Option<&str>) -> Result<Extracted, Error> {
    if is_qasm2(source) {
        return Err(Error::Qasm2);
    }
    let search: Option<&[&str]> = None;
    let parsed = parse_source_string(source, name, search);
    if parsed.any_errors() {
        return Err(Error::Parse(ParseErrors(Box::new(parsed))));
    }
    Ok(extract(parsed.program(), parsed.symbol_table()))
}

/// True when the version statement declares major version 2. Decided on the
/// parsed-out version line rather than substring search over the whole file.
pub fn is_qasm2(source: &str) -> bool {
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
