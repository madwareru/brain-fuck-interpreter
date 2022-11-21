use combine::{parser, between, many, Parser, token, choice, none_of};

macro_rules! ref_parser {
    ($foo:ident) => { parser(|input| { $foo().parse_stream(input).into_result() }) }
}

pub enum Node {
    Root(Vec<Node>),
    Inc,
    Dec,
    IncTapePos,
    DecTapePos,
    PutChar,
    GetChar,
    Garbage,
    Loop(Vec<Node>)
}

fn parse_root<'a>() -> impl Parser<&'a str, Output = Node> {
    many(parse_entry())
        .map(|nodes: Vec<Node>| Node::Root(nodes))
}
fn parse_inc<'a>() -> impl Parser<&'a str, Output = Node> {
    token('+')
        .map(|_| Node::Inc)
}
fn parse_dec<'a>() -> impl Parser<&'a str, Output = Node> {
    token('-')
        .map(|_| Node::Dec)
}
fn parse_inc_tape_pos<'a>() -> impl Parser<&'a str, Output = Node> {
    token('>')
        .map(|_| Node::IncTapePos)
}
fn parse_dec_tape_pos<'a>() -> impl Parser<&'a str, Output = Node> {
    token('<')
        .map(|_| Node::DecTapePos)
}
fn parse_put_char<'a>() -> impl Parser<&'a str, Output = Node> {
    token('.')
        .map(|_| Node::PutChar)
}
fn parse_get_char<'a>() -> impl Parser<&'a str, Output = Node> {
    token(',')
        .map(|_| Node::GetChar)
}

fn parse_garbage<'a>() -> impl Parser<&'a str, Output = Node> {
    none_of("+-><.,[]".chars())
        .map(|_| Node::Garbage)
}

fn parse_entry<'a>() -> impl Parser<&'a str, Output = Node> {
    choice!(
        parse_inc(),
        parse_dec(),
        parse_inc_tape_pos(),
        parse_dec_tape_pos(),
        parse_get_char(),
        parse_put_char(),
        parse_garbage(),
        ref_parser!(parse_loop)
    )
}

fn parse_loop<'a>() -> impl Parser<&'a str, Output = Node> {
    between(
        token('['),
        token(']'),
        many(parse_entry())
    ).map(|nodes: Vec<Node>| Node::Loop(nodes))
}

pub fn parse_bf(bf_string: &str) -> Node {
    parse_root().parse(bf_string).unwrap().0
}