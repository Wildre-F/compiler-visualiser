//! Recursive-descent parser: tokens → AST, every node spanned and id-tagged.
//!
//! Grammar (v1 - no functions yet):
//!
//! ```text
//! program    := stmt*
//! stmt       := "let" IDENT "=" expr ";"
//!             | IDENT "=" expr ";"
//!             | "if" "(" expr ")" block ("else" block)?
//!             | "while" "(" expr ")" block
//!             | "print" "(" expr ")" ";"
//! block      := "{" stmt* "}"
//! expr       := additive (cmp_op additive)*
//! additive   := multiplic (("+"|"-") multiplic)*
//! multiplic  := unary (("*"|"/"|"%") unary)*
//! unary      := "-" unary | primary
//! primary    := INT | IDENT | "(" expr ")"
//! ```

use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use crate::span::{CompileError, Span};

pub fn parse(tokens: &[Token]) -> Result<Vec<Stmt>, CompileError> {
    let mut p = Parser {
        tokens,
        pos: 0,
        next_id: 0,
    };
    let mut stmts = Vec::new();
    while !p.at_end() {
        stmts.push(p.stmt()?);
    }
    Ok(stmts)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    next_id: NodeId,
}

impl<'a> Parser<'a> {
    // ---- token helpers -------------------------------------------------

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    fn advance(&mut self) -> &'a Token {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t
    }

    fn check(&mut self, kind: TokenKind) -> bool {
        if self.peek_kind() == Some(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind, what: &str) -> Result<&'a Token, CompileError> {
        match self.peek() {
            Some(t) if t.kind == kind => Ok(self.advance()),
            Some(t) => Err(CompileError::new(
                format!("expected {what}, found '{}'", t.text),
                t.span,
            )),
            None => Err(CompileError::new(
                format!("expected {what}, found end of input"),
                self.eof_span(),
            )),
        }
    }

    fn eof_span(&self) -> Span {
        match self.tokens.last() {
            Some(t) => Span::new(t.span.end, t.span.end),
            None => Span::new(0, 0),
        }
    }

    fn id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ---- statements ----------------------------------------------------

    fn stmt(&mut self) -> Result<Stmt, CompileError> {
        match self.peek_kind() {
            Some(TokenKind::Let) => self.let_stmt(),
            Some(TokenKind::If) => self.if_stmt(),
            Some(TokenKind::While) => self.while_stmt(),
            Some(TokenKind::Print) => self.print_stmt(),
            Some(TokenKind::Ident) => self.assign_stmt(),
            Some(_) => {
                let t = self.peek().unwrap();
                Err(CompileError::new(
                    format!("expected a statement, found '{}'", t.text),
                    t.span,
                ))
            }
            None => Err(CompileError::new(
                "expected a statement, found end of input",
                self.eof_span(),
            )),
        }
    }

    fn let_stmt(&mut self) -> Result<Stmt, CompileError> {
        let kw = self.advance(); // `let`
        let name_tok = self.expect(TokenKind::Ident, "a variable name")?;
        self.expect(TokenKind::Assign, "'='")?;
        let init = self.expr()?;
        let semi = self.expect(TokenKind::Semi, "';'")?;
        Ok(Stmt {
            id: self.id(),
            span: kw.span.merge(semi.span),
            kind: StmtKind::Let {
                name: name_tok.text.clone(),
                init,
            },
        })
    }

    fn assign_stmt(&mut self) -> Result<Stmt, CompileError> {
        let name_tok = self.advance(); // IDENT
        self.expect(TokenKind::Assign, "'='")?;
        let value = self.expr()?;
        let semi = self.expect(TokenKind::Semi, "';'")?;
        Ok(Stmt {
            id: self.id(),
            span: name_tok.span.merge(semi.span),
            kind: StmtKind::Assign {
                name: name_tok.text.clone(),
                value,
            },
        })
    }

    fn if_stmt(&mut self) -> Result<Stmt, CompileError> {
        let kw = self.advance(); // `if`
        self.expect(TokenKind::LParen, "'('")?;
        let cond = self.expr()?;
        self.expect(TokenKind::RParen, "')'")?;
        let (then_body, mut end_span) = self.block()?;
        let else_body = if self.check(TokenKind::Else) {
            let (body, espan) = self.block()?;
            end_span = espan;
            Some(body)
        } else {
            None
        };
        Ok(Stmt {
            id: self.id(),
            span: kw.span.merge(end_span),
            kind: StmtKind::If {
                cond,
                then_body,
                else_body,
            },
        })
    }

    fn while_stmt(&mut self) -> Result<Stmt, CompileError> {
        let kw = self.advance(); // `while`
        self.expect(TokenKind::LParen, "'('")?;
        let cond = self.expr()?;
        self.expect(TokenKind::RParen, "')'")?;
        let (body, end_span) = self.block()?;
        Ok(Stmt {
            id: self.id(),
            span: kw.span.merge(end_span),
            kind: StmtKind::While { cond, body },
        })
    }

    fn print_stmt(&mut self) -> Result<Stmt, CompileError> {
        let kw = self.advance(); // `print`
        self.expect(TokenKind::LParen, "'('")?;
        let value = self.expr()?;
        self.expect(TokenKind::RParen, "')'")?;
        let semi = self.expect(TokenKind::Semi, "';'")?;
        Ok(Stmt {
            id: self.id(),
            span: kw.span.merge(semi.span),
            kind: StmtKind::Print { value },
        })
    }

    /// Returns the statements plus the span of the closing brace.
    fn block(&mut self) -> Result<(Vec<Stmt>, Span), CompileError> {
        self.expect(TokenKind::LBrace, "'{'")?;
        let mut stmts = Vec::new();
        loop {
            if self.peek_kind() == Some(TokenKind::RBrace) {
                let close = self.advance();
                return Ok((stmts, close.span));
            }
            if self.at_end() {
                return Err(CompileError::new("unclosed block: expected '}'", self.eof_span()));
            }
            stmts.push(self.stmt()?);
        }
    }

    // ---- expressions (precedence climbing) ------------------------------

    fn expr(&mut self) -> Result<Expr, CompileError> {
        let mut lhs = self.additive()?;
        while let Some(op) = self.cmp_op() {
            self.pos += 1;
            let rhs = self.additive()?;
            lhs = self.binary(op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn cmp_op(&self) -> Option<BinOp> {
        match self.peek_kind()? {
            TokenKind::EqEq => Some(BinOp::Eq),
            TokenKind::BangEq => Some(BinOp::Ne),
            TokenKind::Lt => Some(BinOp::Lt),
            TokenKind::LtEq => Some(BinOp::Le),
            TokenKind::Gt => Some(BinOp::Gt),
            TokenKind::GtEq => Some(BinOp::Ge),
            _ => None,
        }
    }

    fn additive(&mut self) -> Result<Expr, CompileError> {
        let mut lhs = self.multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => BinOp::Add,
                Some(TokenKind::Minus) => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.multiplicative()?;
            lhs = self.binary(op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn multiplicative(&mut self) -> Result<Expr, CompileError> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Star) => BinOp::Mul,
                Some(TokenKind::Slash) => BinOp::Div,
                Some(TokenKind::Percent) => BinOp::Rem,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.unary()?;
            lhs = self.binary(op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, CompileError> {
        if self.peek_kind() == Some(TokenKind::Minus) {
            let minus = self.advance();
            let operand = self.unary()?;
            return Ok(Expr {
                id: self.id(),
                span: minus.span.merge(operand.span),
                kind: ExprKind::Unary {
                    op: UnOp::Neg,
                    operand: Box::new(operand),
                },
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, CompileError> {
        match self.peek_kind() {
            Some(TokenKind::Int) => {
                let t = self.advance();
                let value: i32 = t.text.parse().map_err(|_| {
                    CompileError::new("integer literal does not fit in 32 bits", t.span)
                })?;
                Ok(Expr {
                    id: self.id(),
                    span: t.span,
                    kind: ExprKind::Int { value },
                })
            }
            Some(TokenKind::Ident) => {
                let t = self.advance();
                Ok(Expr {
                    id: self.id(),
                    span: t.span,
                    kind: ExprKind::Var {
                        name: t.text.clone(),
                    },
                })
            }
            Some(TokenKind::LParen) => {
                self.pos += 1;
                let inner = self.expr()?;
                self.expect(TokenKind::RParen, "')'")?;
                Ok(inner)
            }
            Some(_) => {
                let t = self.peek().unwrap();
                Err(CompileError::new(
                    format!("expected an expression, found '{}'", t.text),
                    t.span,
                ))
            }
            None => Err(CompileError::new(
                "expected an expression, found end of input",
                self.eof_span(),
            )),
        }
    }

    fn binary(&mut self, op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
        Expr {
            id: self.id(),
            span: lhs.span.merge(rhs.span),
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_src(src: &str) -> Vec<Stmt> {
        parse(&lex(src).unwrap()).unwrap()
    }

    #[test]
    fn parses_let_with_precedence() {
        let stmts = parse_src("let x = 1 + 2 * 3;");
        assert_eq!(stmts.len(), 1);
        let StmtKind::Let { name, init } = &stmts[0].kind else {
            panic!("expected let");
        };
        assert_eq!(name, "x");
        // `+` at the top, `*` underneath
        let ExprKind::Binary { op: BinOp::Add, rhs, .. } = &init.kind else {
            panic!("expected add at top: {:?}", init.kind);
        };
        assert!(matches!(rhs.kind, ExprKind::Binary { op: BinOp::Mul, .. }));
    }

    #[test]
    fn parses_while_with_block() {
        let stmts = parse_src("let i = 0; while (i < 5) { i = i + 1; }");
        assert_eq!(stmts.len(), 2);
        let StmtKind::While { cond, body } = &stmts[1].kind else {
            panic!("expected while");
        };
        assert!(matches!(cond.kind, ExprKind::Binary { op: BinOp::Lt, .. }));
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn parses_if_else() {
        let stmts = parse_src("if (1 == 1) { print(1); } else { print(2); }");
        let StmtKind::If { else_body, .. } = &stmts[0].kind else {
            panic!("expected if");
        };
        assert!(else_body.is_some());
    }

    #[test]
    fn spans_cover_statements() {
        let src = "let x = 42;";
        let stmts = parse_src(src);
        let s = stmts[0].span;
        assert_eq!(&src[s.start as usize..s.end as usize], "let x = 42;");
    }

    #[test]
    fn node_ids_are_unique() {
        let stmts = parse_src("let a = 1 + 2; print(a);");
        let mut ids = Vec::new();
        fn walk_expr(e: &Expr, ids: &mut Vec<u32>) {
            ids.push(e.id);
            match &e.kind {
                ExprKind::Binary { lhs, rhs, .. } => {
                    walk_expr(lhs, ids);
                    walk_expr(rhs, ids);
                }
                ExprKind::Unary { operand, .. } => walk_expr(operand, ids),
                _ => {}
            }
        }
        for s in &stmts {
            ids.push(s.id);
            match &s.kind {
                StmtKind::Let { init, .. } => walk_expr(init, &mut ids),
                StmtKind::Print { value } => walk_expr(value, &mut ids),
                _ => {}
            }
        }
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "duplicate node ids");
    }

    #[test]
    fn reports_missing_semi() {
        let err = parse(&lex("let x = 1").unwrap()).unwrap_err();
        assert!(err.message.contains("';'"), "{}", err.message);
    }
}
