use std::sync::Arc;

use tokio::{io::AsyncWriteExt, sync::Mutex};

use crate::{canvas::{Canvas, Cell, Color, Rect, Style, Vector2}, escape_codes::{CursorForward, EraseCharacter, MoveCursor, ResetStyle, SetCursorVisibility}, span::{self, Node, NodeData, SpanDirection}, Process, StateContainer};

pub async fn find_process_by_id(state_container: StateContainer, id: usize) -> Option<Arc<Mutex<Process>>> {
    let processes = state_container.get_state().processes.lock().await.clone();
    for process in processes {
        let process_inner = process.lock().await;
        if process_inner.span_id == id {
            return Some(process.clone());
        }
    }

    None
}

pub async fn draw_node_content(state_container: StateContainer, node: &Node, process: Arc<Mutex<Process>>, output_canvas: &mut Canvas) -> Result<(), Box<dyn std::error::Error>> {
    let process = process.lock().await;
    let size = output_canvas.size();
    let mut terminal = process.terminal_info.lock().await;
    terminal.set_size(size);
    let canvas = terminal.canvas();
    {
        let mut terminal = process.terminal.lock().await;
        if terminal.size() != size {
            terminal.set_size(size)?;
        }
    }
    output_canvas.put_canvas(&canvas, Vector2::new(0, 0));

    Ok(())
}

pub fn find_span_dimensions(node: &Node, span_id: usize, parent_dimensions: Rect) -> Option<Rect> {
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
            let mut last_size = Vector2::new(0, 0);
            let mut last_position = parent_dimensions.position;
            
            let mut i = 0;
            for child in &span.children {
                let is_last = i == span.children.len() - 1;
                i += 1;
                let size = child.size;
                let ratio = size / total;
                let size = if is_last {
                    match direction {
                        SpanDirection::Horizontal => Vector2::new((parent_dimensions.size.x as f64 * ratio).ceil() as isize, parent_dimensions.size.y),
                        SpanDirection::Vertical => Vector2::new(parent_dimensions.size.x, (parent_dimensions.size.y as f64 * ratio).ceil() as isize),
                    }
                } else {
                    match direction {
                        SpanDirection::Horizontal => Vector2::new((parent_dimensions.size.x as f64 * ratio).round() as isize, parent_dimensions.size.y),
                        SpanDirection::Vertical => Vector2::new(parent_dimensions.size.x, (parent_dimensions.size.y as f64 * ratio).floor() as isize),
                    }
                };
                let position = match direction {
                    SpanDirection::Horizontal => Vector2::new(last_position.x + last_size.x, last_position.y),
                    SpanDirection::Vertical => Vector2::new(last_position.x, last_position.y + last_size.y),
                };
                
                last_size = size;
                last_position = position;

                let sub_dim = find_span_dimensions(&child.node, span_id, Rect {
                    position,
                    size,
                });

                if let Some(sub_dim) = sub_dim {
                    return Some(sub_dim);
                }
            }
        },
        NodeData::Void => {
            return None;
        }
    };

    None
}

pub async fn draw_node(state_container: StateContainer, root: &Node, node: &Node, canvas: &mut Canvas) -> Result<(), Box<dyn std::error::Error>> {
    match node.data {
        NodeData::Span(ref span) => {
            for child in &span.children {
                let child_node = &child.node;

                let future = draw_node(state_container.clone(), root, child_node, canvas);
                Box::pin(future).await?;
            }
        },
        NodeData::Void => {
            let dimensions = find_span_dimensions(root, node.id, Rect {
                position: Vector2::new(0, 0),
                size: canvas.size(),
            });
            let Some(dimensions) = dimensions else {
                return Err("Could not find dimensions of span".into());
            };
            let parent_canvas = canvas;
            let canvas = &mut Canvas::new(dimensions.size);

            let vertical_bar = Cell::new_styled("│", Style::default()
                .with_foreground_color(Color::new_one_byte(8+7)));
            let horizontal_bar = Cell::new_styled("─", vertical_bar.style.clone());
            for y in 0..canvas.size().y {
                let left = Vector2::new(0, y);
                let right = Vector2::new(canvas.size().x-1, y);
                canvas.set_cell(left, vertical_bar.clone());
                canvas.set_cell(right, vertical_bar.clone());
            }
            for x in 0..canvas.size().x {
                let top = Vector2::new(x, 0);
                let bottom = Vector2::new(x, canvas.size().y-1);
                canvas.set_cell(top, horizontal_bar.clone());
                canvas.set_cell(bottom, horizontal_bar.clone());
            }
            let top_left = Cell::new_styled("┌", horizontal_bar.style.clone());
            canvas.set_cell(Vector2::new(0, 0), top_left);
            let top_right = Cell::new_styled("┐", horizontal_bar.style.clone());
            canvas.set_cell(Vector2::new(canvas.size().x-1, 0), top_right);
            let bottom_left = Cell::new_styled("└", horizontal_bar.style.clone());
            canvas.set_cell(Vector2::new(0, canvas.size().y-1), bottom_left);
            let bottom_right = Cell::new_styled("┘", horizontal_bar.style.clone());
            canvas.set_cell(Vector2::new(canvas.size().x-1, canvas.size().y-1), bottom_right);

            let mut proc_canvas = Canvas::new(canvas.size() - Vector2::new(2, 2));
            let process = find_process_by_id(state_container.clone(), node.id).await;
            if let Some(process) = process {
                {
                    let process = process.lock().await;
                    let terminal_info = process.terminal_info.lock().await;
                    let title = format!("[{}]", terminal_info.title());
                    let mut title: Canvas = title.into();
                    title.iter_mut_cells().for_each(|cell| {
                        cell.style = Style::default()
                            .with_background_color(Color::new_rgb(153, 100, 193))
                            .with_foreground_color(Color::new_one_byte(8+7));
                    });
                    title.set_size(Vector2::new(isize::min(title.size().x, canvas.size().x-2), 1));
                    canvas.put_canvas(&title, Vector2::new(
                        1,
                        0
                    ));
                }
                let future = draw_node_content(state_container.clone(), node, process, &mut proc_canvas);
                Box::pin(future).await?;
            }

            parent_canvas.put_canvas(&canvas, dimensions.position);
            parent_canvas.put_canvas(&proc_canvas, dimensions.position+Vector2::new(1, 1));
        }
    };

    Ok(())
}

