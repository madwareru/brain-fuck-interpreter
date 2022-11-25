use std::collections::VecDeque;
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
    AddToTheLeftAndClear(usize),
    DecFromTheLeftAndClear(usize),
    Comment,
    Loop(Vec<Node>)
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SimOperation {
    Noop,
    Inc(u8),
    Dec(u8),
    IncTapePos(u32),
    DecTapePos(u32),
    IncTapePosUntilEmpty,
    DecTapePosUntilEmpty,
    PutChar,
    GetChar,
    Clear,
    AddToTheRightAndClear(u32),
    DecFromTheRightAndClear(u32),
    AddToTheLeftAndClear(u32),
    DecFromTheLeftAndClear(u32),
    JnzSaveIP { target_ip: u32 },
    JnzRestoreIP { target_ip: u32 },
    EndProgram
}

#[derive(Clone, PartialEq, Debug)]
pub enum NumberedNode {
    Root( Vec<NumberedNode> ),
    Loop{ id: usize, operations: Vec<NumberedNode> },
    Operation { id: usize, data: SimOperation }
}

impl NumberedNode {
    fn get_id(&self) -> usize {
        match self {
            NumberedNode::Loop { id, .. } => *id,
            NumberedNode::Operation { id, .. } => *id,
            _ => unreachable!()
        }
    }

    fn from(source: &Node) -> Self {
        match source {
            Node::Root(nodes) => {
                let mut nodes: Vec<NumberedNode> = nodes
                    .iter()
                    .map(|it| NumberedNode::from(it))
                    .collect();
                nodes.push(NumberedNode::Operation { id: 0, data: SimOperation::EndProgram });
                Self::Root(nodes)
            }
            Node::Inc(amount) => {
                Self::Operation { id: 0, data: SimOperation::Inc(*amount) }
            }
            Node::Dec(amount) => {
                Self::Operation { id: 0, data: SimOperation::Dec(*amount) }
            }
            Node::IncTapePos(offset) => {
                Self::Operation { id: 0, data: SimOperation::IncTapePos(*offset as u32) }
            }
            Node::DecTapePos(offset) => {
                Self::Operation { id: 0, data: SimOperation::DecTapePos(*offset as u32) }
            }
            Node::IncTapePosUntilEmpty => {
                Self::Operation { id: 0, data: SimOperation::IncTapePosUntilEmpty }
            }
            Node::DecTapePosUntilEmpty => {
                Self::Operation { id: 0, data: SimOperation::DecTapePosUntilEmpty }
            }
            Node::PutChar => {
                Self::Operation { id: 0, data: SimOperation::PutChar }
            }
            Node::GetChar => {
                Self::Operation { id: 0, data: SimOperation::GetChar }
            }
            Node::Clear => {
                Self::Operation { id: 0, data: SimOperation::Clear }
            }
            Node::AddToTheRightAndClear(offset) => {
                Self::Operation { id: 0, data: SimOperation::AddToTheRightAndClear(*offset as u32) }
            }
            Node::DecFromTheRightAndClear(offset) => {
                Self::Operation { id: 0, data: SimOperation::DecFromTheRightAndClear(*offset as u32) }
            }
            Node::AddToTheLeftAndClear(offset) => {
                Self::Operation { id: 0, data: SimOperation::AddToTheLeftAndClear(*offset as u32) }
            }
            Node::DecFromTheLeftAndClear(offset) => {
                Self::Operation { id: 0, data: SimOperation::DecFromTheLeftAndClear(*offset as u32) }
            }
            Node::Loop(nodes) => {
                let mut operations = nodes
                    .iter()
                    .map(|it| NumberedNode::from(it))
                    .collect::<Vec<_>>();
                operations.push( NumberedNode::Operation {
                    id: 0,
                    data: SimOperation::Noop
                });
                Self::Loop { id: 0, operations }
            }
            _ => unreachable!()
        }
    }

    pub fn numerize(root_node: &mut Self) -> usize {
        let mut id_sequence = 0;
        let mut queue = VecDeque::new();
        if let Self::Root(nodes) = root_node {
            for child_node in nodes.iter_mut() {
                queue.push_back(child_node);
            }
        }
        while let Some(next_node) = queue.pop_front() {
            match next_node {
                NumberedNode::Loop { id, operations } => {
                    *id = id_sequence;
                    id_sequence += 1;
                    for op_node in operations.iter_mut() {
                        queue.push_back(op_node);
                    }
                }
                NumberedNode::Operation { id, .. } => {
                    *id = id_sequence;
                    id_sequence += 1;
                }
                _ => unreachable!()
            }
        }
        id_sequence
    }

