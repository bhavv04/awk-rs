use std::collections::HashMap;
use regex::Regex;
use crate::ast::*;
use crate::value::Value;

// Control flow signals
enum Signal {
    None,
    Next,
    Break,
    Continue,
    Return(Value),
}

pub struct Interpreter {
    vars: HashMap<String, Value>,
    fields: Vec<String>,  // $0, $1, $2, ...
}

impl Interpreter {
    pub fn new() -> Self {
        let mut vars = HashMap::new();
        // awk defaults
        vars.insert("FS".to_string(),  Value::String(" ".to_string()));
        vars.insert("OFS".to_string(), Value::String(" ".to_string()));
        vars.insert("RS".to_string(),  Value::String("\n".to_string()));
        vars.insert("ORS".to_string(), Value::String("\n".to_string()));
        vars.insert("NR".to_string(),  Value::Number(0.0));
        vars.insert("NF".to_string(),  Value::Number(0.0));
        Interpreter { vars, fields: Vec::new() }
    }

    // ── Record handling ───────────────────────────────────────────────────────

    fn set_record(&mut self, line: &str) {
        self.fields.clear();
        self.fields.push(line.to_string()); // $0

        let fs = self.vars.get("FS")
            .cloned()
            .unwrap_or(Value::String(" ".to_string()))
            .to_string_val();

        let parts: Vec<String> = if fs == " " {
            // default: split on any whitespace, trim leading/trailing
            line.split_whitespace().map(|s| s.to_string()).collect()
        } else if fs.len() == 1 {
            line.split(fs.chars().next().unwrap())
                .map(|s| s.to_string())
                .collect()
        } else {
            // regex FS
            let re = Regex::new(&fs).unwrap_or_else(|_| Regex::new(" ").unwrap());
            re.split(line).map(|s| s.to_string()).collect()
        };

        for p in parts {
            self.fields.push(p);
        }

        let nf = (self.fields.len() - 1) as f64;
        self.vars.insert("NF".to_string(), Value::Number(nf));
    }

    fn rebuild_record(&mut self) {
        let ofs = self.vars.get("OFS")
            .cloned()
            .unwrap_or(Value::String(" ".to_string()))
            .to_string_val();
        let record = self.fields[1..].join(&ofs);
        self.fields[0] = record;
    }

    // ── Program entry points ──────────────────────────────────────────────────

    pub fn run(&mut self, program: &Program, input: &str) {
        // BEGIN
        for rule in &program.rules {
            if matches!(rule.pattern, Pattern::Begin) {
                self.exec_block(&rule.action);
            }
        }

        // main rules
        let nr_start = self.vars.get("NR")
            .and_then(|v| if let Value::Number(n) = v { Some(*n) } else { None })
            .unwrap_or(0.0);
        let mut nr = nr_start as i64;

        for line in input.lines() {
            nr += 1;
            self.vars.insert("NR".to_string(), Value::Number(nr as f64));
            self.set_record(line);

            let mut do_next = false;
            for rule in &program.rules {
                if do_next { break; }
                match &rule.pattern {
                    Pattern::Begin | Pattern::End => continue,
                    Pattern::Always => {
                        if let Signal::Next = self.exec_block(&rule.action) {
                            do_next = true;
                        }
                    }
                    Pattern::Expr(expr) => {
                        let val = self.eval_expr(expr);
                        if val.to_bool() {
                            if let Signal::Next = self.exec_block(&rule.action) {
                                do_next = true;
                            }
                        }
                    }
                    Pattern::Range(_, _) => {
                        // simplified: treat as always for now
                        if let Signal::Next = self.exec_block(&rule.action) {
                            do_next = true;
                        }
                    }
                }
            }
        }

        // END
        for rule in &program.rules {
            if matches!(rule.pattern, Pattern::End) {
                self.exec_block(&rule.action);
            }
        }
    }

    // ── Statement execution ───────────────────────────────────────────────────

