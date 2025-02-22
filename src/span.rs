pub enum SpanDirection {
    Horizontal,
    Vertical,
}

pub enum NodeData {
    Span(Span),
    Void,
}

pub struct Span {
    direction: SpanDirection,
    children: Vec<SpanChild>,
}

impl Span {
    pub fn new(kind: SpanDirection) -> Self {
        Span { direction: kind, children: Vec::new() }
    }
}

impl NodeData {
    pub fn new() -> Self {
        NodeData::Void
    }
}

pub struct SpanChild {
    size: f64,
    child: Node,
}

pub struct Node {
    id: usize,
    data: NodeData,
}

impl Node {
    pub fn new(id: usize, data: NodeData) -> Self {
        Self { id, data }
    }
}