mod ast;
mod lexer;
mod parser;
mod interpreter;
mod value;

use lexer::Lexer;
use parser::Parser;

fn main() {
    let src = r#"BEGIN { print "hello" } { print $1 } END { print "done" }"#;
    let mut lex = Lexer::new(src);
    let tokens = lex.tokenize();
    let mut parser = Parser::new(tokens);
    match parser.parse_program() {
        Ok(program) => println!("{:#?}", program),
        Err(e) => eprintln!("Parse error: {}", e),
    }
}