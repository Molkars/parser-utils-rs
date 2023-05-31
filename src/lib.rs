#![allow(dead_code)]

use std::ops::Range;
use std::panic::Location;

pub mod error;

#[derive(Debug, Clone)]
pub struct Token<Kind> {
    pub kind: Kind,
    pub index: u32,
    pub len: u32,
}

pub struct SimpleTokenizer<'a> {
    source: &'a str,
    index: usize,
    start: usize,
}

#[derive(Debug)]
pub enum TokenizeErrorKind {
    ExpectedChar {
        expected: char,
        got: char,
    },
    UnexpectedChar {
        got: char,
    },
    UnexpectedEndOfInput,
    Custom {
        message: &'static str,
    }
}

#[derive(Debug)]
pub struct TokenizeError {
    index: u32,
    kind: TokenizeErrorKind,
    #[cfg(debug_assertions)]
    source: &'static Location<'static>,
}

impl<Kind> Token<Kind> {
    pub fn range(tok: &Self) -> Range<usize> {
        let index: usize = tok.index.try_into().expect("token index too big");
        let len: usize = tok.len.try_into().expect("token len too big");
        index..index + len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionInfo {
    pub line: u32,
    pub column: u32,
    pub line_start_index: usize,
    pub index: usize,
}

impl TokenizeError {
    pub fn position(this: &Self, source: &str) -> PositionInfo {
        let index = this.index.try_into().expect("input too big!");
        let mut line = 1;
        let mut line_start_index = 0;
        let mut column = 0;
        let mut str_index = 0;

        for c in source.chars() {
            if c == '\n' {
                line += 1;
                line_start_index = str_index;
                column = 0;
            } else {
                column += 1;
            }
            str_index += c.len_utf8();
            if str_index >= index {
                break;
            }
        }

        PositionInfo {
            line,
            column,
            line_start_index,
            index,
        }
    }


    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn kind(&self) -> &TokenizeErrorKind {
        &self.kind
    }

    #[cfg(debug_assertions)]
    pub fn trace(&self) -> &'static Location<'static> {
        self.source
    }
}

impl<'a> SimpleTokenizer<'a> {
    pub fn begin_token(&mut self) {
        self.start = self.index;
    }

    pub fn set_index(&mut self, index: u32) {
        let index = index.try_into().expect("input too big!");
        if self.start > index {
            self.start = index;
        }
        self.index = index;
    }

    pub fn get_start(&mut self) -> u32 {
        self.start.try_into().expect("input too big!")
    }

    pub fn get_index(&mut self) -> u32 {
        self.index.try_into().expect("input too big!")
    }

    pub fn slice(&self, range: Range<u32>) -> &'a str {
        let range: Range<usize> = range.start.try_into().expect("input too big!")
            ..range.end.try_into().expect("input too big!");
        &self.source[range]
    }

    #[track_caller]
    pub fn take(&mut self) -> Result<char, TokenizeError> {
        self.source[self.index..].chars().next()
            .ok_or(TokenizeError {
                index: self.index.try_into().expect("input too big!"),
                kind: TokenizeErrorKind::UnexpectedEndOfInput,
                #[cfg(debug_assertions)]
                source: Location::caller(),
            })
            .map(|char| {
                self.index += char.len_utf8();
                char
            })
    }

    pub fn content<Kind>(&self, tok: &Token<Kind>) -> Option<&str> {
        let start = usize::try_from(tok.index).expect("token index too big");
        let end = usize::try_from(tok.index + tok.len).expect("token end too long");
        self.source.get(start..end)
    }

    #[track_caller]
    pub fn peek(&self) -> Result<char, TokenizeError> {
        self.source[self.index..].chars().next()
            .ok_or(TokenizeError {
                index: self.index.try_into().expect("input too big!"),
                kind: TokenizeErrorKind::UnexpectedEndOfInput,
                #[cfg(debug_assertions)]
                source: Location::caller(),
            })
    }

    pub fn take_while(&mut self, predicate: impl Fn(char) -> bool) -> Option<&str> {
        let start = self.index;
        while let Some(c) = self.peek().ok().filter(|c| predicate(*c)) {
            self.index += c.len_utf8();
        }
        (start != self.index).then(|| &self.source[start..self.index])
    }

    pub fn expect(&mut self, expected: char) -> Result<(), TokenizeError> {
        match self.peek()? {
            char if char.eq(&expected) => {
                self.index += char.len_utf8();
                Ok(())
            }
            char => Err(TokenizeError {
                index: self.index.try_into().expect("input too big!"),
                kind: TokenizeErrorKind::ExpectedChar {
                    expected,
                    got: char,
                },
                #[cfg(debug_assertions)]
                source: Location::caller(),
            }),
        }
    }

    pub fn end_token<Kind>(&mut self, kind: Kind) -> Token<Kind> {
        Token {
            index: u32::try_from(self.start).expect("input too big"),
            len: u32::try_from(self.index - self.start).expect("input too big"),
            kind,
        }
    }

    #[track_caller]
    pub fn unexpected(&self, got: char) -> TokenizeError {
        TokenizeError {
            index: self.index.try_into().expect("input too big!"),
            kind: TokenizeErrorKind::UnexpectedChar { got },
            #[cfg(debug_assertions)]
            source: Location::caller(),
        }
    }

    #[track_caller]
    pub fn custom(&self, message: &'static str) -> TokenizeError {
        TokenizeError {
            index: self.index.try_into().expect("input too big!"),
            kind: TokenizeErrorKind::Custom { message },
            #[cfg(debug_assertions)]
            source: Location::caller(),
        }
    }

    pub fn matches(&self, expected: char) -> bool {
        self.peek().ok().filter(|c| expected.eq(c)).is_some()
    }

    pub fn has_more_chars(&self) -> bool {
        self.index < self.source.len()
    }

    pub fn match_and_take(&mut self, expected: char) -> bool {
        if self.matches(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }
}

impl<'a> From<&'a str> for SimpleTokenizer<'a> {
    fn from(value: &'a str) -> Self {
        SimpleTokenizer {
            source: value,
            index: 0,
            start: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParseErrorKind<Kind> {
    UnexpectedToken(Token<Kind>),
    ExpectedToken {
        expected: Kind,
        got: Token<Kind>,
    },
    ExpectedString {
        expected: String,
        got: String,
        token: Token<Kind>,
    },
    UnexpectedEndOfInput,
}

#[derive(Debug, Clone)]
pub struct ParseError<Kind> {
    kind: ParseErrorKind<Kind>,
    #[cfg(debug_assertions)]
    source: &'static Location<'static>,
}

pub struct Tokens<Kind> {
    inner: Vec<Token<Kind>>,
}

impl<Kind> FromIterator<Token<Kind>> for Tokens<Kind> {
    fn from_iter<T: IntoIterator<Item=Token<Kind>>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

pub struct TokenView<'a, Kind> {
    source: &'a str,
    tokens: &'a Tokens<Kind>,
    index: usize,
}

impl<'a, Kind> TokenView<'a, Kind> {
    pub fn new(source: &'a str, tokens: &'a Tokens<Kind>) -> Self {
        Self {
            source,
            tokens,
            index: 0,
        }
    }

    #[track_caller]
    pub fn peek(&self) -> Result<&'a Token<Kind>, ParseError<Kind>> {
        self.tokens.inner.get(self.index).ok_or(ParseError {
            kind: ParseErrorKind::UnexpectedEndOfInput,
            #[cfg(debug_assertions)]
            source: Location::caller(),
        })
    }

    pub fn peekn(&self, off: isize) -> Option<&'a Token<Kind>> {
        let pos = isize::try_from(self.index).expect("too many tokens!");
        let pos = pos + off;
        usize::try_from(pos).ok().and_then(|pos| self.tokens.inner.get(pos))
    }

    #[track_caller]
    pub fn take(&mut self) -> Result<&'a Token<Kind>, ParseError<Kind>> {
        if self.index < self.tokens.inner.len() {
            let token = &self.tokens.inner[self.index];
            self.index += 1;
            Ok(token)
        } else {
            Err(ParseError {
                kind: ParseErrorKind::UnexpectedEndOfInput,
                #[cfg(debug_assertions)]
                source: Location::caller(),
            })
        }
    }

    #[track_caller]
    pub fn content_take(&mut self) -> Result<&'a str, ParseError<Kind>> {
        self.take().map(|tok| self.content(tok).expect("token content not in source"))
    }

