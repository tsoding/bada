use std::fs;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::ExitCode;

mod asm {
    use std::collections::HashMap;
    use parser::Expr;

    #[repr(u8)]
    enum Tag {
        U = 0,
        I = 1,
        A = 2,
        X = 3,
        // Y = 4,
        F = 5,
        // H = 6,
        // Z = 7,
    }

    fn encode_arg(tag: Tag, n: i32) -> Vec<u8> {
        if n < 0 {
            todo!("negative");
        } else if n < 16 {
            // (N bsl 4) bor Tag;
            let tag = tag as u8;
            let n = n as u8;
            vec![(n<<4)|tag]
        } else if n < 0x800 {
            // [((N bsr 3) band 2#11100000) bor Tag bor 2#00001000, N band 16#ff];
            let tag = tag as u32;
            let n = n as u32;
            let a = (((n>>3)&0b11100000u32)|tag|0b00001000u32) as u8;
            let b = (n&0xFF) as u8;
            vec![a, b]
        } else {
            todo!("large numbers");
        }
    }

    #[repr(u8)]
    enum OpCode {
        Label = 1,
        FuncInfo = 2,
        IntCodeEnd = 3,
        Return = 19,
        Move = 64,
        GcBif2 = 125,
    }

    // aaaa|aaaa|a000|
    fn pad_chunk(word_size: usize, chunk: &mut Vec<u8>) {
        let len = chunk.len();
        let new_len = (len + word_size - 1)/word_size*word_size;
        chunk.resize(new_len, 0)
    }