    fn linearize(root_node: &mut Self, capacity: usize) -> Vec<SimOperation> {
        let mut result = vec![SimOperation::Noop; capacity];
        let mut queue = VecDeque::new();
        if let Self::Root(nodes) = root_node {
            for child_node in nodes.iter_mut() {
                queue.push_back(child_node);
            }
        }
        while let Some(next_node) = queue.pop_front() {
            match next_node {
                NumberedNode::Loop { operations, id } => {
                    let start_id = operations.first().unwrap().get_id();
                    result[*id] = SimOperation::JnzSaveIP { target_ip: start_id as u32 };
                    let len = operations.len();
                    let mut i = 0;
                    for node in operations.iter_mut() {
                        if i == len-1 {
                            match node {
                                NumberedNode::Operation { data, ..} => {
                                    *data = SimOperation::JnzRestoreIP { target_ip: start_id as u32 };
                                }
                                _ => unreachable!()
                            }
                        }
                        queue.push_back(node);
                        i += 1;
                    }
                }
                NumberedNode::Operation { data, id } => {
                    result[*id] = *data;
                }
                _ => unreachable!()
            }
        }
        result
    }
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
                    &[Node::Inc(1)] => Node::Clear, // Eventually it will overflow to zero
                    &[Node::IncTapePos(1)] => Node::IncTapePosUntilEmpty,
                    &[Node::DecTapePos(1)] => Node::DecTapePosUntilEmpty,

                    &[Node::IncTapePos(shr),
                    Node::Inc(1),
                    Node::DecTapePos(shl),
                    Node::Dec(1)
                    ] if shr == shl => {
                        Node::AddToTheRightAndClear(shr)
                    },
                    &[Node::Dec(1),
                    Node::IncTapePos(shr),
                    Node::Inc(1),
                    Node::DecTapePos(shl)
                    ] if shr == shl => {
                        Node::AddToTheRightAndClear(shr)
                    },

                    &[Node::IncTapePos(shr),
                      Node::Dec(1),
                      Node::DecTapePos(shl),
                      Node::Dec(1)
                    ] if shr == shl => {
                        Node::DecFromTheRightAndClear(shr)
                    },
                    &[Node::Dec(1),
                    Node::IncTapePos(shr),
                    Node::Dec(1),
                    Node::DecTapePos(shl)
                    ] if shr == shl => {
                        Node::DecFromTheRightAndClear(shr)
                    },

                    &[Node::DecTapePos(shl),
                    Node::Inc(1),
                    Node::IncTapePos(shr),
                    Node::Dec(1)
                    ] if shr == shl => {
                        Node::AddToTheLeftAndClear(shl)
                    },
                    &[Node::Dec(1),
                    Node::DecTapePos(shl),
                    Node::Inc(1),
                    Node::IncTapePos(shr)
                    ] if shr == shl => {
                        Node::AddToTheLeftAndClear(shl)
                    },

                    &[Node::DecTapePos(shl),
                    Node::Dec(1),
                    Node::IncTapePos(shr),
                    Node::Dec(1)
                    ] if shr == shl => {
                        Node::DecFromTheLeftAndClear(shl)
                    },
                    &[Node::Dec(1),
                    Node::DecTapePos(shl),
                    Node::Dec(1),
                    Node::IncTapePos(shr)
                    ] if shr == shl => {
                        Node::DecFromTheLeftAndClear(shl)
                    },

