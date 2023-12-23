use diag::*;
use lex::{Token, TokenKind, Lexer};
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
        let token = lexer.expect_tokens(&[TokenKind::Number, TokenKind::Ident])?;
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
    pub params: Vec<Param>,
    pub body: Expr,
}

#[derive(Default)]
pub struct Module {
    pub funcs: HashMap<String, Func>,
}

pub enum Type {
    Int,
}

impl Type {
    fn parse(lexer: &mut Lexer) -> Option<Self> {
        let ident = lexer.expect_tokens(&[TokenKind::Ident])?;
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
    pub typ: Type
}

pub fn parse_module(lexer: &mut Lexer) -> Option<Module> {
    let mut module = Module::default();
    loop {
        let name = lexer.expect_tokens(&[
            TokenKind::Ident,
            TokenKind::End
        ])?;
        match name.kind {
            TokenKind::Ident => {
                let _ = lexer.expect_tokens(&[TokenKind::OpenParen])?;

                let mut params: Vec<Param> = Vec::new();
                'parse_params: loop {
                    let name = lexer.expect_tokens(&[TokenKind::Ident, TokenKind::ClosedParen])?;
                    if let Some(existing_param) = params.iter().find(|param| param.name.text == name.text) {
                        report!(&name.loc, "ERROR", "Redefinition of existing parameter {name}", name = name.text);
                        report!(&existing_param.name.loc, "INFO", "The existing parameter is defined here");
                        return None;
                    }
                    match name.kind {
                        TokenKind::Ident => {
                            let typ = Type::parse(lexer)?;
                            params.push(Param {name, typ});
                        },
                        TokenKind::ClosedParen => break 'parse_params,
                        _ => unreachable!()
                    }
                }

                let _ = lexer.expect_tokens(&[TokenKind::Equals])?;
                let lhs = Expr::parse(lexer)?;
                let token = lexer.expect_tokens(&[
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
                        // TODO: check for function redefinition
                        module.funcs.insert(name.text.clone(), Func {
                            name,
                            params,
                            body: Expr::Binop(Binop {
                                kind: binop_of_token(token.kind).expect("binop kind"),
                                lhs: Box::new(lhs),
                                rhs: Box::new(rhs),
                            })
                        });
                        lexer.expect_tokens(&[TokenKind::SemiColon])?;
                    }
                    _ => unreachable!(),
                }
            }
            TokenKind::End => return Some(module),
            _ => unreachable!(),
        }
    }
}
