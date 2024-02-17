use std::path::Path;
use std::fmt::Display;
use std::rc::Rc;

use crate::diag::Loc;

#[derive(Clone, Copy, PartialEq)]
pub enum TokenKind {
    Ident,
    Number,

    Equals,
    Plus,
    Minus,
    SemiColon,
    OpenParen,
    ClosedParen,
    Colon,

    End,
    Unknown
}

const FIXED_TOKENS: &[(&[char], TokenKind)] = &[
    (&['='], TokenKind::Equals),
    (&['+'], TokenKind::Plus),
    (&['-'], TokenKind::Minus),
    (&[';'], TokenKind::SemiColon),
    (&[':'], TokenKind::Colon),
    (&['('], TokenKind::OpenParen),
    (&[')'], TokenKind::ClosedParen),
];

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ident => "identifier",
            Self::Number => "number",

            Self::Equals => "equals",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::SemiColon => "semi-colon",
            Self::OpenParen => "open paren",
            Self::ClosedParen => "closed paren",
            Self::Colon => "colon",

            Self::End => "end of input",
            Self::Unknown => "unknown token",
        })
    }
}

pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub loc: Loc,
}

// I use Rc<Path> instead of the previous String, because `String::clone` has
// a big cost, Path is made to represent file path (like str to represent
// string) and Rc owned only 1 paths and not 10,000
pub struct Lexer {
    content: Vec<char>,
    file_path: Rc<Path>,
    pos: usize,
    bol: usize,
    row: usize,
}

impl Lexer {
    pub fn new(content: Vec<char>, file_path: Rc<Path>) -> Self {
        Self { content, file_path, pos: 0, bol: 0, row: 0 }
    }

    pub fn expect_tokens<E: AsRef<[TokenKind]>>(&mut self, expected_kinds: E) -> Option<Token> {
        let expected_kinds = expected_kinds.as_ref();

        let token = self.next_token();
        for kind in expected_kinds {
            if token.kind == *kind {
                return Some(token)
            }
        }

        let expected_list = expected_kinds.iter().enumerate()
            .fold(String::new(), |acc, (i, kind)| {
                match i {
                    0 => format!("{kind}"),
                    x if x < expected_kinds.len() - 1 => format!("{acc}, {kind}"),
                    _ => format!("{acc}, or {kind}")
                }
            });

        report!(token.loc, "ERROR", "Expected {expected_list}, but got {}", token.kind);
        None
    }

    fn starts_with(&self, prefix: &[char]) -> bool {
        self.content[self.pos..].starts_with(prefix)
    }

    fn drop_line(&mut self) {
        while let Some(x) = self.current_char() {
            self.chop_char();
            if x == '\n' {
                break
            }
        }
    }

    fn next_token(&mut self) -> Token {
        // TODO: move into it's owned function
        'trim_whitespaces_and_comments: loop {
            self.trim_whitespaces();
            if self.starts_with(&['/', '/']) {
                self.drop_line();
            } else {
                break 'trim_whitespaces_and_comments
            }
        }

        let loc = Loc {
            file_path: Rc::clone(&self.file_path),
            row: self.row + 1,
            col: self.pos - self.bol + 1,
        };

        let (text, kind) = match self.current_char() {
            Some(x) if x.is_alphabetic() => (self.lex_ident(), TokenKind::Ident),
            Some(x) if x.is_numeric() => (self.lex_number(), TokenKind::Number),
            Some(x) => 'fixed_tokens: {
                for &(prefix, kind) in FIXED_TOKENS.iter() {
                    if self.starts_with(prefix) {
                        self.chop_chars(prefix.len());
                        break 'fixed_tokens (prefix.iter().collect(), kind);
                    }
                }
        
                self.chop_char();

                (x.to_string(), TokenKind::Unknown)
            }
            None => (String::new(), TokenKind::End)
        };

        Token { text, loc, kind }
    }

    fn lex_ident(&mut self) -> String {
        let mut text = String::new();
        while let Some(x) = self.current_char() {
            if x.is_alphanumeric() {
                self.chop_char();
                text.push(x);
            } else {
                break;
            }
        }
        text
    }

    fn lex_number(&mut self) -> String {
        let mut text = String::new();
        while let Some(x) = self.current_char() {
            if x.is_numeric() {
                self.chop_char();
                text.push(x);
            } else {
                break;
            }
        }
        text
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
        // to avoid unnecessary clone
        if let Some(x) = self.content.get(self.pos) {
            self.pos += 1;
            if *x == '\n' {
                self.row += 1;
                self.bol = self.pos;
            }
        }
    }

    fn chop_chars(&mut self, n: usize) {
        for _ in 0..n {
            self.chop_char()
        }
    }
}
