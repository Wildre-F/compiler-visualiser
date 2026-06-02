//! AST node definitions.
//!
//! Every node carries a unique `id` (used by provenance links: instruction →
//! AST node) and a `span` (AST node → source characters).

use crate::span::Span;
use serde::Serialize;

pub type NodeId = u32;

#[derive(Debug, Clone, Serialize)]
pub struct Stmt {
    pub id: NodeId,
    pub span: Span,
    #[serde(flatten)]
    pub kind: StmtKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StmtKind {
    Let {
        name: String,
        init: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Print {
        value: Expr,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    pub id: NodeId,
    pub span: Span,
    #[serde(flatten)]
    pub kind: ExprKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExprKind {
    Int {
        value: i32,
    },
    Var {
        name: String,
    },
    Unary {
        op: UnOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnOp {
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
