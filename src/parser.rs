use diag;
use lex::{Token, TokenKind, Lexer};
use std::collections::HashMap;

pub enum Expr {
    Number(usize),
    Sum{lhs: Box<Expr>, rhs: Box<Expr>}
}

pub struct Func {
    pub name: Token,
    pub body: Expr,
}

#[derive(Default)]
pub struct Module {
    pub funcs: HashMap<String, Func>,
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
                let _ = lexer.expect_tokens(&[TokenKind::Equals])?;
                let number = lexer.expect_tokens(&[TokenKind::Number])?;
                let lhs = match number.text.parse::<usize>() {
                    Ok(lhs) => lhs,
                    Err(err) => {
                        report!(&number.loc, "ERROR", "Could not parse number: {err}");
                        return None
                    }
                };
                let token = lexer.expect_tokens(&[
                    TokenKind::SemiColon,
                    TokenKind::Plus,
                ])?;
                match token.kind {
                    TokenKind::SemiColon => {
                        module.funcs.insert(name.text.clone(), Func {
                            name,
                            body: Expr::Number(lhs)
                        });
                    }
                    TokenKind::Plus => {
                        let number = lexer.expect_tokens(&[TokenKind::Number])?;
                        let rhs = match number.text.parse::<usize>() {
                            Ok(rhs) => rhs,
                            Err(err) => {
                                report!(&number.loc, "ERROR", "Could not parse number: {err}");
                                return None
                            }
                        };
                        module.funcs.insert(name.text.clone(), Func {
                            name,
                            body: Expr::Sum{
                                lhs: Box::new(Expr::Number(lhs)),
                                rhs: Box::new(Expr::Number(rhs)),
                            }
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
