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
    IncTapePosUntilEmpty,
    DecTapePosUntilEmpty,
    PutChar,
    GetChar,
    Clear,
    AddToTheRightAndClear(usize),
    DecFromTheRightAndClear(usize),
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
                        // the loop after loop never runs. Eliminating:
                        (Node::Loop(_), Some(Node::Loop(_))) => {},
                        // eliminate anything suited as a commentary chars
                        (Node::Comment, _) => {},
                        // eliminate empty loops
                        (Node::Loop(loop_nodes), _) if loop_nodes.is_empty() => {},
                        (Node::Loop(loop_nodes), _) if !loop_nodes.is_empty() => {
                            if let Node::Loop(optimized_nodes) = node.optimize_series() {
                                if !optimized_nodes.is_empty() {
                                    new_nodes.push(Node::Loop(optimized_nodes));
                                }
                            }
                        },
                        // join sequential incs, decs, as well as tape position shifts
                        (Node::Inc(amount), Some(Node::Inc(a))) => *a += amount,
                        (Node::Dec(amount), Some(Node::Dec(a))) => *a += amount,
                        (Node::IncTapePos(amount), Some(Node::IncTapePos(a))) => *a += amount,
                        (Node::DecTapePos(amount), Some(Node::DecTapePos(a))) => *a += amount,
                        _  => new_nodes.push(node.clone()),
                    }
                }
                Node::Root(new_nodes)
            },
            Node::Loop(nodes) => {
                let mut new_nodes = Vec::with_capacity(nodes.len());
                for node in nodes.iter() {
                    match (node, new_nodes.last_mut()) {
                        // the loop after loop never runs. Eliminating:
                        (Node::Loop(_), Some(Node::Loop(_))) => {},
                        // eliminate anything suited as a commentary chars
                        (Node::Comment, _) => {},
                        // eliminate empty loops
                        (Node::Loop(loop_nodes), _) if loop_nodes.is_empty() => {},
                        (Node::Loop(loop_nodes), _) if !loop_nodes.is_empty() => {
                            if let Node::Loop(optimized_nodes) = node.optimize_series() {
                                if !optimized_nodes.is_empty() {
                                    new_nodes.push(Node::Loop(optimized_nodes));
                                }
                            }
                        },
                        // join sequential incs, decs, as well as tape position shifts
                        (Node::Inc(amount), Some(Node::Inc(a))) => *a += amount,
                        (Node::Dec(amount), Some(Node::Dec(a))) => *a += amount,
                        (Node::IncTapePos(amount), Some(Node::IncTapePos(a))) => *a += amount,
                        (Node::DecTapePos(amount), Some(Node::DecTapePos(a))) => *a += amount,
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
                    &[Node::IncTapePos(1)] => Node::IncTapePosUntilEmpty,
                    &[Node::DecTapePos(1)] => Node::DecTapePosUntilEmpty,
                    &[Node::IncTapePos(shr), Node::Inc(1), Node::DecTapePos(shl), Node::Dec(1)]
                        if shr == shl => Node::AddToTheRightAndClear(shr),
                    &[Node::Dec(1), Node::IncTapePos(shr), Node::Inc(1), Node::DecTapePos(shl)]
                    if shr == shl => Node::AddToTheRightAndClear(shr),
                    &[Node::IncTapePos(shr), Node::Dec(1), Node::DecTapePos(shl), Node::Dec(1)]
                    if shr == shl => Node::DecFromTheRightAndClear(shr),
                    &[Node::Dec(1), Node::IncTapePos(shr), Node::Dec(1), Node::DecTapePos(shl)]
                    if shr == shl => Node::DecFromTheRightAndClear(shr),
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
    fn ensure_sequential_loops_eliminates() {
        let bf = parse_bf("[.][+]");
        assert_eq!(Node::Root(vec![Node::Loop(vec![Node::PutChar])]), bf);

        let bf = parse_bf("[-][+]");
        assert_eq!(Node::Root(vec![Node::Clear]), bf);
    }

    #[test]
    fn ensure_empty_loops_eliminates() {
        let bf = parse_bf("[]");
        assert_eq!(Node::Root(vec![]), bf);

        let bf = parse_bf("+[]+");
        assert_eq!(Node::Root(vec![Node::Inc(2)]), bf);

        let bf = parse_bf("[+[]+]");
        assert_eq!(Node::Root(vec![Node::Loop(vec![Node::Inc(2)])]), bf);

        let bf = parse_bf("[[]]");
        assert_eq!(Node::Root(vec![]), bf);
    }

    #[test]
    fn ensure_comments_work() {
        let bf = parse_bf("clear:[-]");
        assert_eq!(Node::Root(vec![Node::Clear]), bf);

        let bf = parse_bf(".comment");
        assert_eq!(Node::Root(vec![Node::PutChar]), bf);

        let bf = parse_bf("comment");
        assert_eq!(Node::Root(vec![]), bf);

        let bf = parse_bf("[comment]");
        assert_eq!(Node::Root(vec![]), bf);
    }

    #[test]
    fn ensure_node_add_to_right_and_clear_converges() {
        let bf = parse_bf("[->+<]");
        assert_eq!(Node::Root(vec![Node::AddToTheRightAndClear(1)]), bf);

        let bf = parse_bf("[>+<-]");
        assert_eq!(Node::Root(vec![Node::AddToTheRightAndClear(1)]), bf);
    }

    #[test]
    fn ensure_node_dec_from_right_and_clear_converges() {
        let bf = parse_bf("[->-<]");
        assert_eq!(Node::Root(vec![Node::DecFromTheRightAndClear(1)]), bf);

        let bf = parse_bf("[>-<-]");
        assert_eq!(Node::Root(vec![Node::DecFromTheRightAndClear(1)]), bf);
    }

    #[test]
    fn ensure_simple_cases_work() {
        let bf = parse_bf("++++[,]");
        assert_eq!(Node::Root(vec![
            Node::Inc(4),
            Node::Loop(vec![Node::GetChar])
        ]), bf);

        let bf = parse_bf("++++[.]");
        assert_eq!(Node::Root(vec![
            Node::Inc(4),
            Node::Loop(vec![Node::PutChar])
        ]), bf);
    }

    #[test]
    fn ensure_node_series_converges() {
        let bf = Node::PutChar;
        let bf = bf.optimize_series();
        assert_eq!(Node::PutChar, bf);

        let bf = Node::GetChar;
        let bf = bf.optimize_series();
        assert_eq!(Node::GetChar, bf);

        let bf = Node::Dec(1);
        let bf = bf.optimize_series();
        assert_eq!(Node::Dec(1), bf);

        let bf = Node::Inc(1);
        let bf = bf.optimize_series();
        assert_eq!(Node::Inc(1), bf);

        let bf = Node::IncTapePos(1);
        let bf = bf.optimize_series();
        assert_eq!(Node::IncTapePos(1), bf);

        let bf = Node::IncTapePosUntilEmpty;
        let bf = bf.optimize_series();
        assert_eq!(Node::IncTapePosUntilEmpty, bf);

        let bf = Node::DecTapePos(1);
        let bf = bf.optimize_series();
        assert_eq!(Node::DecTapePos(1), bf);

        let bf = Node::DecTapePosUntilEmpty;
        let bf = bf.optimize_series();
        assert_eq!(Node::DecTapePosUntilEmpty, bf);

        let bf = Node::Clear;
        let bf = bf.optimize_series();
        assert_eq!(Node::Clear, bf);

        let bf = Node::AddToTheRightAndClear(10);
        let bf = bf.optimize_series();
        assert_eq!(Node::AddToTheRightAndClear(10), bf);

        let bf = Node::DecFromTheRightAndClear(10);
        let bf = bf.optimize_series();
        assert_eq!(Node::DecFromTheRightAndClear(10), bf);

        let bf = Node::Root(vec![
            Node::Inc(3), Node::Inc(8)
        ]);
        let bf = bf.optimize_series();
        assert_eq!(Node::Root(vec![Node::Inc(11)]), bf);

        let bf = Node::Root(vec![
            Node::Dec(3), Node::Dec(8)
        ]);
        let bf = bf.optimize_series();
        assert_eq!(Node::Root(vec![Node::Dec(11)]), bf);

        let bf = Node::Root(vec![
            Node::IncTapePos(3), Node::IncTapePos(8)
        ]);
        let bf = bf.optimize_series();
        assert_eq!(Node::Root(vec![Node::IncTapePos(11)]), bf);

        let bf = Node::Root(vec![
            Node::DecTapePos(3), Node::DecTapePos(8)
        ]);
        let bf = bf.optimize_series();
        assert_eq!(Node::Root(vec![Node::DecTapePos(11)]), bf);

        let bf = Node::Root(vec![
            Node::Loop(vec![Node::Inc(3), Node::Inc(8)])
        ]);
        let bf = bf.optimize_series();
        assert_eq!(Node::Root(vec![Node::Loop(vec![Node::Inc(11)])]), bf);

        let bf = Node::Root(vec![
            Node::Loop(vec![Node::Dec(3), Node::Dec(8)])
        ]);
        let bf = bf.optimize_series();
        assert_eq!(Node::Root(vec![Node::Loop(vec![Node::Dec(11)])]), bf);

        let bf = Node::Root(vec![
            Node::Loop(vec![Node::IncTapePos(3), Node::IncTapePos(8)])
        ]);
        let bf = bf.optimize_series();
        assert_eq!(Node::Root(vec![Node::Loop(vec![Node::IncTapePos(11)]) ]), bf);

        let bf = Node::Root(vec![
            Node::Loop(vec![Node::DecTapePos(3), Node::DecTapePos(8)])
        ]);
        let bf = bf.optimize_series();
        assert_eq!(Node::Root(vec![Node::Loop(vec![Node::DecTapePos(11)])]), bf);

        let bf = parse_bf("+++++");
        assert_eq!(Node::Root(vec![Node::Inc(5)]), bf);

        let bf = parse_bf("-----");
        assert_eq!(Node::Root(vec![Node::Dec(5)]), bf);

        let bf = parse_bf(">>>>>");
        assert_eq!(Node::Root(vec![Node::IncTapePos(5)]), bf);

        let bf = parse_bf("[>]");
        assert_eq!(Node::Root(vec![Node::IncTapePosUntilEmpty]), bf);

        let bf = parse_bf("[<]");
        assert_eq!(Node::Root(vec![Node::DecTapePosUntilEmpty]), bf);

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