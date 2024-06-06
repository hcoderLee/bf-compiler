use std::process::ExitCode;

use crate::program::{Program, UnbalancedBrackets};

mod program;

fn main() -> ExitCode {
    if std::env::args().len() != 2 {
        println!("Usage: {} <file_name>", std::env::args().nth(0).unwrap());
        return ExitCode::from(1);
    }

    let file_name = std::env::args().nth(1).unwrap();
    let source_code = match std::fs::read(file_name) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::from(2);
        }
    };
    let mut program = match Program::new(&source_code) {
        Ok(p) => p,
        Err(UnbalancedBrackets(c, pos)) => {
            eprintln!("Unbalanced brackets: {} at instruction position {}", c, pos);
            return ExitCode::from(3);
        }
    };

    if let Err(e) = program.run() {
        eprintln!("Error running program: {}", e);
    }

    ExitCode::from(0)
}
