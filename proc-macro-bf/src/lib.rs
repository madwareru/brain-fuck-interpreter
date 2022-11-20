use proc_macro2::{TokenStream};
use combine::{parser, between, many, Parser, token};
use quote::{format_ident, quote};

enum Node {
    Inc,
    Dec,
    IncTapePos,
    DecTapePos,
    PutChar,
    GetChar,
    Loop(Vec<Node>)
}

impl Node {
    fn to_token_stream(&self) -> TokenStream {
        match self {
            Node::Inc => quote!(tape[tape_pos] += 1;),
            Node::Dec => quote!(tape[tape_pos] -= 1;),
            Node::IncTapePos => quote!(tape_pos += 1;),
            Node::DecTapePos => quote!(tape_pos -= 1;),
            Node::PutChar => quote!(print!("{}", tape[tape_pos] as char);),
            Node::GetChar => quote!(tape[tape_pos] = unsafe { libc::getchar() } as u8;),
            Node::Loop(nodes) => {
                let statements: TokenStream = nodes
                    .iter()
                    .map(|node| node.to_token_stream())
                    .collect();

                quote!(
                    while tape[tape_pos as usize] != 0 {
                        #statements
                    }
                )
            }
        }
    }
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

fn parse_bf(bf_string: &str) -> Vec<Node> {
    let mut parser = many(primitive_parser()).map(|nodes: Vec<Node>| nodes);
    parser.parse(bf_string).unwrap().0
}

#[proc_macro]
pub fn bf(items: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut items_iter = items.into_iter();

    let function_name = match items_iter.next() {
        Some(proc_macro::TokenTree::Ident(ident)) => ident,
        _ => panic!("expected identifier")
    };

    match items_iter.next() {
        Some(proc_macro::TokenTree::Punct(punct)) => {
            if punct.as_char() != ',' {
                panic!("expected ,")
            }
        },
        _ => panic!("expected punct")
    };

    let literal = match items_iter.next() {
        Some(proc_macro::TokenTree::Literal(literal)) => {
            literal.to_string()
        },
        _ => panic!("expected literal")
    };

    let literal_trimmed = literal.trim_matches('\"');

    let parsed_ast = parse_bf(literal_trimmed);

    let statements: TokenStream = parsed_ast
        .iter()
        .map(|it| it.to_token_stream())
        .collect();

    let foo_name = format_ident!("{}", function_name.to_string());

    let pm2 = quote!(
        pub fn #foo_name() {
            let mut tape: Vec<u8> = Vec::with_capacity(0x100000);
            tape.resize(0x100000, 0);
            let mut tape_pos = 0;
            #statements
        }
    );

    proc_macro::TokenStream::from(pm2)
}