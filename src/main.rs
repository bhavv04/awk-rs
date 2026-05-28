mod ast;
mod lexer;
mod parser;
mod interpreter;
mod value;

use lexer::Lexer;
use parser::Parser;
use interpreter::Interpreter;

fn main() {
    let src = r#"BEGIN { print "hello" } { print $1 } END { print "done" }"#;
    let input = "foo bar\nbaz qux\n";

    let mut lex = Lexer::new(src);
    let tokens = lex.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse error");

    let mut interp = Interpreter::new();
    interp.run(&program, input);
}