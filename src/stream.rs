//! Extraction of a normalized, linear operation stream from an OpenQASM 3
//! semantic ASG. The stream is what gets diffed: declarations and other
//! non-executable statements contribute to summary counts only.

use oq3_semantics::asg;
use oq3_semantics::symbols::{SymbolIdResult, SymbolTable, SymbolType};
use oq3_semantics::types::Type;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// One executable operation in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    /// Operation class: "gate", "measure", "reset", "barrier", "ctrl-flow",
    /// "gatedef", "delay", "assign", "expr".
    pub kind: &'static str,
    /// Gate or construct name ("h", "cx", "if", gate-def name, ...).
    pub name: String,
    /// Canonical operand rendering (qubits, and `-> target` for measure).
    pub operands: String,
    /// Canonical parameter rendering; empty when the op takes none. For
    /// opaque constructs this is a content hash, so a body change shows up
    /// as a parameter-level change (`~`) rather than insert/delete noise.
    pub params: String,
}

impl Op {
    /// Alignment key: everything except parameters. Two ops with equal keys
    /// but different `params` render as a modification (`~`).
    pub fn align_key(&self) -> String {
        format!("{} {} {}", self.kind, self.name, self.operands)
    }

    pub fn render(&self) -> String {
        let mut s = format!("{} {}", self.name, self.operands);
        if !self.params.is_empty() {
            s.push_str(&format!(" ({})", self.params));
        }
        s.trim_end().to_string()
    }
}

/// Counts that describe a whole program, independent of the stream diff.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub qubits: usize,
    pub bits: usize,
    pub total_ops: usize,
    pub two_qubit_gates: usize,
    pub gate_defs: usize,
}

pub struct Extracted {
    pub ops: Vec<Op>,
    pub summary: Summary,
}

fn short_hash<T: Hash>(value: &T) -> String {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    format!("#{:08x}", h.finish() as u32)
}

fn hash_debug<T: std::fmt::Debug>(value: &T) -> String {
    short_hash(&format!("{value:?}"))
}

fn sym_name(id: &SymbolIdResult, table: &SymbolTable) -> String {
    match id {
        Ok(sid) => table[sid].name().to_string(),
        Err(_) => "<unresolved>".to_string(),
    }
}

fn lvalue_str(lv: &asg::LValue, table: &SymbolTable) -> String {
    match lv {
        asg::LValue::Identifier(id) => sym_name(id, table),
        asg::LValue::IndexedIdentifier(ii) => indexed_identifier_str(ii, table),
    }
}

fn indexed_identifier_str(ii: &asg::IndexedIdentifier, table: &SymbolTable) -> String {
    let mut s = sym_name(ii.identifier(), table);
    for idx in ii.indexes() {
        s.push_str(&format!("[{}]", hash_or_indexes(idx, table)));
    }
    s
}

fn hash_or_indexes(idx: &asg::IndexOperator, table: &SymbolTable) -> String {
    match idx {
        asg::IndexOperator::SetExpression(se) => exprs_str(se.expressions(), table),
        asg::IndexOperator::ExpressionList(el) => exprs_str(&el.expressions, table),
    }
}

fn exprs_str(exprs: &[asg::TExpr], table: &SymbolTable) -> String {
    exprs
        .iter()
        .map(|e| expr_str(e, table))
        .collect::<Vec<_>>()
        .join(",")
}

/// Canonical, whitespace-free rendering of an expression. Falls back to a
/// stable content hash for constructs v0 does not render; the hash keeps the
/// diff deterministic without pretending to pretty-print them.
fn expr_str(texpr: &asg::TExpr, table: &SymbolTable) -> String {
    use asg::Expr;
    match texpr.expression() {
        Expr::Literal(lit) => literal_str(lit),
        Expr::Identifier(id) => sym_name(id, table),
        Expr::IndexedIdentifier(ii) => indexed_identifier_str(ii, table),
        Expr::GateOperand(go) => gate_operand_str(go, table),
        Expr::BinaryExpr(be) => format!(
            "({}{}{})",
            expr_str(be.left(), table),
            binary_op_str(be.op()),
            expr_str(be.right(), table)
        ),
        Expr::UnaryExpr(ue) => format!(
            "({}{})",
            unary_op_str(ue.op()),
            expr_str(ue.operand(), table)
        ),
        Expr::MeasureExpression(me) => format!("measure {}", expr_str(me.operand(), table)),
        // IndexExpression exposes no field accessors in oq3_semantics 0.7,
        // so it participates as an opaque content hash.
        Expr::IndexExpression(ie) => hash_debug(&ie),
        Expr::HardwareQubit(hq) => format!("${}", hq.identifier()),
        other => hash_debug(&other),
    }
}

