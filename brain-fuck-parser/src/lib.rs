use combine::{parser, between, many, Parser, token, choice, none_of};

macro_rules! ref_parser {
    ($foo:ident) => { parser(|input| { $foo().parse_stream(input).into_result() }) }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Node {
    Root(Vec<Node>),
    Inc(u8),
    Dec(u8),
    IncTapePos(usize),
    DecTapePos(usize),
    PutChar,
    GetChar,
    Clear,
    AddToNextAndClear,
    Comment,
    Loop(Vec<Node>)
}

fn parse_root<'a>() -> impl Parser<&'a str, Output = Node> {
    many(parse_entry())
        .map(|nodes: Vec<Node>| Node::Root(nodes))
}
fn parse_inc<'a>() -> impl Parser<&'a str, Output = Node> {
    token('+')
        .map(|_| Node::Inc(1))
}
fn parse_dec<'a>() -> impl Parser<&'a str, Output = Node> {
    token('-')
        .map(|_| Node::Dec(1))
}
fn parse_inc_tape_pos<'a>() -> impl Parser<&'a str, Output = Node> {
    token('>')
        .map(|_| Node::IncTapePos(1))
}
fn parse_dec_tape_pos<'a>() -> impl Parser<&'a str, Output = Node> {
    token('<')
        .map(|_| Node::DecTapePos(1))
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
        .map(|_| Node::Comment)
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
    parse_root()
        .parse(bf_string)
        .unwrap().0
        .optimize_series()
        .optimize_loops()
}

impl Node {
    fn optimize_series(&self) -> Self {
        match self {
            Node::Root(nodes) => {
                let mut new_nodes = Vec::with_capacity(nodes.len());
                for node in nodes.iter() {
                    match (node, new_nodes.last_mut()) {
                        (Node::Root(_), _) | (Node::Loop(_), _) => new_nodes.push(node.optimize_series()),
                        (Node::Inc(amount), Some(Node::Inc(a))) => *a += amount,
                        (Node::Dec(amount), Some(Node::Dec(a))) => *a += amount,
                        (Node::IncTapePos(amount), Some(Node::IncTapePos(a))) => *a += amount,
                        (Node::DecTapePos(amount), Some(Node::DecTapePos(a))) => *a += amount,
                        (Node::Comment, _) => {},
                        _  => new_nodes.push(node.clone()),
                    }
                }
                Node::Root(new_nodes)
            },
            Node::Loop(nodes) => {
                let mut new_nodes = Vec::with_capacity(nodes.len());
                for node in nodes.iter() {
                    match (node, new_nodes.last_mut()) {
                        (Node::Root(_), _) | (Node::Loop(_), _) => new_nodes.push(node.optimize_series()),
                        (Node::Inc(amount), Some(Node::Inc(a))) => *a += amount,
                        (Node::Dec(amount), Some(Node::Dec(a))) => *a += amount,
                        (Node::IncTapePos(amount), Some(Node::IncTapePos(a))) => *a += amount,
                        (Node::DecTapePos(amount), Some(Node::DecTapePos(a))) => *a += amount,
                        (Node::Comment, _) => {},
                        _  => new_nodes.push(node.clone()),
                    }
                }
                Node::Loop(new_nodes)
            },
            _ => self.clone()
        }
    }
    fn optimize_loops(&self) -> Self {
        match self {
            Node::Root(nodes) => {
                Node::Root(nodes.iter().map(|it| it.optimize_loops()).collect())
            }
            Node::Loop(nodes) => {
                match &nodes[..] {
                    &[Node::Dec(1)] => Node::Clear,
                    &[Node::IncTapePos(1), Node::Inc(1), Node::DecTapePos(1), Node::Dec(1)]
                        => Node::AddToNextAndClear,
                    &[Node::Dec(1), Node::IncTapePos(1), Node::Inc(1), Node::DecTapePos(1)]
                        => Node::AddToNextAndClear,
                    _ => Node::Loop(nodes.iter().map(|it| it.optimize_loops()).collect())
                }
            }
            _ => self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Node, parse_bf};

    #[test]
    fn ensure_node_clear_converges() {
        let bf = parse_bf("[-]");
        assert_eq!(Node::Root(vec![Node::Clear]), bf);
    }

    #[test]
    fn ensure_comments_work() {
        let bf = parse_bf("clear:[-]");
        assert_eq!(Node::Root(vec![Node::Clear]), bf);
    }

    #[test]
    fn ensure_node_add_to_next_and_clear_converges() {
        let bf = parse_bf("[->+<]");
        assert_eq!(Node::Root(vec![Node::AddToNextAndClear]), bf);

        let bf = parse_bf("[>+<-]");
        assert_eq!(Node::Root(vec![Node::AddToNextAndClear]), bf);
    }

    #[test]
    fn ensure_node_series_converges() {
        let bf = parse_bf("+++++");
        assert_eq!(Node::Root(vec![Node::Inc(5)]), bf);

        let bf = parse_bf("-----");
        assert_eq!(Node::Root(vec![Node::Dec(5)]), bf);

        let bf = parse_bf(">>>>>");
        assert_eq!(Node::Root(vec![Node::IncTapePos(5)]), bf);

        let bf = parse_bf("<<<<<");
        assert_eq!(Node::Root(vec![Node::DecTapePos(5)]), bf);

        let bf = parse_bf("<<<<---+++++-->>");
        assert_eq!(Node::Root(vec![
            Node::DecTapePos(4),
            Node::Dec(3),
            Node::Inc(5),
            Node::Dec(2),
            Node::IncTapePos(2)
        ]), bf);
    }
}