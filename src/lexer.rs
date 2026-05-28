#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Number(f64),
    StringLit(String),
    Regex(String),      // /pattern/

    // Identifiers & keywords
    Ident(String),
    Begin,
    End,
    Print,
    Printf,
    If,
    Else,
    While,
    For,
    Return,
    Next,
    Break,
    Continue,
    In,

    // Operators
    Plus, Minus, Star, Slash, Percent,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or, Not,
    Assign,             // =
    PlusAssign,         // +=
    MinusAssign,        // -=
    MulAssign,          // *=
    DivAssign,          // /=
    Match,              // ~
    NotMatch,           // !~
    Dollar,             // $
    Concat,             // handled implicitly but useful as separator

    // Delimiters
    LParen, RParen,
    LBrace, RBrace,
    LBracket, RBracket,
    Semicolon,
    Comma,
    Newline,
    Pipe,
    Append,             // >>

    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Token {
        self.advance(); // consume opening "
        let mut s = String::new();
        while let Some(c) = self.advance() {
            match c {
                '"' => break,
                '\\' => match self.advance() {
                    Some('n')  => s.push('\n'),
                    Some('t')  => s.push('\t'),
                    Some('\\') => s.push('\\'),
                    Some('"')  => s.push('"'),
                    Some(c)    => { s.push('\\'); s.push(c); }
                    None       => break,
                },
                c => s.push(c),
            }
        }
        Token::StringLit(s)
    }

    fn read_regex(&mut self) -> Token {
        self.advance(); // consume opening /
        let mut s = String::new();
        while let Some(c) = self.advance() {
            match c {
                '/' => break,
                '\\' => {
                    s.push('\\');
                    if let Some(nc) = self.advance() { s.push(nc); }
                }
                c => s.push(c),
            }
        }
        Token::Regex(s)
    }

    fn read_number(&mut self) -> Token {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        Token::Number(s.parse().unwrap_or(0.0))
    }

    fn read_ident(&mut self) -> Token {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match s.as_str() {
            "BEGIN"    => Token::Begin,
            "END"      => Token::End,
            "print"    => Token::Print,
            "printf"   => Token::Printf,
            "if"       => Token::If,
            "else"     => Token::Else,
            "while"    => Token::While,
            "for"      => Token::For,
            "return"   => Token::Return,
            "next"     => Token::Next,
            "break"    => Token::Break,
            "continue" => Token::Continue,
            "in"       => Token::In,
            _          => Token::Ident(s),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => { tokens.push(Token::Eof); break; }
                Some(c) => {
                    let tok = match c {
                        '\n' => { self.advance(); Token::Newline }
                        '#'  => {
                            // comment — skip to end of line
                            while let Some(c) = self.peek() {
                                if c == '\n' { break; }
                                self.advance();
                            }
                            continue;
                        }
                        '"'  => self.read_string(),
                        '0'..='9' => self.read_number(),
                        'a'..='z' | 'A'..='Z' | '_' => self.read_ident(),
                        '/' => {
                            // ambiguous: division or regex
                            // heuristic: if last token was a number, ident, or )
                            // it's division — otherwise regex
                            let last = tokens.last();
                            let is_div = matches!(last,
                                Some(Token::Number(_)) |
                                Some(Token::Ident(_))  |
                                Some(Token::RParen)
                            );
                            if is_div {
                                self.advance();
                                Token::Slash
                            } else {
                                self.read_regex()
                            }
                        }
                        '+' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::PlusAssign }
                            else { Token::Plus }
                        }
                        '-' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::MinusAssign }
                            else { Token::Minus }
                        }
                        '*' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::MulAssign }
                            else { Token::Star }
                        }
                        '>' => {
                            self.advance();
                            if self.peek() == Some('>') { self.advance(); Token::Append }
                            else if self.peek() == Some('=') { self.advance(); Token::Ge }
                            else { Token::Gt }
                        }
                        '<' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Le }
                            else { Token::Lt }
                        }
                        '=' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Eq }
                            else { Token::Assign }
                        }
                        '!' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Ne }
                            else if self.peek() == Some('~') { self.advance(); Token::NotMatch }
                            else { Token::Not }
                        }
                        '~' => { self.advance(); Token::Match }
                        '&' => {
                            self.advance();
                            if self.peek() == Some('&') { self.advance(); Token::And }
                            else { Token::Ident("&".to_string()) } // rare, just recover
                        }
                        '|' => {
                            self.advance();
                            if self.peek() == Some('|') { self.advance(); Token::Or }
                            else { Token::Pipe }
                        }
                        '$' => { self.advance(); Token::Dollar }
                        '(' => { self.advance(); Token::LParen }
                        ')' => { self.advance(); Token::RParen }
                        '{' => { self.advance(); Token::LBrace }
                        '}' => { self.advance(); Token::RBrace }
                        '[' => { self.advance(); Token::LBracket }
                        ']' => { self.advance(); Token::RBracket }
                        ';' => { self.advance(); Token::Semicolon }
                        ',' => { self.advance(); Token::Comma }
                        '%' => { self.advance(); Token::Percent }
                        _   => { self.advance(); continue; } // skip unknown chars
                    };
                    tokens.push(tok);
                }
            }
        }
        tokens
    }
}