fn literal_str(lit: &asg::Literal) -> String {
    use asg::Literal as L;
    match lit {
        L::Int(i) => format!("{}{}", if *i.sign() { "" } else { "-" }, i.value()),
        L::Float(f) => f.value().to_string(),
        L::Bool(b) => format!("{:?}", b.value()),
        other => hash_debug(&other),
    }
}

fn binary_op_str(op: &asg::BinaryOp) -> &'static str {
    use asg::{ArithOp, BinaryOp, CmpOp};
    match op {
        BinaryOp::ArithOp(a) => match a {
            ArithOp::Add => "+",
            ArithOp::Sub => "-",
            ArithOp::Mul => "*",
            ArithOp::Div => "/",
            ArithOp::Mod => "%",
            _ => "·",
        },
        BinaryOp::CmpOp(c) => match c {
            CmpOp::Eq => "==",
            CmpOp::Neq => "!=",
        },
        BinaryOp::ConcatenationOp => "++",
    }
}

fn unary_op_str(op: &asg::UnaryOp) -> &'static str {
    use asg::UnaryOp;
    match op {
        UnaryOp::Minus => "-",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
    }
}

fn gate_operand_str(go: &asg::GateOperand, table: &SymbolTable) -> String {
    match go {
        asg::GateOperand::Identifier(id) => sym_name(id, table),
        asg::GateOperand::IndexedIdentifier(ii) => indexed_identifier_str(ii, table),
        asg::GateOperand::HardwareQubit(hq) => format!("${}", hq.identifier()),
    }
}

fn modifier_prefix(mods: &[asg::GateModifier], table: &SymbolTable) -> String {
    let mut s = String::new();
    for m in mods {
        match m {
            asg::GateModifier::Inv => s.push_str("inv@"),
            asg::GateModifier::Pow(p) => s.push_str(&format!("pow({})@", expr_str(p, table))),
            asg::GateModifier::Ctrl(n) => match n {
                Some(e) => s.push_str(&format!("ctrl({})@", expr_str(e, table))),
                None => s.push_str("ctrl@"),
            },
            asg::GateModifier::NegCtrl(n) => match n {
                Some(e) => s.push_str(&format!("negctrl({})@", expr_str(e, table))),
                None => s.push_str("negctrl@"),
            },
        }
    }
    s
}

/// Number of qubits a declared quantum symbol contributes.
fn qubit_width(ty: &Type) -> usize {
    match ty {
        Type::Qubit => 1,
        Type::QubitArray(dims) => dims.dims().iter().product(),
        _ => 0,
    }
}

fn bit_width(ty: &Type) -> usize {
    match ty {
        Type::Bit(_) => 1,
        Type::BitArray(dims, _) => dims.dims().iter().product(),
        _ => 0,
    }
}

pub fn extract(program: &asg::Program, table: &SymbolTable) -> Extracted {
    let mut ops = Vec::new();
    let mut summary = Summary::default();

    for stmt in program.stmts() {
        extract_stmt(stmt, table, &mut ops, &mut summary);
    }

    summary.total_ops = ops.len();
    summary.two_qubit_gates = ops
        .iter()
        .filter(|op| op.kind == "gate" && op.operands.split(',').count() >= 2)
        .count();
    Extracted { ops, summary }
}