    #[track_caller]
    pub fn expect(&mut self, kind: Kind) -> Result<&'a Token<Kind>, ParseError<Kind>> where Kind: Eq + Clone {
        let token = self.take()?;
        match token {
            token if token.kind == kind => Ok(token),
            token => Err(ParseError {
                kind: ParseErrorKind::ExpectedToken {
                    expected: kind,
                    got: token.clone(),
                },
                #[cfg(debug_assertions)]
                source: Location::caller(),
            }),
        }
    }

    #[track_caller]
    pub fn content_expect(&mut self, kind: Kind) -> Result<&'a str, ParseError<Kind>> where Kind: Eq + Clone {
        self.expect(kind)
            .map(|tok| self.content(tok).expect("token content not in source"))
    }

    pub fn content(&self, token: &'a Token<Kind>) -> Option<&'a str> {
        let start = usize::try_from(token.index).expect("token index too big");
        let end = usize::try_from(token.index + token.len).expect("token end too long");
        self.source.get(start..end)
    }

    #[track_caller]
    pub fn content_matches(&self, input: impl AsRef<str>) -> Result<&'a str, ParseError<Kind>> where Kind: Clone {
        let input = String::from(input.as_ref());
        let token = self.peek()?;
        let content = self.content(token).expect("token content not in source");
        if content == input {
            Ok(content)
        } else {
            Err(ParseError {
                kind: ParseErrorKind::ExpectedString {
                    expected: input,
                    got: String::from(content),
                    token: token.clone(),
                },
                #[cfg(debug_assertions)]
                source: Location::caller(),
            })
        }
    }

    #[track_caller]
    pub fn matches(&self, kind: Kind) -> bool where Kind: Eq {
        self.peek().ok().filter(|token| token.kind == kind).is_some()
    }

    #[track_caller]
    pub fn match_and_take(&mut self, kind: Kind) -> bool where Kind: Eq {
        self.peek().ok().filter(|token| token.kind == kind)
            .map(|_| self.index += 1)
            .is_some()
    }

    pub fn has_more_tokens(&self) -> bool {
        self.index < self.tokens.inner.len()
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn set_position(&mut self, idx: usize) -> bool {
        if idx < self.tokens.inner.len() {
            self.index = idx;
            true
        } else {
            false
        }
    }

    #[track_caller]
    pub fn unexpected(&self, token: &'a Token<Kind>) -> ParseError<Kind> where Kind: Clone {
        ParseError {
            kind: ParseErrorKind::UnexpectedToken(token.clone()),
            #[cfg(debug_assertions)]
            source: Location::caller(),
        }
    }

    #[track_caller]
    pub fn unexpected_end(&self) -> ParseError<Kind> {
        ParseError {
            kind: ParseErrorKind::UnexpectedEndOfInput,
            #[cfg(debug_assertions)]
            source: Location::caller(),
        }
    }
}

#[derive(Debug)]
pub enum Error<Kind> {
    Tokenizer(TokenizeError),
    Parser(ParseError<Kind>),
}