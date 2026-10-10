//! Alignment and rendering of two operation streams.

use crate::stream::{Extracted, Op};
use similar::{capture_diff_slices, Algorithm, DiffOp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Same,
    Removed,
    Added,
    /// Same alignment key (op + operands) but different parameters/content.
    Modified,
}

#[derive(Debug)]
pub struct DiffLine {
    pub change: Change,
    /// Rendering of the old op (Removed/Same/Modified) or new op (Added).
    pub text: String,
    /// For Modified: rendering of the new parameters.
    pub new_params: Option<String>,
}

pub struct DiffResult {
    pub lines: Vec<DiffLine>,
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
}

impl DiffResult {
    pub fn has_changes(&self) -> bool {
        self.added + self.removed + self.modified > 0
    }
}

/// Diff two streams. Alignment uses the parameter-free key so that a
/// parameter-only change pairs up and reports as `~` instead of `-`/`+`.
pub fn diff(a: &Extracted, b: &Extracted) -> DiffResult {
    let keys_a: Vec<String> = a.ops.iter().map(Op::align_key).collect();
    let keys_b: Vec<String> = b.ops.iter().map(Op::align_key).collect();
    let ops = capture_diff_slices(Algorithm::Patience, &keys_a, &keys_b);

    let mut lines = Vec::new();
    let (mut added, mut removed, mut modified) = (0usize, 0usize, 0usize);

    for op in ops {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for k in 0..len {
                    let oa = &a.ops[old_index + k];
                    let ob = &b.ops[new_index + k];
                    if oa.params == ob.params {
                        lines.push(DiffLine {
                            change: Change::Same,
                            text: oa.render(),
                            new_params: None,
                        });
                    } else {
                        modified += 1;
                        lines.push(DiffLine {
                            change: Change::Modified,
                            text: oa.render(),
                            new_params: Some(if ob.params.is_empty() {
                                "(none)".to_string()
                            } else {
                                ob.params.clone()
                            }),
                        });
                    }
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for k in 0..old_len {
                    removed += 1;
                    lines.push(DiffLine {
                        change: Change::Removed,
                        text: a.ops[old_index + k].render(),
                        new_params: None,
                    });
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for k in 0..new_len {
                    added += 1;
                    lines.push(DiffLine {
                        change: Change::Added,
                        text: b.ops[new_index + k].render(),
                        new_params: None,
                    });
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                for k in 0..old_len {
                    removed += 1;
                    lines.push(DiffLine {
                        change: Change::Removed,
                        text: a.ops[old_index + k].render(),
                        new_params: None,
                    });
                }
                for k in 0..new_len {
                    added += 1;
                    lines.push(DiffLine {
                        change: Change::Added,
                        text: b.ops[new_index + k].render(),
                        new_params: None,
                    });
                }
            }
        }
    }

    DiffResult {
        lines,
        added,
        removed,
        modified,
    }
}
