use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let tok = self.tokens.get(self.pos).unwrap_or(&Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let tok = self.peek().clone();
        if std::mem::discriminant(&tok) == std::mem::discriminant(expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, tok))
        }
    }

    fn skip_newlines(&mut self) {
        while self.peek() == &Token::Newline || self.peek() == &Token::Semicolon {
            self.advance();
        }
    }

    // ── Program ──────────────────────────────────────────────────────────────

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut rules = Vec::new();
        self.skip_newlines();
        while self.peek() != &Token::Eof {
            rules.push(self.parse_rule()?);
            self.skip_newlines();
        }
        Ok(Program { rules })
    }

    fn parse_rule(&mut self) -> Result<Rule, String> {
        let pattern = self.parse_pattern()?;
        self.skip_newlines();
        let action = if self.peek() == &Token::LBrace {
            self.parse_block()?
        } else {
            // pattern with no action → default print
            vec![Stmt::Print(vec![], None)]
        };
        Ok(Rule { pattern, action })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        match self.peek().clone() {
            Token::Begin => { self.advance(); Ok(Pattern::Begin) }
            Token::End   => { self.advance(); Ok(Pattern::End) }
            Token::LBrace => Ok(Pattern::Always),
            _ => {
                let expr = self.parse_expr()?;
                // check for range pattern: pat1, pat2
                if self.peek() == &Token::Comma {
                    self.advance();
                    let expr2 = self.parse_expr()?;
                    Ok(Pattern::Range(expr, expr2))
                } else {
                    Ok(Pattern::Expr(expr))
                }
            }
        }
    }

    // ── Block & Statements ───────────────────────────────────────────────────

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let stmt = match self.peek().clone() {
            Token::Print  => self.parse_print()?,
            Token::Printf => self.parse_printf()?,
            Token::If     => self.parse_if()?,
            Token::While  => self.parse_while()?,
            Token::For    => self.parse_for()?,
            Token::Return => {
                self.advance();
                let expr = if self.peek() != &Token::Newline
                    && self.peek() != &Token::Semicolon
                    && self.peek() != &Token::RBrace
                {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                Stmt::Return(expr)
            }
            Token::Next     => { self.advance(); Stmt::Next }
            Token::Break    => { self.advance(); Stmt::Break }
            Token::Continue => { self.advance(); Stmt::Continue }
            _ => Stmt::Expr(self.parse_expr()?),
        };
        // consume trailing separator
        if self.peek() == &Token::Semicolon || self.peek() == &Token::Newline {
            self.advance();
        }
        Ok(stmt)
    }

    fn parse_print(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'print'
        let mut args = Vec::new();
        let mut redirect = None;

        if self.peek() != &Token::Newline
            && self.peek() != &Token::Semicolon
            && self.peek() != &Token::RBrace
        {
            args.push(self.parse_expr()?);
            while self.peek() == &Token::Comma {
                self.advance();
                args.push(self.parse_expr()?);
            }
            // output redirect: > "file"
            if self.peek() == &Token::Gt {
                self.advance();
                if let Token::StringLit(s) = self.peek().clone() {
                    self.advance();
                    redirect = Some(s);
                }
            }
        }
        Ok(Stmt::Print(args, redirect))
    }

    fn parse_printf(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'printf'
        let mut args = Vec::new();
        args.push(self.parse_expr()?);
        while self.peek() == &Token::Comma {
            self.advance();
            args.push(self.parse_expr()?);
        }
        Ok(Stmt::Printf(args))
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'if'
        self.expect(&Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.skip_newlines();
        let then_branch = self.parse_block()?;
        self.skip_newlines();
        let else_branch = if self.peek() == &Token::Else {
            self.advance();
            self.skip_newlines();
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::If(cond, then_branch, else_branch))
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'while'
        self.expect(&Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::While(cond, body))
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'for'
        self.expect(&Token::LParen)?;

        let init = if self.peek() != &Token::Semicolon {
            Some(Box::new(Stmt::Expr(self.parse_expr()?)))
        } else {
            None
        };
        self.expect(&Token::Semicolon)?;

        let cond = if self.peek() != &Token::Semicolon {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(&Token::Semicolon)?;

        let update = if self.peek() != &Token::RParen {
            Some(Box::new(Stmt::Expr(self.parse_expr()?)))
        } else {
            None
        };
        self.expect(&Token::RParen)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::For(init, cond, update, body))
    }

    // ── Expressions (Pratt parser) ────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr, String> {
        let expr = self.parse_or()?;
        match self.peek().clone() {
            Token::Assign => {
                self.advance();
                let rhs = self.parse_assign()?;
                match expr {
                    Expr::Var(name) => Ok(Expr::Assign(name, Box::new(rhs))),
                    Expr::Field(f)  => Ok(Expr::FieldAssign(f, Box::new(rhs))),
                    _ => Err("Invalid assignment target".to_string()),
                }
            }
            Token::PlusAssign => {
                self.advance();
                let rhs = self.parse_assign()?;
                if let Expr::Var(name) = expr {
                    Ok(Expr::Assign(
                        name.clone(),
                        Box::new(Expr::BinOp(
                            Box::new(Expr::Var(name)),
                            BinOp::Add,
                            Box::new(rhs),
                        )),
                    ))
                } else {
                    Err("Invalid += target".to_string())
                }
            }
            Token::MinusAssign => {
                self.advance();
                let rhs = self.parse_assign()?;
                if let Expr::Var(name) = expr {
                    Ok(Expr::Assign(
                        name.clone(),
                        Box::new(Expr::BinOp(
                            Box::new(Expr::Var(name)),
                            BinOp::Sub,
                            Box::new(rhs),
                        )),
                    ))
                } else {
                    Err("Invalid -= target".to_string())
                }
            }
            _ => Ok(expr),
        }
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.peek() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp(Box::new(left), BinOp::Or, Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_match()?;
        while self.peek() == &Token::And {
            self.advance();
            let right = self.parse_match()?;
            left = Expr::BinOp(Box::new(left), BinOp::And, Box::new(right));
        }
        Ok(left)
    }

    fn parse_match(&mut self) -> Result<Expr, String> {
        let left = self.parse_cmp()?;
        match self.peek().clone() {
            Token::Match => {
                self.advance();
                if let Token::Regex(pat) = self.peek().clone() {
                    self.advance();
                    Ok(Expr::Match(Box::new(left), pat))
                } else {
                    Err("Expected regex after ~".to_string())
                }
            }
            Token::NotMatch => {
                self.advance();
                if let Token::Regex(pat) = self.peek().clone() {
                    self.advance();
                    Ok(Expr::NotMatch(Box::new(left), pat))
                } else {
                    Err("Expected regex after !~".to_string())
                }
            }
            _ => Ok(left),
        }
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_concat()?;
        loop {
            let op = match self.peek().clone() {
                Token::Eq => BinOp::Eq,
                Token::Ne => BinOp::Ne,
                Token::Lt => BinOp::Lt,
                Token::Le => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_concat()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_concat(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_add()?;
        // implicit concat: two adjacent exprs not separated by operator
        loop {
            match self.peek() {
                Token::Number(_) | Token::StringLit(_) | Token::Ident(_)
                | Token::Dollar | Token::LParen => {
                    let right = self.parse_add()?;
                    left = Expr::BinOp(Box::new(left), BinOp::Concat, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Token::Plus  => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star    => BinOp::Mul,
                Token::Slash   => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                Ok(Expr::UnaryOp(UnaryOp::Neg, Box::new(self.parse_unary()?)))
            }
            Token::Not => {
                self.advance();
                Ok(Expr::UnaryOp(UnaryOp::Not, Box::new(self.parse_unary()?)))
            }
            Token::Dollar => {
                self.advance();
                Ok(Expr::Field(Box::new(self.parse_primary()?)))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Number(n) => { self.advance(); Ok(Expr::Number(n)) }
            Token::StringLit(s) => { self.advance(); Ok(Expr::String(s)) }
            Token::Regex(r) => { self.advance(); Ok(Expr::Regex(r)) }
            Token::Ident(name) => {
                self.advance();
                // function call?
                if self.peek() == &Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != &Token::RParen {
                        args.push(self.parse_expr()?);
                        while self.peek() == &Token::Comma {
                            self.advance();
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            tok => Err(format!("Unexpected token in expression: {:?}", tok)),
        }
    }
}