fn extract_stmt(stmt: &asg::Stmt, table: &SymbolTable, ops: &mut Vec<Op>, summary: &mut Summary) {
    use asg::Stmt;
    match stmt {
        Stmt::GateCall(gc) => {
            let name = format!(
                "{}{}",
                modifier_prefix(gc.modifiers(), table),
                sym_name(gc.name(), table)
            );
            ops.push(Op {
                kind: "gate",
                name,
                operands: exprs_str(gc.qubits(), table),
                params: gc.params().map(|p| exprs_str(p, table)).unwrap_or_default(),
            });
        }
        Stmt::GPhaseCall(g) => ops.push(Op {
            kind: "gate",
            name: "gphase".into(),
            operands: String::new(),
            params: expr_str(g.arg(), table),
        }),
        Stmt::ModifiedGPhaseCall(g) => ops.push(Op {
            kind: "gate",
            name: format!("{}gphase", modifier_prefix(g.modifiers(), table)),
            operands: String::new(),
            params: expr_str(g.arg(), table),
        }),
        Stmt::Reset(r) => ops.push(Op {
            kind: "reset",
            name: "reset".into(),
            operands: expr_str(r.gate_operand(), table),
            params: String::new(),
        }),
        Stmt::Barrier(b) => ops.push(Op {
            kind: "barrier",
            name: "barrier".into(),
            operands: b
                .qubits()
                .map(|q| exprs_str(q, table))
                .unwrap_or_else(|| "(all)".into()),
            params: String::new(),
        }),
        Stmt::Assignment(a) => {
            // `c = measure q;` is the dominant executable assignment; render
            // it as a measure op. Everything else is an opaque classical
            // assignment whose value change shows as `~`.
            if let asg::Expr::MeasureExpression(me) = a.rvalue().expression() {
                ops.push(Op {
                    kind: "measure",
                    name: "measure".into(),
                    operands: format!(
                        "{} -> {}",
                        expr_str(me.operand(), table),
                        lvalue_str(a.lvalue(), table)
                    ),
                    params: String::new(),
                });
            } else {
                ops.push(Op {
                    kind: "assign",
                    name: lvalue_str(a.lvalue(), table),
                    operands: String::new(),
                    params: expr_str(a.rvalue(), table),
                });
            }
        }
        Stmt::ExprStmt(texpr) => {
            if let asg::Expr::MeasureExpression(me) = texpr.expression() {
                ops.push(Op {
                    kind: "measure",
                    name: "measure".into(),
                    operands: expr_str(me.operand(), table),
                    params: String::new(),
                });
            } else {
                ops.push(Op {
                    kind: "expr",
                    name: "expr".into(),
                    operands: String::new(),
                    params: expr_str(texpr, table),
                });
            }
        }
        // Control flow is opaque in v0: the condition (where cheap) is part
        // of the alignment key, the body participates only as a content
        // hash, so a body edit reports as one modified unit.
        Stmt::If(i) => ops.push(Op {
            kind: "ctrl-flow",
            name: "if".into(),
            operands: expr_str(i.condition(), table),
            params: hash_debug(&(i.then_branch(), i.else_branch())),
        }),
        Stmt::While(w) => ops.push(Op {
            kind: "ctrl-flow",
            name: "while".into(),
            operands: expr_str(w.condition(), table),
            params: hash_debug(&w.loop_body()),
        }),
        Stmt::ForStmt(f) => ops.push(Op {
            kind: "ctrl-flow",
            name: "for".into(),
            operands: sym_name(f.loop_var(), table),
            params: hash_debug(&(f.iterable(), f.loop_body())),
        }),
        Stmt::SwitchCaseStmt(s) => ops.push(Op {
            kind: "ctrl-flow",
            name: "switch".into(),
            operands: String::new(),
            params: hash_debug(&s),
        }),
        Stmt::Block(b) => ops.push(Op {
            kind: "ctrl-flow",
            name: "block".into(),
            operands: String::new(),
            params: hash_debug(&b),
        }),
        Stmt::GateDefinition(gd) => {
            summary.gate_defs += 1;
            ops.push(Op {
                kind: "gatedef",
                name: sym_name(gd.name(), table),
                operands: String::new(),
                params: hash_debug(&gd),
            });
        }
        Stmt::Delay(_) => ops.push(Op {
            kind: "delay",
            name: "delay".into(),
            operands: String::new(),
            params: hash_debug(&stmt),
        }),
        Stmt::DeclareQuantum(d) => {
            if let Ok(sid) = d.name() {
                summary.qubits += qubit_width(table[sid].symbol_type());
            }
        }
        Stmt::DeclareClassical(d) => {
            if let Ok(sid) = d.name() {
                summary.bits += bit_width(table[sid].symbol_type());
            }
        }
        Stmt::AnnotatedStmt(a) => extract_stmt(a.statement(), table, ops, summary),
        // Declarations, includes, pragmas, io declarations: summary-only or
        // ignored; they do not participate in the executable stream.
        _ => {}
    }
}
