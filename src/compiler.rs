use std::collections::HashMap;
use crate::parser::{Expr, Module, Func, BinopKind, Binop, Param};

#[repr(u8)]
enum Tag {
    U = 0,                      // unsigned?
    I = 1,                      // integer
    A = 2,                      // atom
    X = 3,                      // x register
    // Y = 4,                   // y register
    F = 5,                      // label
    // H = 6,                   // character?
    // Z = 7,                   // ?
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

// FIXME: too much parametres
fn compile_expr(expr: &Expr, atoms: &mut Atoms, imports: &HashMap<(u32, u32, u32), u32>, code: &mut Vec<u8>, params: &HashMap<String, Param>, stack_size: &mut usize) -> Option<()> {
    let stack_start = params.len();
    match expr {
        Expr::Var(name) => {
            match params.get(&name.text) {
                Some(param) => {
                    code.push(OpCode::Move as u8);
                    code.extend(encode_arg(Tag::X, param.index as i32));
                    code.extend(encode_arg(Tag::X, (stack_start + *stack_size) as i32));
                    *stack_size += 1;
                    Some(())
                }
                None => {
                    report!(&name.loc, "ERROR", "Unknown variable {name}", name = name.text);
                    None
                }
            }
        }
        Expr::Number(x) => {
            code.push(OpCode::Move as u8);
            code.extend(encode_arg(Tag::I, (*x) as i32));
            code.extend(encode_arg(Tag::X, (stack_start + *stack_size) as i32));
            *stack_size += 1;
            Some(())
        },
        Expr::Binop(Binop{kind, lhs, rhs}) => {
            compile_expr(lhs, atoms, imports, code, params, stack_size)?;
            compile_expr(rhs, atoms, imports, code, params, stack_size)?;

            assert!(*stack_size >= 2);

            code.push(OpCode::GcBif2 as u8);
            code.extend(encode_arg(Tag::F, 0)); // Lbl
            code.extend(encode_arg(Tag::U, 2)); // Live
            let bif2 = match kind {
                BinopKind::Sum => imports
                    .get(&resolve_function_signature(atoms, "erlang", "+", 2))
                    .cloned()
                    .expect("erlang:'+' should be always present"),
                BinopKind::Sub => imports
                    .get(&resolve_function_signature(atoms, "erlang", "-", 2))
                    .cloned()
                    .expect("erlang:'-' should be always present"),
            };
            code.extend(encode_arg(Tag::U, bif2 as i32)); // Bif
            code.extend(encode_arg(Tag::X, (stack_start + *stack_size - 2) as i32)); // Arg1
            code.extend(encode_arg(Tag::X, (stack_start + *stack_size - 1) as i32)); // Arg2
            code.extend(encode_arg(Tag::X, (stack_start + *stack_size - 2) as i32)); // Res
            *stack_size -= 1;
            Some(())
        },
    }
}

struct CompiledFunc {
    label: u32,
    arity: u32,
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
fn encode_code_chunk<'a>(module: &'a Module, imports: &HashMap<(u32, u32, u32), u32>, atoms: &mut Atoms, labels: &mut HashMap<u32, CompiledFunc>) -> Vec<u8> {
    let mut label_count: u32 = 0;
    let mut function_count: u32 = 0;

    let mut code = Vec::new();
    for (_, Func{name, params, body}) in module.funcs.iter() {
        function_count += 1;

        label_count += 1;
        code.push(OpCode::Label as u8);
        code.extend(encode_arg(Tag::U, label_count as i32));

        code.push(OpCode::FuncInfo as u8);
        code.extend(encode_arg(Tag::A, atoms.get_id("bada") as i32));
        let name_id = atoms.get_id(&name.text);
        code.extend(encode_arg(Tag::A, name_id as i32));
        code.extend(encode_arg(Tag::U, params.len() as i32));

        label_count += 1;
        code.push(OpCode::Label as u8);
        code.extend(encode_arg(Tag::U, label_count as i32));
        labels.insert(name_id, CompiledFunc {
            label: label_count,
            arity: params.len() as u32,
        });

        let mut stack_size = 0;
        compile_expr(body, atoms, imports, &mut code, &params, &mut stack_size);

        if params.len() > 0 {
            code.push(OpCode::Move as u8);
            code.extend(encode_arg(Tag::X, params.len() as i32));
            code.extend(encode_arg(Tag::X, 0i32));
        }
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
fn encode_atom_chunk(atoms: &Atoms) -> Vec<u8> {
    let mut chunk = Vec::new();
    chunk.extend((atoms.names.len() as u32).to_be_bytes());
    for atom in atoms.names.iter() {
        chunk.extend((atom.len() as u8).to_be_bytes());
        chunk.extend(atom.as_bytes());
    }

    encode_chunk(*b"AtU8", chunk)
}

fn resolve_function_signature(atoms: &mut Atoms, module: &str, func: &str, arity: u32) -> (u32, u32, u32) {
    (atoms.get_id(module),
     atoms.get_id(func),
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
fn encode_imports_chunk(atoms: &mut Atoms, imports: &mut HashMap<(u32, u32, u32), u32>) -> Vec<u8> {
    let mut chunk = Vec::new();
    let import_count: u32 = 2;
    chunk.extend(import_count.to_be_bytes());

    let (module, func, arity) = resolve_function_signature(atoms, "erlang", "+", 2);
    chunk.extend(module.to_be_bytes());
    chunk.extend(func.to_be_bytes());
    chunk.extend(arity.to_be_bytes());
    imports.insert((module, func, arity), 0);

    let (module, func, arity) = resolve_function_signature(atoms, "erlang", "-", 2);
    chunk.extend(module.to_be_bytes());
    chunk.extend(func.to_be_bytes());
    chunk.extend(arity.to_be_bytes());
    imports.insert((module, func, arity), 1);

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
fn encode_exports_chunk(labels: &HashMap<u32, CompiledFunc>) -> Vec<u8> {
    let mut chunk = Vec::new();
    let export_count: u32 = labels.len() as u32;
    chunk.extend(export_count.to_be_bytes());

    for (name_id, CompiledFunc{label, arity}) in labels.iter() {
        chunk.extend(name_id.to_be_bytes());
        chunk.extend(arity.to_be_bytes());
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
fn encode_string_chunk() -> Vec<u8> {
    encode_chunk(*b"StrT", vec![])
}

#[derive(Default)]
struct Atoms {
    names: Vec<String>,
}

impl Atoms {
    fn get_id(&mut self, needle: &str) -> u32 {
        let result = self.names
            .iter()
            .enumerate()
            .find(|(_, name)| name == &needle)
            .map(|(index, _)| index + 1);
        if let Some(id) = result {
            id as u32
        } else {
            self.names.push(needle.to_string());
            self.names.len() as u32
        }
    }
}

pub fn compile_beam_module(module: &Module) -> Vec<u8> {
    let mut atoms = Atoms::default();
    let mut labels: HashMap<u32, CompiledFunc> = HashMap::new();
    let mut imports: HashMap<(u32, u32, u32), u32> = HashMap::new();

    // TODO: get module name from the stem of the input file
    let _ = atoms.get_id("bada");

    let mut beam = Vec::new();
    beam.extend("BEAM".as_bytes());
    beam.extend(encode_imports_chunk(&mut atoms, &mut imports));
    beam.extend(encode_code_chunk(&module, &imports, &mut atoms, &mut labels));
    beam.extend(encode_exports_chunk(&labels));
    beam.extend(encode_string_chunk());
    beam.extend(encode_atom_chunk(&atoms));
    beam
}