    fn encode_chunk(tag: [u8; 4], chunk: Vec<u8>) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend(tag);
        result.extend((chunk.len() as u32).to_be_bytes());
        result.extend(chunk);
        pad_chunk(4, &mut result);
        result
    }

    // X0, X1, X2, X3, ..., X1000
    //          ^

    fn compile_expr(expr: &Expr, atoms: &mut Atoms, imports: &HashMap<(u32, u32, u32), u32>, code: &mut Vec<u8>, stack_size: &mut usize) {
        match expr {
            Expr::Number(x) => {
                code.push(OpCode::Move as u8);
                code.extend(encode_arg(Tag::I, (*x) as i32));
                code.extend(encode_arg(Tag::X, (*stack_size) as i32));
                *stack_size += 1;
            },
            Expr::Sum{lhs, rhs} => {
                compile_expr(lhs, atoms, imports, code, stack_size);
                compile_expr(rhs, atoms, imports, code, stack_size);

                assert!(*stack_size >= 2);

                let bif2_plus = imports
                    .get(&resolve_function_signature(atoms, "erlang", "+", 2))
                    .cloned()
                    .expect("erlang:'+' should be always present");

                code.push(OpCode::GcBif2 as u8);
                code.extend(encode_arg(Tag::F, 0)); // Lbl
                code.extend(encode_arg(Tag::U, 2)); // Live
                code.extend(encode_arg(Tag::U, bif2_plus as i32)); // Bif
                code.extend(encode_arg(Tag::X, (*stack_size - 2) as i32)); // Arg1
                code.extend(encode_arg(Tag::X, (*stack_size - 1) as i32)); // Arg2
                code.extend(encode_arg(Tag::X, (*stack_size - 2) as i32)); // Res
                *stack_size -= 1;
            },
        }
    }

    // CodeChunk = <<
    //   ChunkName:4/unit:8 = "Code",
    //   ChunkSize:32/big,
    //   SubSize:32/big,
    //   InstructionSet:32/big,        % Must match code version in the emulator
    //   OpcodeMax:32/big,
    //   LabelCount:32/big,
    //   FunctionCount:32/big,
    //   Code:(ChunkSize-SubSize)/binary,  % all remaining data
    //   Padding4:0..3/unit:8
    // >>
    pub fn encode_code_chunk<'a>(program: &'a HashMap<String, Expr>, imports: &HashMap<(u32, u32, u32), u32>, atoms: &mut Atoms, labels: &mut HashMap<u32, u32>) -> Vec<u8> {
        let mut label_count: u32 = 0;
        let mut function_count: u32 = 0;

        let mut code = Vec::new();
        for (name, expr) in program.iter() {
            function_count += 1;

            label_count += 1;
            code.push(OpCode::Label as u8);
            code.extend(encode_arg(Tag::U, label_count as i32));

            code.push(OpCode::FuncInfo as u8);
            code.extend(encode_arg(Tag::A, atoms.get_id("bada") as i32));
            let name_id = atoms.get_id(&name);
            code.extend(encode_arg(Tag::A, name_id as i32));
            code.extend(encode_arg(Tag::U, 0));

            label_count += 1;
            code.push(OpCode::Label as u8);
            code.extend(encode_arg(Tag::U, label_count as i32));
            labels.insert(name_id as u32, label_count);

            let mut stack_size = 0;
            compile_expr(expr, atoms, imports, &mut code, &mut stack_size);

            code.push(OpCode::Return as u8);
        }
        code.push(OpCode::IntCodeEnd as u8);

        label_count += 1;

        let sub_size: u32 = 16;
        let instruction_set: u32 = 0;
        let opcode_max: u32 = 169;

        let mut chunk = Vec::new();
        chunk.extend(sub_size.to_be_bytes());
        chunk.extend(instruction_set.to_be_bytes());
        chunk.extend(opcode_max.to_be_bytes());
        chunk.extend(label_count.to_be_bytes());
        chunk.extend(function_count.to_be_bytes());
        chunk.extend(code);

        encode_chunk(*b"Code", chunk)
    }

    // AtomChunk = <<
    //   ChunkName:4/unit:8 = "Atom" | "AtU8",
    //   ChunkSize:32/big,
    //   NumberOfAtoms:32/big,
    //   [<<AtomLength:8, AtomName:AtomLength/unit:8>> || repeat NumberOfAtoms],
    //   Padding4:0..3/unit:8
    // >>
    pub fn encode_atom_chunk(atoms: &Atoms) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend((atoms.names.len() as u32).to_be_bytes());
        for atom in atoms.names.iter() {
            chunk.extend((atom.len() as u8).to_be_bytes());
            chunk.extend(atom.as_bytes());
        }

        encode_chunk(*b"AtU8", chunk)
    }

    fn resolve_function_signature(atoms: &mut Atoms, module: &str, func: &str, arity: u32) -> (u32, u32, u32) {
        (atoms.get_id(module) as u32,
         atoms.get_id(func) as u32,
         arity)
    }

    // ImportChunk = <<
    //   ChunkName:4/unit:8 = "ImpT",
    //   ChunkSize:32/big,
    //   ImportCount:32/big,
    //   [ << ModuleName:32/big,
    //        FunctionName:32/big,
    //        Arity:32/big
    //     >> || repeat ImportCount ],
    //   Padding4:0..3/unit:8
    // >>
    pub fn encode_imports_chunk(atoms: &mut Atoms, imports: &mut HashMap<(u32, u32, u32), u32>) -> Vec<u8> {
        let mut chunk = Vec::new();
        let import_count: u32 = 1;
        chunk.extend(import_count.to_be_bytes());
        let (module, func, arity) = resolve_function_signature(atoms, "erlang", "+", 2);
        chunk.extend(module.to_be_bytes());
        chunk.extend(func.to_be_bytes());
        chunk.extend(arity.to_be_bytes());
        imports.insert((module, func, arity), 0);

        encode_chunk(*b"ImpT", chunk)
    }

    // ExportChunk = <<
    //   ChunkName:4/unit:8 = "ExpT",
    //   ChunkSize:32/big,
    //   ExportCount:32/big,
    //   [ << FunctionName:32/big,
    //        Arity:32/big,
    //        Label:32/big
    //     >> || repeat ExportCount ],
    //   Padding4:0..3/unit:8
    // >>
    pub fn encode_exports_chunk(labels: &HashMap<u32, u32>) -> Vec<u8> {
        let mut chunk = Vec::new();
        let export_count: u32 = labels.len() as u32;
        chunk.extend(export_count.to_be_bytes());

        for (name_id, label) in labels.iter() {
            chunk.extend(name_id.to_be_bytes());
            chunk.extend(0u32.to_be_bytes());
            chunk.extend(label.to_be_bytes());
        }

        encode_chunk(*b"ExpT", chunk)
    }

    // StringChunk = <<
    //   ChunkName:4/unit:8 = "StrT",
    //   ChunkSize:32/big,
    //   Data:ChunkSize/binary,
    //   Padding4:0..3/unit:8
    // >>
    pub fn encode_string_chunk() -> Vec<u8> {
        encode_chunk(*b"StrT", vec![])
    }

    #[derive(Default)]
    pub struct Atoms {
        names: Vec<String>,
    }

    impl Atoms {
        pub fn get_id(&mut self, needle: &str) -> usize {
            let result = self.names
                .iter()
                .enumerate()
                .find(|(_, name)| name == &needle)
                .map(|(index, _)| index + 1);
            if let Some(id) = result {
                id
            } else {
                self.names.push(needle.to_string());
                self.names.len()
            }
        }
    }
}

