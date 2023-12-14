use diag::Loc;

#[derive(Clone, Copy, PartialEq)]
pub enum TokenKind {
    Ident,
    Equals,
    Plus,
    SemiColon,
    Number,
    End,
    Unknown
}

const FIXED_TOKENS: &[(&[char], TokenKind)] = &[
    (&[';'], TokenKind::SemiColon),
    (&['+'], TokenKind::Plus),
    (&['='], TokenKind::Equals),
];

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

    fn starts_with(&self, prefix: &[char]) -> bool {
        self.content[self.pos..].starts_with(prefix)
    }

    fn next_token(&mut self) -> Token {
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

        for &(prefix, kind) in FIXED_TOKENS.iter() {
            if self.starts_with(prefix) {
                self.chop_chars(prefix.len());
                return Token {
                    text: prefix.iter().collect(),
                    loc,
                    kind,
                }
            }
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

    fn chop_chars(&mut self, n: usize) {
        for _ in 0..n {
            self.chop_char()
        }
    }
}
