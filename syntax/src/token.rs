use super::*;
use chumsky::prelude::*;
use internment::Intern;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Delimiter {
    Paren,
    Brack,
    Brace,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    // Sum
    Add, Sub,
    // Product
    Mul, Div, Rem,
    // Equality
    Eq, NotEq,
    // Comparison
    Less, LessEq,
    More, MoreEq,
    // Logical
    Not, And, Or, Xor,
    // Lists
    Join,
    Ellipsis,
    Dot,
    LArrow,
    RArrow,
    RFlow,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Op::Add => write!(f, "+"),
            Op::Sub => write!(f, "-"),
            Op::Mul => write!(f, "*"),
            Op::Div => write!(f, "/"),
            Op::Rem => write!(f, "%"),
            Op::Eq => write!(f, "="),
            Op::NotEq => write!(f, "!="),
            Op::Less => write!(f, "<"),
            Op::LessEq => write!(f, "<="),
            Op::More => write!(f, ">"),
            Op::MoreEq => write!(f, ">="),
            Op::Not => write!(f, "!"),
            Op::And => write!(f, "and"),
            Op::Or => write!(f, "or"),
            Op::Xor => write!(f, "xor"),
            Op::Join => write!(f, "++"),
            Op::Dot => write!(f, "."),
            Op::Ellipsis => write!(f, ".."),
            Op::LArrow => write!(f, "<-"),
            Op::RArrow => write!(f, "->"),
            Op::RFlow => write!(f, "=>"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub enum Token {
    Nat(u64),
    Num(Intern<String>),
    Char(char),
    Bool(bool),
    Str(Intern<String>),
    Open(Delimiter),
    Close(Delimiter),
    Op(Op),
    TermIdent(ast::Ident),
    TypeIdent(ast::Ident),
    Intrinsic(ast::Ident),
    Comma,
    Separator,
    Colon,
    Wildcard,
    Question,
    Pipe,
    EndPipe,
    Import,
    Data,
    Type,
    Def,
    Class,
    Member,
    For,
    Fn,
    Let,
    If,
    Match,
    Then,
    Else,
    In,
    Of,
    Do,
    Tilde,
    Dollar,
    Semicolon,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Nat(x) => write!(f, "{}", x),
            Token::Num(x) => write!(f, "{}", x),
            Token::Char(c) => write!(f, "{}", c),
            Token::Bool(x) => write!(f, "{}", x),
            Token::Str(s) => write!(f, "{}", s),
            Token::Open(Delimiter::Paren) => write!(f, "("),
            Token::Open(Delimiter::Brack) => write!(f, "["),
            Token::Open(Delimiter::Brace) => write!(f, "{{"),
            Token::Close(Delimiter::Paren) => write!(f, ")"),
            Token::Close(Delimiter::Brack) => write!(f, "]"),
            Token::Close(Delimiter::Brace) => write!(f, "}}"),
            Token::Op(op) => write!(f, "{}", op),
            Token::TermIdent(ident) => write!(f, "{}", ident),
            Token::TypeIdent(ident) => write!(f, "{}", ident),
            Token::Intrinsic(ident) => write!(f, "@{}", ident),
            Token::Comma => write!(f, ","),
            Token::Separator => write!(f, "::"),
            Token::Colon => write!(f, ":"),
            Token::Wildcard => write!(f, "_"),
            Token::Question => write!(f, "?"),
            Token::Pipe => write!(f, "|"),
            Token::EndPipe => write!(f, "\\"),
            Token::Import => write!(f, "import"),
            Token::Data => write!(f, "data"),
            Token::Type => write!(f, "type"),
            Token::Def => write!(f, "def"),
            Token::Class => write!(f, "class"),
            Token::Member => write!(f, "member"),
            Token::For => write!(f, "for"),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::If => write!(f, "if"),
            Token::Match => write!(f, "match"),
            Token::Then => write!(f, "then"),
            Token::Else => write!(f, "else"),
            Token::In => write!(f, "in"),
            Token::Of => write!(f, "of"),
            Token::Do => write!(f, "do"),
            Token::Tilde => write!(f, "~"),
            Token::Dollar => write!(f, "$"),
            Token::Semicolon => write!(f, ";"),
        }
    }
}

