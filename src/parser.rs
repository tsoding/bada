use crate::lex::{Token, TokenKind, Lexer};
use std::collections::HashMap;

pub enum BinopKind {
    Sum,
    Sub,
}

fn binop_of_token(kind: TokenKind) -> Option<BinopKind> {
    match kind {
        TokenKind::Plus => Some(BinopKind::Sum),
        TokenKind::Minus => Some(BinopKind::Sub),
        _ => None,
    }
}

pub struct Binop {
    pub kind: BinopKind,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>
}

pub enum Expr {
    Number(usize),
    Var(Token),
    Binop(Binop),
}

impl Expr {
    fn parse(lexer: &mut Lexer) -> Option<Self> {
        let token = lexer.expect_tokens([TokenKind::Number, TokenKind::Ident])?;
        match token.kind {
            TokenKind::Ident => Some(Expr::Var(token)),
            TokenKind::Number => {
                match token.text.parse::<usize>() {
                    Ok(number) => Some(Expr::Number(number)),
                    Err(err) => {
                        report!(&token.loc, "ERROR", "Could not parse number: {err}");
                        None
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

pub struct Func {
    pub name: Token,
    pub params: HashMap<String, Param>,
    pub body: Expr,
}

pub enum Type {
    Int,
}

impl Type {
    fn parse(lexer: &mut Lexer) -> Option<Self> {
        let ident = lexer.expect_tokens([TokenKind::Ident])?;
        match ident.text.as_str() {
            "int" => Some(Type::Int),
            unknown => {
                report!(&ident.loc, "ERROR", "Unknown type `{unknown}`");
                None
            }
        }
    }
}

pub struct Param {
    pub name: Token,
    pub typ: Type,
    pub index: usize,
}

#[derive(Default)]
pub struct Module {
    pub funcs: HashMap<String, Func>,
}

impl Module {
    pub fn parse(lexer: &mut Lexer) -> Option<Module> {
        // module should not be defined in this way but in main (and therefore be
        // self) and the result of the function should be a Result<(), Error>.
        let mut module = Module::default();

        loop {
            let name = lexer.expect_tokens([
                TokenKind::Ident,
                TokenKind::End
            ])?;

            match name.kind {
                TokenKind::Ident => {
                    // TODO: redefinition of the function should be allowed for function with different arity
                    if let Some(existing_func) = module.funcs.get(&name.text) {
                        report!(&name.loc, "ERROR", "Redefinition of existing function {name}", name = name.text);
                        report!(&existing_func.name.loc, "INFO", "The existing function is defined here");
                        return None;
                    }

                    lexer.expect_tokens([TokenKind::OpenParen])?;

                    let mut params: HashMap<String, Param> = HashMap::new();
                    'parse_params: loop {
                        let name = lexer.expect_tokens([TokenKind::Ident, TokenKind::ClosedParen])?;
                        match name.kind {
                            TokenKind::Ident => {
                                // to test only if it's an ident
                                if let Some(existing_param) = params.get(&name.text) {
                                    report!(&name.loc, "ERROR", "Redefinition of existing parameter {name}", name = name.text);
                                    report!(&existing_param.name.loc, "INFO", "The existing parameter is defined here");
                                    return None;
                                }

                                let typ = Type::parse(lexer)?;
                                let index = params.len();
                                params.insert(name.text.clone(), Param {name, typ, index});
                            },
                            TokenKind::ClosedParen => break 'parse_params,
                            _ => unreachable!()
                        }
                    }

                    lexer.expect_tokens([TokenKind::Equals])?;

                    // the operations are expressions "a + 2" is an expression, not just "a" and "2"
                    let lhs = Expr::parse(lexer)?;
                    let token = lexer.expect_tokens([
                        TokenKind::SemiColon,
                        TokenKind::Plus,
                        TokenKind::Minus,
                    ])?;
                    match token.kind {
                        TokenKind::SemiColon => {
                            module.funcs.insert(name.text.clone(), Func {
                                name,
                                params,
                                body: lhs
                            });
                        }
                        TokenKind::Plus | TokenKind::Minus => {
                            let rhs = Expr::parse(lexer)?;
                            module.funcs.insert(name.text.clone(), Func {
                                name,
                                params,
                                body: Expr::Binop(Binop {
                                    kind: binop_of_token(token.kind).expect("binop kind"),
                                    lhs: Box::new(lhs),
                                    rhs: Box::new(rhs),
                                })
                            });
                            lexer.expect_tokens([TokenKind::SemiColon])?;
                        }
                        _ => unreachable!(),
                    }
                }
                TokenKind::End => return Some(module),
                _ => unreachable!(),
            }
        }
    }
}
