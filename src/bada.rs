use std::fs;
use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;

#[macro_use]
mod diag;
mod compiler;
mod lex;
mod parser;

use crate::parser::Module;

fn main() -> ExitCode {
    let mut args = env::args();
    let program = args.next().expect("program");

    let input_path: Rc<Path> = match args.next() {
        Some(input_path) => Rc::from(Path::new(input_path.as_str())),
        None => {
            eprintln!("Usage: {program} <bada.boom>");
            eprintln!("ERROR: no input is provided");
            return ExitCode::FAILURE;
        }
    };

    let output_path = input_path.with_extension("beam");

    let content: Vec<_> = match fs::read_to_string(input_path.as_ref()) {
        Ok(content) => content.chars().collect(),
        Err(err) => {
            eprintln!("ERROR: could not load file {0}: {err}", input_path.display());
            return ExitCode::FAILURE;
        }
    };

    let mut lexer = lex::Lexer::new(content, input_path);
    let Some(module) = Module::parse(&mut lexer) else {
        return ExitCode::FAILURE;
    };

    let beam = compiler::compile_beam_module(&module);
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend(b"FOR1");
    bytes.extend((beam.len() as u32).to_be_bytes());
    bytes.extend(beam);


    match fs::write(&output_path, &bytes) {
        Ok(()) => {
            println!("INFO: Generated {0}", output_path.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("ERROR: Could not write file {0}: {err}", output_path.display());
            ExitCode::FAILURE
        }
    }
}

// TODO: implement BEAM disassembler as part of bada compiler
