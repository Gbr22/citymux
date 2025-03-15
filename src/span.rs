use renterm::{rect::Rect, vector::Vector2};

use crate::StateContainer;

#[derive(Debug, Clone, Copy)]
pub enum SpanDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub enum NodeData {
    Span(Span),
    Void,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub direction: SpanDirection,
    pub children: Vec<SpanChild>,
}

impl Span {
    pub fn new(kind: SpanDirection) -> Self {
        Span {
            direction: kind,
            children: Vec::new(),
        }
    }
}

impl NodeData {
    pub fn new() -> Self {
        NodeData::Void
    }
}

#[derive(Debug, Clone)]
pub struct SpanChild {
    pub size: f64,
    pub node: Node,
}

impl SpanChild {
    pub fn new(child: Node) -> Self {
        SpanChild {
            size: 1.0,
            node: child,
        }
    }
    pub fn with_size(self, size: f64) -> Self {
        SpanChild {
            size,
            node: self.node,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub data: NodeData,
}

fn find_by_id_internal<'a>(
    node: &'a mut Node,
    id: usize,
    path: &mut Vec<usize>,
) -> Option<&'a mut Node> {
    path.push(node.id());
    if node.id == id {
        path.pop();
        return Some(node);
    }
    if let NodeData::Span(span) = &mut node.data {
        for child in &mut span.children {
            let node = &mut child.node;
            if node.id == id {
                return Some(node);
            }
            let node = find_by_id_internal(node, id, path);
            if node.is_some() {
                return node;
            }
        }
    };

    path.pop();

    None
}

impl Node {
    pub fn new(id: usize, data: NodeData) -> Self {
        Self { id, data }
    }
    pub fn id(&self) -> usize {
        self.id
    }
    pub fn find_by_id(&mut self, id: usize) -> Option<(&mut Node, Vec<usize>)> {
        let mut path = Vec::new();
        let result = find_by_id_internal(self, id, &mut path);

        result.map(|node| (node, path))
    }
}

pub async fn get_root_dimensions(state_container: StateContainer) -> Rect {
    let state = state_container.state();
    let size = state.size.read().await;

    Rect::new(Vector2::new(0, 0), *size)
}