#[macro_use]
mod diag {
    pub struct Loc {
        pub file_path: String,
        pub row: usize,
        pub col: usize,
    }

    macro_rules! report {
        ($loc:expr, $level:literal, $fmt:literal) => {
            let diag::Loc{file_path, row, col} = $loc;
            let level = $level;
            eprint!("{file_path}:{row}:{col}: {level}: ");
            eprintln!($fmt);
        };
        ($loc:expr, $level:literal, $fmt:literal, $($args:tt)*) => {
            let Loc{file_path, row, col} = $loc;
            let level = $level;
            eprint!("{file_path}:{row}:{col}: {level}: ");
            eprintln!($fmt, $($args)*);
        };
    }
}

mod lex {
    use diag::Loc;

    #[derive(PartialEq)]
    pub enum TokenKind {
        Ident,
        Equals,
        Plus,
        SemiColon,
        Number,
        End,
        Unknown
    }

    impl TokenKind {
        fn human(&self) -> &str {
            match self {
                Self::Ident => "identifier",
                Self::Equals => "equals",
                Self::Plus => "plus",
                Self::SemiColon => "semi-colon",
                Self::Number => "number",
                Self::End => "end of input",
                Self::Unknown => "unknown token",
            }
        }
    }

    pub struct Token {
        pub kind: TokenKind,
        pub text: String,
        pub loc: Loc,
    }

    pub struct Lexer<'a> {
        content: &'a [char],
        file_path: String,
        pos: usize,
        bol: usize,
        row: usize,
    }

    impl<'a> Lexer<'a> {
        pub fn new(content: &'a [char], file_path: String) -> Self {
            Self {content, file_path, pos: 0, bol: 0, row: 0}
        }

        pub fn expect_tokens(&mut self, expected_kinds: &[TokenKind]) -> Option<Token> {
            let token = self.next_token();
            for kind in expected_kinds {
                if token.kind == *kind {
                    return Some(token)
                }
            }

            let mut expected_list = String::new();
            for (i, kind) in expected_kinds.iter().enumerate() {
                if i == 0 {
                    expected_list.push_str(&format!("{}", kind.human()))
                } else if i + 1 >= expected_kinds.len() {
                    expected_list.push_str(&format!(", or {}", kind.human()))
                } else {
                    expected_list.push_str(&format!(", {}", kind.human()))
                }
            }

            report!(token.loc, "ERROR", "Expected {expected_list}, but got {actual}",
                    actual = token.kind.human());
            None
        }

        pub fn next_token(&mut self) -> Token {
            self.trim_whitespaces();

            let loc = Loc {
                file_path: self.file_path.clone(),
                row: self.row + 1,
                col: self.pos - self.bol + 1,
            };

            let x = if let Some(x) = self.current_char() {
                x
            } else {
                return Token {
                    text: "".to_string(),
                    loc,
                    kind: TokenKind::End,
                }
            };

            if x.is_alphabetic() {
                let mut text = String::new();
                while let Some(x) = self.current_char() {
                    if x.is_alphanumeric() {
                        self.chop_char();
                        text.push(x);
                    } else {
                        break;
                    }
                }
                return Token {
                    text,
                    loc,
                    kind: TokenKind::Ident,
                }
            }

            if x.is_numeric() {
                let mut text = String::new();
                while let Some(x) = self.current_char() {
                    if x.is_numeric() {
                        self.chop_char();
                        text.push(x);
                    } else {
                        break;
                    }
                }
                return Token {
                    text,
                    loc,
                    kind: TokenKind::Number,
                }
            }

            match x {
                ';' => {
                    self.chop_char();
                    return Token {
                        text: x.to_string(),
                        loc,
                        kind: TokenKind::SemiColon,
                    }
                }
                '+' => {
                    self.chop_char();
                    return Token {
                        text: x.to_string(),
                        loc,
                        kind: TokenKind::Plus,
                    }
                }
                '=' => {
                    self.chop_char();
                    return Token {
                        text: x.to_string(),
                        loc,
                        kind: TokenKind::Equals,
                    }
                }
                _ => {}
            }

            self.chop_char();
            Token {
                text: x.to_string(),
                loc,
                kind: TokenKind::Unknown,
            }
        }

        fn trim_whitespaces(&mut self) {
            while self.current_char().map(|x| x.is_whitespace()).unwrap_or(false) {
                self.chop_char();
            }
        }

        fn current_char(&self) -> Option<char> {
            self.content.get(self.pos).cloned()
        }

        fn chop_char(&mut self) {
            if let Some(x) = self.current_char() {
                self.pos += 1;
                if x == '\n' {
                    self.row += 1;
                    self.bol = self.pos;
                }
            }
        }
    }
}

