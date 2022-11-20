use combine::{parser, between, many, Parser, token};

pub(crate) enum Node {
    Inc,
    Dec,
    IncTapePos,
    DecTapePos,
    PutChar,
    GetChar,
    Loop(Vec<Node>)
}

fn parse_inc<'a>() -> impl Parser<&'a str, Output = Node> {
    token('+').map(|_| Node::Inc)
}

fn parse_dec<'a>() -> impl Parser<&'a str, Output = Node> {
    token('-').map(|_| Node::Dec)
}

fn parse_inc_tape_pos<'a>() -> impl Parser<&'a str, Output = Node> {
    token('>').map(|_| Node::IncTapePos)
}

fn parse_dec_tape_pos<'a>() -> impl Parser<&'a str, Output = Node> {
    token('<').map(|_| Node::DecTapePos)
}

fn parse_put_char<'a>() -> impl Parser<&'a str, Output = Node> {
    token('.').map(|_| Node::PutChar)
}

fn parse_get_char<'a>() -> impl Parser<&'a str, Output = Node> {
    token(',').map(|_| Node::GetChar)
}

macro_rules! ref_parser {
    ($parser_fn:ident) => {
        parser(|input| {
            let _: &mut &str = input;
            $parser_fn().parse_stream(input).into_result()
        })
    }
}

fn primitive_parser<'a>() -> impl Parser<&'a str, Output = Node> {
    parse_inc()
        .or(parse_dec())
        .or(parse_inc_tape_pos())
        .or(parse_dec_tape_pos())
        .or(parse_get_char())
        .or(parse_put_char())
        .or(ref_parser!(parse_loop))
}

fn parse_loop<'a>() -> impl Parser<&'a str, Output = Node> {
    between(token('['), token(']'), many(primitive_parser()))
        .map(|nodes: Vec<Node>| Node::Loop(nodes))
}

pub(crate) fn parse_bf(bf_string: &str) -> Vec<Node> {
    let mut parser = many(primitive_parser()).map(|nodes: Vec<Node>| nodes);
    parser.parse(bf_string).unwrap().0
}