                    _ => Node::Loop(nodes.iter().map(|it| it.optimize_loops()).collect())
                }
            }
            _ => self.clone()
        }
    }

    pub fn compile_bytecode(&self) -> Vec<SimOperation> {
        let mut new_tree = NumberedNode::from(self);
        let capacity = NumberedNode::numerize(&mut new_tree);
        NumberedNode::linearize(&mut new_tree, capacity)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Node, NumberedNode, parse_bf, SimOperation};

    #[test]
    fn numerization_test() {
        let bf = parse_bf("-+<>[-]+[>]-[<].,[+]");
        let mut numerized_zeroed = NumberedNode::from(&bf);
        assert_eq!(
            &NumberedNode::Root(vec![
                NumberedNode::Operation { id: 0, data: SimOperation::Dec(1) },
                NumberedNode::Operation { id: 0, data: SimOperation::Inc(1) },
                NumberedNode::Operation { id: 0, data: SimOperation::DecTapePos(1) },
                NumberedNode::Operation { id: 0, data: SimOperation::IncTapePos(1) },
                NumberedNode::Operation { id: 0, data: SimOperation::Clear },
                NumberedNode::Operation { id: 0, data: SimOperation::Inc(1) },
                NumberedNode::Operation { id: 0, data: SimOperation::IncTapePosUntilEmpty },
                NumberedNode::Operation { id: 0, data: SimOperation::Dec(1) },
                NumberedNode::Operation { id: 0, data: SimOperation::DecTapePosUntilEmpty },
                NumberedNode::Operation { id: 0, data: SimOperation::PutChar },
                NumberedNode::Operation { id: 0, data: SimOperation::GetChar },
                NumberedNode::Operation { id: 0, data: SimOperation::Clear },
                NumberedNode::Operation { id: 0, data: SimOperation::EndProgram },
            ]),
            &numerized_zeroed
        );

        NumberedNode::numerize(&mut numerized_zeroed);
        let numerized = numerized_zeroed;
        assert_eq!(
            NumberedNode::Root(vec![
                NumberedNode::Operation { id: 0, data: SimOperation::Dec(1) },
                NumberedNode::Operation { id: 1, data: SimOperation::Inc(1) },
                NumberedNode::Operation { id: 2, data: SimOperation::DecTapePos(1) },
                NumberedNode::Operation { id: 3, data: SimOperation::IncTapePos(1) },
                NumberedNode::Operation { id: 4, data: SimOperation::Clear },
                NumberedNode::Operation { id: 5, data: SimOperation::Inc(1) },
                NumberedNode::Operation { id: 6, data: SimOperation::IncTapePosUntilEmpty },
                NumberedNode::Operation { id: 7, data: SimOperation::Dec(1) },
                NumberedNode::Operation { id: 8, data: SimOperation::DecTapePosUntilEmpty },
                NumberedNode::Operation { id: 9, data: SimOperation::PutChar },
                NumberedNode::Operation { id: 10, data: SimOperation::GetChar },
                NumberedNode::Operation { id: 11, data: SimOperation::Clear },
                NumberedNode::Operation { id: 12, data: SimOperation::EndProgram },
            ]),
            numerized
        );
    }

    #[test]
    fn linearization_test() {
        let bf = parse_bf("++[->>]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Inc(2),
                SimOperation::JnzSaveIP { target_ip: 3 },
                SimOperation::EndProgram,
                SimOperation::Dec(1),
                SimOperation::IncTapePos(2),
                SimOperation::JnzRestoreIP { target_ip: 3 }
            ],
            linearized
        );

        let bf = parse_bf("-+<>[-][>][<][+]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::Clear,
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[->+<]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::AddToTheRightAndClear(1),
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[>+<-]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::AddToTheRightAndClear(1),
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[->-<]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::DecFromTheRightAndClear(1),
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[>-<-]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::DecFromTheRightAndClear(1),
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[<+>-]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::AddToTheLeftAndClear(1),
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[-<+>]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::AddToTheLeftAndClear(1),
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[-<->]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::DecFromTheLeftAndClear(1),
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[<->-]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::DecFromTheLeftAndClear(1),
                SimOperation::EndProgram
            ],
            linearized
        );

        let bf = parse_bf("-+<>[-]+[>]-[<].,[+]");
        let linearized = bf.compile_bytecode();
        assert_eq!(
            vec![
                SimOperation::Dec(1),
                SimOperation::Inc(1),
                SimOperation::DecTapePos(1),
                SimOperation::IncTapePos(1),
                SimOperation::Clear,
                SimOperation::Inc(1),
                SimOperation::IncTapePosUntilEmpty,
                SimOperation::Dec(1),
                SimOperation::DecTapePosUntilEmpty,
                SimOperation::PutChar,
                SimOperation::GetChar,
                SimOperation::Clear,
                SimOperation::EndProgram
            ],
            linearized
        );
    }

    #[test]
    fn ensure_node_clear_converges() {
        let bf = parse_bf("[-]");
        assert_eq!(Node::Root(vec![Node::Clear]), bf);

        let bf = parse_bf("[+]");
        assert_eq!(Node::Root(vec![Node::Clear]), bf);
    }

    #[test]
    fn ensure_sequential_loops_eliminates() {
        let bf = parse_bf("[.][+]");
        assert_eq!(Node::Root(vec![Node::Loop(vec![Node::PutChar])]), bf);

        let bf = parse_bf("[-][+]");
        assert_eq!(Node::Root(vec![Node::Clear]), bf);

        let bf = parse_bf("[+][-]");
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