    fn exec_block(&mut self, stmts: &[Stmt]) -> Signal {
        for stmt in stmts {
            match self.exec_stmt(stmt) {
                Signal::None => {}
                sig => return sig,
            }
        }
        Signal::None
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Signal {
        match stmt {
            Stmt::Expr(expr) => {
                self.eval_expr(expr);
                Signal::None
            }

            Stmt::Print(args, redirect) => {
                let ofs = self.vars.get("OFS")
                    .cloned()
                    .unwrap_or(Value::String(" ".to_string()))
                    .to_string_val();
                let ors = self.vars.get("ORS")
                    .cloned()
                    .unwrap_or(Value::String("\n".to_string()))
                    .to_string_val();

                let output = if args.is_empty() {
                    // bare print → print $0
                    self.fields.first().cloned().unwrap_or_default()
                } else {
                    args.iter()
                        .map(|a| self.eval_expr(a).to_string_val())
                        .collect::<Vec<_>>()
                        .join(&ofs)
                };

                match redirect {
                    Some(path) => {
                        use std::io::Write;
                        let mut f = std::fs::OpenOptions::new()
                            .create(true).append(true).open(path)
                            .expect("could not open output file");
                        write!(f, "{}{}", output, ors).ok();
                    }
                    None => print!("{}{}", output, ors),
                }
                Signal::None
            }

            Stmt::Printf(args) => {
                if args.is_empty() { return Signal::None; }
                let fmt_str = self.eval_expr(&args[0]).to_string_val();
                let rest: Vec<Value> = args[1..].iter()
                    .map(|a| self.eval_expr(a))
                    .collect();
                let out = self.sprintf(&fmt_str, &rest);
                print!("{}", out);
                Signal::None
            }

            Stmt::If(cond, then_b, else_b) => {
                if self.eval_expr(cond).to_bool() {
                    self.exec_block(then_b)
                } else if let Some(eb) = else_b {
                    self.exec_block(eb)
                } else {
                    Signal::None
                }
            }

            Stmt::While(cond, body) => {
                loop {
                    if !self.eval_expr(cond).to_bool() { break; }
                    match self.exec_block(body) {
                        Signal::Break => break,
                        Signal::Next => return Signal::Next,
                        Signal::Return(v) => return Signal::Return(v),
                        _ => {}
                    }
                }
                Signal::None
            }

            Stmt::For(init, cond, update, body) => {
                if let Some(i) = init { self.exec_stmt(i); }
                loop {
                    if let Some(c) = cond {
                        if !self.eval_expr(c).to_bool() { break; }
                    }
                    match self.exec_block(body) {
                        Signal::Break => break,
                        Signal::Next => return Signal::Next,
                        Signal::Return(v) => return Signal::Return(v),
                        _ => {}
                    }
                    if let Some(u) = update { self.exec_stmt(u); }
                }
                Signal::None
            }

            Stmt::Next     => Signal::Next,
            Stmt::Break    => Signal::Break,
            Stmt::Continue => Signal::Continue,
            Stmt::Return(expr) => {
                let val = expr.as_ref()
                    .map(|e| self.eval_expr(e))
                    .unwrap_or(Value::Uninitialized);
                Signal::Return(val)
            }
        }
    }

    // ── Expression evaluation ─────────────────────────────────────────────────

    fn eval_expr(&mut self, expr: &Expr) -> Value {
        match expr {
            Expr::Number(n) => Value::Number(*n),
            Expr::String(s) => Value::String(s.clone()),
            Expr::Regex(r)  => {
                // bare regex in boolean context matches $0
                let s = self.fields.first().cloned().unwrap_or_default();
                let matched = Regex::new(r)
                    .map(|re| re.is_match(&s))
                    .unwrap_or(false);
                Value::Number(if matched { 1.0 } else { 0.0 })
            }

            Expr::Var(name) => {
                self.vars.get(name).cloned().unwrap_or(Value::Uninitialized)
            }

            Expr::Field(idx_expr) => {
                let idx = self.eval_expr(idx_expr).to_f64() as usize;
                self.fields.get(idx).cloned()
                    .map(Value::String)
                    .unwrap_or(Value::Uninitialized)
            }

            Expr::Assign(name, rhs) => {
                let val = self.eval_expr(rhs);
                self.vars.insert(name.clone(), val.clone());
                val
            }

            Expr::FieldAssign(idx_expr, rhs) => {
                let idx = self.eval_expr(idx_expr).to_f64() as usize;
                let val = self.eval_expr(rhs);
                while self.fields.len() <= idx {
                    self.fields.push(String::new());
                }
                self.fields[idx] = val.to_string_val();
                if idx > 0 { self.rebuild_record(); }
                val
            }

            Expr::BinOp(left, op, right) => {
                self.eval_binop(left, op, right)
            }

            Expr::UnaryOp(op, operand) => {
                let val = self.eval_expr(operand);
                match op {
                    UnaryOp::Neg => Value::Number(-val.to_f64()),
                    UnaryOp::Not => Value::Number(if val.to_bool() { 0.0 } else { 1.0 }),
                }
            }

            Expr::Match(expr, pat) => {
                let s = self.eval_expr(expr).to_string_val();
                let matched = Regex::new(pat)
                    .map(|re| re.is_match(&s))
                    .unwrap_or(false);
                Value::Number(if matched { 1.0 } else { 0.0 })
            }

            Expr::NotMatch(expr, pat) => {
                let s = self.eval_expr(expr).to_string_val();
                let matched = Regex::new(pat)
                    .map(|re| re.is_match(&s))
                    .unwrap_or(false);
                Value::Number(if matched { 0.0 } else { 1.0 })
            }

            Expr::Call(name, args) => {
                self.eval_call(name, args)
            }
        }
    }

    fn eval_binop(&mut self, left: &Expr, op: &BinOp, right: &Expr) -> Value {
        // short circuit
        if matches!(op, BinOp::And) {
            let l = self.eval_expr(left);
            if !l.to_bool() { return Value::Number(0.0); }
            return Value::Number(if self.eval_expr(right).to_bool() { 1.0 } else { 0.0 });
        }
        if matches!(op, BinOp::Or) {
            let l = self.eval_expr(left);
            if l.to_bool() { return Value::Number(1.0); }
            return Value::Number(if self.eval_expr(right).to_bool() { 1.0 } else { 0.0 });
        }

        let l = self.eval_expr(left);
        let r = self.eval_expr(right);

        match op {
            BinOp::Add => Value::Number(l.to_f64() + r.to_f64()),
            BinOp::Sub => Value::Number(l.to_f64() - r.to_f64()),
            BinOp::Mul => Value::Number(l.to_f64() * r.to_f64()),
            BinOp::Div => {
                let divisor = r.to_f64();
                if divisor == 0.0 {
                    eprintln!("awk-rs: division by zero");
                    Value::Number(0.0)
                } else {
                    Value::Number(l.to_f64() / divisor)
                }
            }
            BinOp::Mod => Value::Number(l.to_f64() % r.to_f64()),
            BinOp::Concat => {
                Value::String(format!("{}{}", l.to_string_val(), r.to_string_val()))
            }
            BinOp::Eq => Value::Number(if l == r { 1.0 } else { 0.0 }),
            BinOp::Ne => Value::Number(if l != r { 1.0 } else { 0.0 }),
            BinOp::Lt => Value::Number(if l <  r { 1.0 } else { 0.0 }),
            BinOp::Le => Value::Number(if l <= r { 1.0 } else { 0.0 }),
            BinOp::Gt => Value::Number(if l >  r { 1.0 } else { 0.0 }),
            BinOp::Ge => Value::Number(if l >= r { 1.0 } else { 0.0 }),
            BinOp::And | BinOp::Or => unreachable!(),
        }
    }

    // ── Built-in functions ────────────────────────────────────────────────────

    fn eval_call(&mut self, name: &str, args: &[Expr]) -> Value {
        match name {
            "length" => {
                let s = if args.is_empty() {
                    self.fields.first().cloned().unwrap_or_default()
                } else {
                    self.eval_expr(&args[0]).to_string_val()
                };
                Value::Number(s.len() as f64)
            }

            "substr" => {
                let s = self.eval_expr(&args[0]).to_string_val();
                let start = (self.eval_expr(&args[1]).to_f64() as usize).saturating_sub(1);
                let chars: Vec<char> = s.chars().collect();
                let slice = if args.len() >= 3 {
                    let len = self.eval_expr(&args[2]).to_f64() as usize;
                    &chars[start.min(chars.len())..(start + len).min(chars.len())]
                } else {
                    &chars[start.min(chars.len())..]
                };
                Value::String(slice.iter().collect())
            }

            "index" => {
                let s   = self.eval_expr(&args[0]).to_string_val();
                let pat = self.eval_expr(&args[1]).to_string_val();
                let pos = s.find(&pat).map(|i| i + 1).unwrap_or(0); // awk is 1-indexed
                Value::Number(pos as f64)
            }

            "split" => {
                let s  = self.eval_expr(&args[0]).to_string_val();
                let fs = if args.len() >= 3 {
                    self.eval_expr(&args[2]).to_string_val()
                } else {
                    self.vars.get("FS").cloned()
                        .unwrap_or(Value::String(" ".to_string()))
                        .to_string_val()
                };
                let parts: Vec<&str> = if fs == " " {
                    s.split_whitespace().collect()
                } else {
                    s.split(fs.as_str()).collect()
                };
                // store in array named by args[1] as var_1, var_2 etc.
                if let Expr::Var(arr) = &args[1] {
                    for (i, p) in parts.iter().enumerate() {
                        self.vars.insert(
                            format!("{}_{}", arr, i + 1),
                            Value::String(p.to_string()),
                        );
                    }
                }
                Value::Number(parts.len() as f64)
            }

            "sub" => {
                let pat  = self.eval_expr(&args[0]).to_string_val();
                let repl = self.eval_expr(&args[1]).to_string_val();
                let target = if args.len() >= 3 {
                    self.eval_expr(&args[2]).to_string_val()
                } else {
                    self.fields.first().cloned().unwrap_or_default()
                };
                let re = Regex::new(&pat).unwrap_or_else(|_| Regex::new("").unwrap());
                let result = re.replacen(&target, 1, repl.as_str()).to_string();
                if args.len() >= 3 {
                    if let Expr::Var(name) = &args[2] {
                        self.vars.insert(name.clone(), Value::String(result));
                    }
                } else if !self.fields.is_empty() {
                    self.fields[0] = result;
                }
                Value::Number(1.0)
            }

            "gsub" => {
                let pat  = self.eval_expr(&args[0]).to_string_val();
                let repl = self.eval_expr(&args[1]).to_string_val();
                let target = if args.len() >= 3 {
                    self.eval_expr(&args[2]).to_string_val()
                } else {
                    self.fields.first().cloned().unwrap_or_default()
                };
                let re = Regex::new(&pat).unwrap_or_else(|_| Regex::new("").unwrap());
                let count = re.find_iter(&target).count();
                let result = re.replace_all(&target, repl.as_str()).to_string();
                if args.len() >= 3 {
                    if let Expr::Var(name) = &args[2] {
                        self.vars.insert(name.clone(), Value::String(result));
                    }
                } else if !self.fields.is_empty() {
                    self.fields[0] = result;
                }
                Value::Number(count as f64)
            }

            "tolower" => {
                let s = self.eval_expr(&args[0]).to_string_val();
                Value::String(s.to_lowercase())
            }

            "toupper" => {
                let s = self.eval_expr(&args[0]).to_string_val();
                Value::String(s.to_uppercase())
            }

            "int" => {
                let n = self.eval_expr(&args[0]).to_f64();
                Value::Number(n.trunc())
            }

            "sqrt" => Value::Number(self.eval_expr(&args[0]).to_f64().sqrt()),
            "log"  => Value::Number(self.eval_expr(&args[0]).to_f64().ln()),
            "exp"  => Value::Number(self.eval_expr(&args[0]).to_f64().exp()),

            _ => {
                eprintln!("awk-rs: unknown function: {}", name);
                Value::Uninitialized
            }
        }
    }

    // ── printf formatting ─────────────────────────────────────────────────────

    fn sprintf(&self, fmt: &str, args: &[Value]) -> String {
        let mut out = String::new();
        let mut chars = fmt.chars().peekable();
        let mut arg_idx = 0;

        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('%') => out.push('%'),
                Some('s') => {
                    let s = args.get(arg_idx)
                        .map(|v| v.to_string_val())
                        .unwrap_or_default();
                    out.push_str(&s);
                    arg_idx += 1;
                }
                Some('d') | Some('i') => {
                    let n = args.get(arg_idx)
                        .map(|v| v.to_f64() as i64)
                        .unwrap_or(0);
                    out.push_str(&n.to_string());
                    arg_idx += 1;
                }
                Some('f') => {
                    let n = args.get(arg_idx)
                        .map(|v| v.to_f64())
                        .unwrap_or(0.0);
                    out.push_str(&format!("{:.6}", n));
                    arg_idx += 1;
                }
                Some('g') => {
                    let n = args.get(arg_idx)
                        .map(|v| v.to_f64())
                        .unwrap_or(0.0);
                    out.push_str(&format!("{}", n));
                    arg_idx += 1;
                }
                Some('n') => out.push('\n'),
                Some(other) => { out.push('%'); out.push(other); }
                None => out.push('%'),
            }
        }
        out
    }
}