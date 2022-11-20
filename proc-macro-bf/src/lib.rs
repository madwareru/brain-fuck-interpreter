use proc_macro2::{TokenStream};
use quote::{format_ident, quote};

mod bf_parser;
use bf_parser::{Node, parse_bf};

#[proc_macro]
pub fn bf(items: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut items_iter = items.into_iter();

    let function_name = match items_iter.next() {
        Some(proc_macro::TokenTree::Ident(ident)) => ident,
        _ => panic!("expected identifier")
    };
    let foo_name = format_ident!("{}", function_name.to_string());

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

    let statements = parse_bf(literal.trim_matches('\"')).to_token_stream();

    proc_macro::TokenStream::from(quote!(
        pub fn #foo_name() {
            let mut tape: Vec<u8> = vec![0; 0x100000];
            let mut tape_pos = 0;
            #statements
        }
    ))
}

impl Node {
    fn to_token_stream(&self) -> TokenStream {
        match self {
            Node::Root(nodes) => nodes.iter().map(|node| node.to_token_stream()).collect(),
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