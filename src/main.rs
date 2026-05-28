mod ast;
mod lexer;
mod parser;
mod interpreter;
mod value;

use std::io::{self, Read};
use lexer::Lexer;
use parser::Parser;
use interpreter::Interpreter;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // usage: awk-rs 'program' [file...]
    //        awk-rs -f script.awk [file...]
    if args.len() < 2 {
        eprintln!("Usage: awk-rs 'program' [file...]");
        eprintln!("       awk-rs -f script.awk [file...]");
        std::process::exit(1);
    }

    let (src, files) = if args[1] == "-f" {
        if args.len() < 3 {
            eprintln!("awk-rs: -f requires a filename");
            std::process::exit(1);
        }
        let src = std::fs::read_to_string(&args[2])
            .unwrap_or_else(|e| { eprintln!("awk-rs: {}", e); std::process::exit(1); });
        (src, &args[3..])
    } else {
        (args[1].clone(), &args[2..])
    };

    // parse once
    let mut lex = Lexer::new(&src);
    let tokens = lex.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap_or_else(|e| {
        eprintln!("awk-rs: parse error: {}", e);
        std::process::exit(1);
    });

    let mut interp = Interpreter::new();

    if files.is_empty() {
        // read from stdin
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)
            .unwrap_or_else(|e| { eprintln!("awk-rs: {}", e); std::process::exit(1); });
        interp.run(&program, &input);
    } else {
        for path in files {
            let input = std::fs::read_to_string(path)
                .unwrap_or_else(|e| { eprintln!("awk-rs: {}: {}", path, e); std::process::exit(1); });
            interp.run(&program, &input);
        }
    }
}