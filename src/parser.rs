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
pub struct Program {
    pub funcs: HashMap<String, Func>,
}

pub fn parse_program(lexer: &mut Lexer) -> Option<Program> {
    let mut program = Program::default();
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
                        program.funcs.insert(name.text.clone(), Func {
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
                        program.funcs.insert(name.text.clone(), Func {
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
            TokenKind::End => return Some(program),
            _ => unreachable!(),
        }
    }
}
