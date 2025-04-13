use renterm::{rect::Rect, vector::Vector2};

use crate::span::{Node, NodeData, SpanDirection};

pub fn get_span_dimensions(
    node: &Node,
    span_id: usize,
    parent_dimensions: impl Into<Rect>,
) -> Option<Rect> {
    let parent_dimensions = parent_dimensions.into();
    if node.id == span_id {
        return Some(parent_dimensions);
    }
    match node.data {
        NodeData::Span(ref span) => {
            let direction = span.direction;
            let mut total = 0.0;
            for child in &span.children {
                total += child.size;
            }

            let mut sizes = vec![Vector2::null(); span.children.len()];
            let mut remaining_size = parent_dimensions.size();
            for (index, child) in span.children.iter().enumerate() {
                let size = child.size;
                let ratio = size / total;
                let size = match direction {
                    SpanDirection::Horizontal => Vector2::new(
                        (parent_dimensions.size().x as f64 * ratio).floor() as i32,
                        parent_dimensions.size().y,
                    ),
                    SpanDirection::Vertical => Vector2::new(
                        parent_dimensions.size().x,
                        (parent_dimensions.size().y as f64 * ratio).floor() as i32,
                    ),
                };
                sizes[index] = size.clone();
                remaining_size = remaining_size - size;
            }
            match direction {
                SpanDirection::Horizontal => {
                    while remaining_size.x > 0 {
                        let smallest = sizes.iter_mut().enumerate().min_by_key(|(_, size)| size.x);
                        let Some(smallest) = smallest else {
                            break;
                        };
                        let smallest = smallest.0;

                        sizes[smallest].x += 1;
                        remaining_size.x -= 1;
                    }
                }
                SpanDirection::Vertical => {
                    while remaining_size.y > 0 {
                        let smallest = sizes.iter_mut().enumerate().min_by_key(|(_, size)| size.y);
                        let Some(smallest) = smallest else {
                            break;
                        };
                        let smallest = smallest.0;

                        sizes[smallest].y += 1;
                        remaining_size.y -= 1;
                    }
                }
            }

            let mut last_size = Vector2::new(0, 0);
            let mut last_position = parent_dimensions.position();
            for (index, child) in span.children.iter().enumerate() {
                let size = &sizes[index];
                let position = match direction {
                    SpanDirection::Horizontal => {
                        Vector2::new(last_position.x + last_size.x, last_position.y)
                    }
                    SpanDirection::Vertical => {
                        Vector2::new(last_position.x, last_position.y + last_size.y)
                    }
                };

                last_size = size.clone();
                last_position = position.clone();

                let sub_dim =
                    get_span_dimensions(&child.node, span_id, Rect::new(position, size.to_owned()));

                if let Some(sub_dim) = sub_dim {
                    return Some(sub_dim);
                }
            }
        }
        NodeData::Void => {
            return None;
        }
    };

    None
}
