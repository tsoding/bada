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
