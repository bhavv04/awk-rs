#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Number(f64),
    String(String),
    Regex(String),

    // Variables
    Var(String),        // regular variable: x
    Field(Box<Expr>),   // field access: $1, $NF

    // Operations
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    Assign(String, Box<Expr>),          // x = expr
    FieldAssign(Box<Expr>, Box<Expr>),  // $1 = expr

    // Functions
    Call(String, Vec<Expr>),

    // Regex match
    Match(Box<Expr>, String),       // expr ~ /pat/
    NotMatch(Box<Expr>, String),    // expr !~ /pat/
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    Concat,  // awk string concat is just adjacency: "a" "b"
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,    // -x
    Not,    // !x
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Print(Vec<Expr>, Option<String>),       // print expr, expr > "file"
    Printf(Vec<Expr>),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>), // if (cond) { } else { }
    While(Expr, Vec<Stmt>),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Box<Stmt>>, Vec<Stmt>),
    Return(Option<Expr>),
    Next,                                   // skip to next record
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Begin,
    End,
    Expr(Expr),         // pattern { action }
    Range(Expr, Expr),  // pat1, pat2 { action }
    Always,             // no pattern, just { action }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub pattern: Pattern,
    pub action: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub rules: Vec<Rule>,
}