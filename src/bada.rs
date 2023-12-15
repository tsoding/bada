use std::fs;
use std::env;
use std::path::Path;
use std::process::ExitCode;

mod compiler;
#[macro_use]
mod diag;
mod lex;
mod parser;

fn main() -> ExitCode {
    let mut args = env::args();
    let program = args.next().expect("program");

    let input_path = if let Some(input_path) = args.next() {
        input_path
    } else {
        eprintln!("Usage: {program} <bada.boom>");
        eprintln!("ERROR: no input is provided");
        return ExitCode::FAILURE;
    };

    let output_path = Path::new(&input_path).with_extension("beam");

    let content: Vec<_> = match fs::read_to_string(&input_path) {
        Ok(content) => content.chars().collect(),
        Err(err) => {
            eprintln!("ERROR: could not load file {input_path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let mut lexer = lex::Lexer::new(&content, input_path.clone());
    let module = if let Some(module) = parser::parse_module(&mut lexer) {
        module
    } else {
        return ExitCode::FAILURE;
    };

    let beam = compiler::compile_beam_module(&module);
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend("FOR1".as_bytes());
    bytes.extend((beam.len() as u32).to_be_bytes());
    bytes.extend(beam);

    if let Err(err) = fs::write(&output_path, &bytes) {
        eprintln!("ERROR: Could not write file {output_path}: {err}", output_path = output_path.display());
        return ExitCode::FAILURE;
    }
    println!("INFO: Generated {output_path}", output_path = output_path.display());
    ExitCode::SUCCESS
}

// TODO: implement BEAM disassembler as part of bada compiler