mod parser {
    use diag;
    use lex;
    use std::collections::HashMap;

    pub enum Expr {
        Number(usize),
        Sum{lhs: Box<Expr>, rhs: Box<Expr>}
    }

    pub fn parse_program(lexer: &mut lex::Lexer) -> Option<HashMap<String, Expr>> {
        let mut program = HashMap::new();
        loop {
            let ident = lexer.expect_tokens(&[
                lex::TokenKind::Ident,
                lex::TokenKind::End
            ])?;
            match ident.kind {
                lex::TokenKind::Ident => {
                    let _ = lexer.expect_tokens(&[lex::TokenKind::Equals])?;
                    let number = lexer.expect_tokens(&[lex::TokenKind::Number])?;
                    let lhs = match number.text.parse::<usize>() {
                        Ok(lhs) => lhs,
                        Err(err) => {
                            report!(&number.loc, "ERROR", "Could not parse number: {err}");
                            return None
                        }
                    };
                    let token = lexer.expect_tokens(&[
                        lex::TokenKind::SemiColon,
                        lex::TokenKind::Plus,
                    ])?;
                    match token.kind {
                        lex::TokenKind::SemiColon => {
                            program.insert(ident.text, Expr::Number(lhs));
                        }
                        lex::TokenKind::Plus => {
                            let number = lexer.expect_tokens(&[lex::TokenKind::Number])?;
                            let rhs = match number.text.parse::<usize>() {
                                Ok(rhs) => rhs,
                                Err(err) => {
                                    report!(&number.loc, "ERROR", "Could not parse number: {err}");
                                    return None
                                }
                            };
                            program.insert(ident.text, Expr::Sum{
                                lhs: Box::new(Expr::Number(lhs)),
                                rhs: Box::new(Expr::Number(rhs)),
                            });
                            lexer.expect_tokens(&[lex::TokenKind::SemiColon])?;
                        }
                        _ => unreachable!(),
                    }
                }
                lex::TokenKind::End => return Some(program),
                _ => unreachable!(),
            }
        }
    }
}

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
    let program = if let Some(program) = parser::parse_program(&mut lexer) {
        program
    } else {
        return ExitCode::FAILURE;
    };

    let mut atoms = asm::Atoms::default();
    let mut labels: HashMap<u32, u32> = HashMap::new();
    let mut imports: HashMap<(u32, u32, u32), u32> = HashMap::new();

    let _ = atoms.get_id("bada");

    let mut beam = Vec::new();
    beam.extend("BEAM".as_bytes());
    beam.extend(asm::encode_imports_chunk(&mut atoms, &mut imports));
    beam.extend(asm::encode_code_chunk(&program, &imports, &mut atoms, &mut labels));
    beam.extend(asm::encode_exports_chunk(&labels));
    beam.extend(asm::encode_string_chunk());
    beam.extend(asm::encode_atom_chunk(&atoms));

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