pub fn lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Error> {
    let nat = text::int(10)
        .map(|s: String| Token::Nat(s.parse().unwrap()));

    let num = text::int(10)
        .then(just('.'))
        .chain::<char, _, _>(text::digits(10))
        .collect::<String>()
        .map(Intern::new)
        .map(Token::Num);

    let ctrl = choice((
        just(',').to(Token::Comma),
        just("::").to(Token::Separator),
        just(':').to(Token::Colon),
        just('?').to(Token::Question),
        just('|').to(Token::Pipe),
        just('\\').to(Token::EndPipe),
        just('~').to(Token::Tilde),
        just('$').to(Token::Dollar),
        just(';').to(Token::Semicolon),
    ));

    let op = choice((
        just("=>").to(Op::RFlow),
        just('=').to(Op::Eq),
        just("..").to(Op::Ellipsis),
        just('.').to(Op::Dot),
        just("!=").to(Op::NotEq),
        just('!').to(Op::Not),
        just("<-").to(Op::LArrow),
        just("<=").to(Op::LessEq),
        just('<').to(Op::Less),
        just(">=").to(Op::MoreEq),
        just('>').to(Op::More),
        just("++").to(Op::Join),
        just('+').to(Op::Add),
        just("->").to(Op::RArrow),
        just('-').to(Op::Sub),
        just('*').to(Op::Mul),
        just('/').to(Op::Div),
        just('%').to(Op::Rem),
        just('=').to(Op::Eq),
    ))
        .map(Token::Op);

    let delim = choice((
        just('(').to(Token::Open(Delimiter::Paren)),
        just(')').to(Token::Close(Delimiter::Paren)),
        just('[').to(Token::Open(Delimiter::Brack)),
        just(']').to(Token::Close(Delimiter::Brack)),
        just('{').to(Token::Open(Delimiter::Brace)),
        just('}').to(Token::Close(Delimiter::Brace)),
    ));

    let escape = just('\\')
        .ignore_then(just('\\')
        .or(just('/'))
        .or(just('"'))
        .or(just('b').to('\x08'))
        .or(just('f').to('\x0C'))
        .or(just('n').to('\n'))
        .or(just('r').to('\r'))
        .or(just('t').to('\t')));

    let r#char = just('\'')
        .ignore_then(filter(|c| *c != '\\' && *c != '\'').or(escape))
        .then_ignore(just('\''))
        .map(Token::Char)
        .labelled("character");

    let string = just('"')
        .ignore_then(filter(|c| *c != '\\' && *c != '"').or(escape).repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .map(Intern::new)
        .map(Token::Str)
        .labelled("string");

    let intrinsic = just('@')
        .ignore_then(text::ident())
        .map(ast::Ident::new)
        .map(Token::Intrinsic);

    let word = text::ident().map(|s: String| match s.as_str() {
        "import" => Token::Import,
        "data" => Token::Data,
        "type" => Token::Type,
        "def" => Token::Def,
        "class" => Token::Class,
        "member" => Token::Member,
        "for" => Token::For,
        "fn" => Token::Fn,
        "let" => Token::Let,
        "if" => Token::If,
        "match" => Token::Match,
        "then" => Token::Then,
        "else" => Token::Else,
        "in" => Token::In,
        "of" => Token::Of,
        "do" => Token::Do,
        "and" => Token::Op(Op::And),
        "or" => Token::Op(Op::Or),
        "xor" => Token::Op(Op::Xor),
        "True" => Token::Bool(true),
        "False" => Token::Bool(false),
        "_" => Token::Wildcard,
        _ => if s.chars().next().map_or(false, |c| c.is_uppercase()) {
            Token::TypeIdent(ast::Ident::new(s))
        } else {
            Token::TermIdent(ast::Ident::new(s))
        },
    });

    let comments = just('#')
        .then_ignore(just('(')
            .ignore_then(none_of(')').ignored().repeated())
            .then_ignore(just(")#"))
            .or(none_of('\n').ignored().repeated()))
        .padded()
        .ignored()
        .repeated();

    let token = choice((
        ctrl,
        word,
        num,
        nat,
        op,
        delim,
        string,
        r#char,
        intrinsic,
    ))
        .map_with_span(move |token, span| (token, span))
        .padded()
        .recover_with(skip_then_retry_until([]));

    token
        .padded_by(comments)
        .repeated()
        .padded()
        .then_ignore(end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let code = "+ - *+++/++++hello)Hello[}!>>=42and";
        let len = code.chars().count();

        let span = |i| Span::new(SrcId::empty(), i..i + 1);

        assert_eq!(
            lexer()
                .parse(chumsky::Stream::from_iter(
                    span(len),
                    code.chars().enumerate().map(|(i, c)| (c, span(i))),
                ))
                .map(|tokens| tokens.into_iter().map(|(tok, _)| tok).collect::<Vec<_>>()),
            Ok(vec![
                Token::Op(Op::Add),
                Token::Op(Op::Sub),
                Token::Op(Op::Mul),
                Token::Op(Op::Join),
                Token::Op(Op::Add),
                Token::Op(Op::Div),
                Token::Op(Op::Join),
                Token::Op(Op::Join),
                Token::TermIdent(ast::Ident::new("hello")),
                Token::Close(Delimiter::Paren),
                Token::TypeIdent(ast::Ident::new("Hello")),
                Token::Open(Delimiter::Brack),
                Token::Close(Delimiter::Brace),
                Token::Op(Op::Not),
                Token::Op(Op::More),
                Token::Op(Op::MoreEq),
                Token::Nat(42),
                Token::Op(Op::And),
            ]),
        );
    }
}