pub async fn draw(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = state_container.get_state().stdout.clone();
    let mut stdout = stdout.lock().await;

    let size: Vector2 = state_container.get_state().size.read().await.clone();
    let mut new_canvas = Canvas::new_filled(
        size,
        Cell::new_styled(
            "#",Style::default()
            .with_background_color(Color::default())
            .with_foreground_color(Color::new_one_byte(8+7))
        )
    );
    
    {
        let state = state_container.get_state();
        let root = state.root_node.lock().await;
        let root = root.as_ref();
        if let Some(root) = root {
            let mut canvas = Canvas::new_filled(size, Cell::new_styled(" ", Style::default()));
            let future = draw_node(state_container.clone(), &root, &root, &mut canvas);
            Box::pin(future).await?;
            new_canvas.put_canvas(&canvas, Vector2::new(0, 0));    
        }
    }

    let mut to_write: Vec<u8> = Vec::new();
    to_write.extend(Into::<&[u8]>::into(ResetStyle::default()));
    to_write.extend(Into::<&[u8]>::into(SetCursorVisibility::new(false)));
    {
        let state = state_container.get_state();
        let mut last_canvas = state.last_canvas.lock().await;
        let mut last_style = Style::default();
        
        if last_canvas.ne(&new_canvas) {
            for y in 0..new_canvas.size().y {
                to_write.extend(&Into::<Vec<u8>>::into(MoveCursor::new(y, 0)));
                let mut empty_count = 0;
                for x in 0..new_canvas.size().x {
                    let cell = new_canvas.get_cell((x, y));
                    let has_next = x + 1 < new_canvas.size().x;
                    let next = new_canvas.get_cell((x + 1, y));
                    
                    let is_empty_optimization_enabled = false;
                    if cell.is_empty() && is_empty_optimization_enabled {
                        if empty_count == 0 {
                            if cell.style != last_style {
                                to_write.extend(Into::<&[u8]>::into(ResetStyle::default()));
                                to_write.extend(&Into::<Vec<u8>>::into(cell.style.clone()));
                            }
                            last_style = cell.style.clone();
                        }
                        empty_count += 1;
                        if !has_next || !next.is_empty() || next.style != last_style {
                            to_write.extend(&Into::<Vec<u8>>::into(EraseCharacter::new(empty_count)));
                            to_write.extend(&Into::<Vec<u8>>::into(CursorForward::new(empty_count)));
                            empty_count = 0;
                        }
                        continue;
                    }

                    if cell.style != last_style {
                        to_write.extend(Into::<&[u8]>::into(ResetStyle::default()));
                        to_write.extend(&Into::<Vec<u8>>::into(cell.style.clone()));
                        last_style = cell.style.clone();
                    }

                    to_write.extend(&cell.value.to_vec());
                }
                to_write.extend("\r".as_bytes());
            }
            to_write.extend(Into::<&[u8]>::into(ResetStyle::default()));
            *last_canvas = new_canvas;
        }
    }

    let mut cursor_position = Vector2::new(0, 0);
    let active_id = state_container.get_state().active_id.load(std::sync::atomic::Ordering::Relaxed);
    let active_process: Option<Arc<Mutex<Process>>> = find_process_by_id(state_container.clone(), active_id).await;
    if let Some(ref process) = active_process {
        let process = process.lock().await;
        let terminal = process.terminal_info.lock().await;
        cursor_position = terminal.cursor_position();
    }

    {
        if let Some(ref process) = active_process {
            let process = process.lock().await;
            let terminal = process.terminal_info.lock().await;
            if terminal.is_cursor_visible() {
                let state = state_container.get_state();
                let root = state.root_node.lock().await;
                let root = root.as_ref();
                if let Some(root) = root {
                    let span = find_span_dimensions(root, process.span_id, Rect {
                        position: Vector2::new(0, 0),
                        size,
                    });
                    if let Some(span) = span {
                        to_write.extend(&Into::<Vec<u8>>::into(MoveCursor::from(span.position + cursor_position + Vector2::new(1, 1))));
                        to_write.extend(Into::<&[u8]>::into(SetCursorVisibility::new(true)));
                    }
                }
            }
        }
    }
    stdout.write(&to_write).await?;
    stdout.flush().await?;

    Ok(())
}

pub async fn draw_loop(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let (width, height) = crossterm::terminal::size()?;
        let size = state_container.get_state().size.clone();
        {
            let mut size = size.write().await;
            size.y = height as isize;
            size.x = width as isize;
        }
        draw(state_container.clone()).await?;
    }
}
