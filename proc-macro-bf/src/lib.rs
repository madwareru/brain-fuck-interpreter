use std::time::Instant;
use proc_macro2::{TokenStream};
use quote::{format_ident, quote};

use brain_fuck_parser::{Node, parse_bf};

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

    let instant = Instant::now();
    let statements = if literal.starts_with('\"') {
        parse_bf(literal.trim_matches('\"')).to_token_stream()
    } else if literal.starts_with("r#") {
        parse_bf((&literal[1..]).trim_matches('#').trim_matches('\"')).to_token_stream()
    } else {
        panic!("expected string literal");
    };
    let codegen_time = instant.elapsed().as_secs_f32();

    proc_macro::TokenStream::from(quote!(
        pub fn #foo_name() {
            let mut tape: Vec<u8> = vec![0; 0x100000];
            let mut tape_pos = 0;
            #statements
            let codegen_time = #codegen_time;
            println!("code generation time: {} seconds", codegen_time);
        }
    ))
}

trait ToTokenStream {
    fn to_token_stream(&self) -> proc_macro2::TokenStream;
}

impl ToTokenStream for Node {
    fn to_token_stream(&self) -> proc_macro2::TokenStream {
        match self {
            Node::Root(nodes) => nodes.iter().map(|node| node.to_token_stream()).collect(),
            Node::Inc(inc_amount) => quote!(tape[tape_pos] += #inc_amount;),
            Node::Dec(dec_amount) => quote!(tape[tape_pos] -= #dec_amount;),
            Node::IncTapePos(inc_amount) => quote!(tape_pos += #inc_amount;),
            Node::DecTapePos(dec_amount) => quote!(tape_pos -= #dec_amount;),
            Node::IncTapePosUntilEmpty => quote!( while tape[tape_pos] != 0 { tape_pos += 1; }),
            Node::DecTapePosUntilEmpty => quote!( while tape[tape_pos] != 0 { tape_pos -= 1; }),
            Node::PutChar => quote!(print!("{}", tape[tape_pos] as char);),
            Node::GetChar => quote!(tape[tape_pos] = unsafe { libc::getchar() } as u8;),
            Node::Clear => quote!(tape[tape_pos] = 0;),
            Node::Set(amount) => quote!(tape[tape_pos] = #amount;),
            Node::AddToTheRightAndClear(offset) => quote!(
                tape[tape_pos + #offset] += tape[tape_pos];
                tape[tape_pos] = 0;
            ),
            Node::DecFromTheRightAndClear(offset) => quote!(
                tape[tape_pos + #offset] -= tape[tape_pos];
                tape[tape_pos] = 0;
            ),
            Node::Loop(nodes) => {
                let statements: TokenStream = nodes
                    .iter()
                    .map(|node| node.to_token_stream())
                    .collect();

                quote!(
                    while tape[tape_pos] != 0 {
                        #statements
                    }
                )
            },
            Node::Comment => unreachable!(),
        }
    }
}