use std::sync::Arc;

use tokio::{io::AsyncWriteExt, sync::Mutex, time::MissedTickBehavior};

use crate::{
    canvas::{Canvas, CanvasLike, CanvasView, Cell, Color, DrawableStr, Rect, Style, Vector2},
    escape_codes::{CursorForward, EraseCharacter, MoveCursor, ResetStyle, SetCursorVisibility},
    layout::get_span_dimensions,
    size::update_size,
    span::{Node, NodeData},
    state::{Process, StateContainer},
};

pub async fn find_process_by_id(
    state_container: StateContainer,
    id: usize,
) -> Option<Arc<Mutex<Process>>> {
    let processes = state_container.state().processes.lock().await.clone();
    for process in processes {
        let process_inner = process.lock().await;
        if process_inner.span_id == id {
            return Some(process.clone());
        }
    }

    None
}

pub async fn draw_node_content(
    state_container: StateContainer,
    node: &Node,
    process: Arc<Mutex<Process>>,
    output_canvas: &mut impl CanvasLike,
) -> anyhow::Result<()> {
    let process = process.lock().await;
    let size = output_canvas.size();
    let mut terminal = process.terminal_info.lock().await;
    terminal.set_size(size);
    {
        let mut terminal = process.terminal.lock().await;
        if terminal.size() != size {
            terminal.set_size(size)?;
        }
    }
    terminal.draw(output_canvas);

    Ok(())
}

pub async fn draw_node(
    state_container: StateContainer,
    root: &Node,
    node: &Node,
    canvas: &mut impl CanvasLike,
) -> anyhow::Result<()> {
    match node.data {
        NodeData::Span(ref span) => {
            for child in &span.children {
                let child_node = &child.node;

                let future = draw_node(state_container.clone(), root, child_node, canvas);
                Box::pin(future).await?;
            }
        }
        NodeData::Void => {
            let dimensions = get_span_dimensions(
                root,
                node.id,
                Rect {
                    position: Vector2::new(0, 0),
                    size: canvas.size(),
                },
            );
            let Some(dimensions) = dimensions else {
                return Err(anyhow::format_err!("Could not find dimensions of span"));
            };
            let parent_canvas = canvas;
            let mut canvas = parent_canvas.to_sub_view(dimensions);

            let is_active = state_container
                .state()
                .active_id
                .load(std::sync::atomic::Ordering::Relaxed)
                == node.id;
            let highlight_color = Color::new_one_byte(8 + 6);
            let inactive_border_style =
                Style::default().with_foreground_color(Color::new_one_byte(8));
            let active_border_style =
                Style::default().with_foreground_color(highlight_color.clone());
            let border_style = if is_active {
                active_border_style
            } else {
                inactive_border_style
            };
            let vertical_bar = Cell::new_styled("│", border_style.clone());
            let horizontal_bar = Cell::new_styled("─", border_style.clone());
            for y in 0..canvas.size().y {
                let left = Vector2::new(0, y);
                let right = Vector2::new(canvas.size().x - 1, y);
                canvas.set_cell(left, vertical_bar.clone());
                canvas.set_cell(right, vertical_bar.clone());
            }
            for x in 0..canvas.size().x {
                let top = Vector2::new(x, 0);
                let bottom = Vector2::new(x, canvas.size().y - 1);
                canvas.set_cell(top, horizontal_bar.clone());
                canvas.set_cell(bottom, horizontal_bar.clone());
            }
            let top_left = Cell::new_styled("┌", border_style.clone());
            canvas.set_cell(Vector2::new(0, 0), top_left);
            let top_right = Cell::new_styled("┐", border_style.clone());
            canvas.set_cell(Vector2::new(canvas.size().x - 1, 0), top_right);
            let bottom_left = Cell::new_styled("└", border_style.clone());
            canvas.set_cell(Vector2::new(0, canvas.size().y - 1), bottom_left);
            let bottom_right = Cell::new_styled("┘", border_style.clone());
            canvas.set_cell(
                Vector2::new(canvas.size().x - 1, canvas.size().y - 1),
                bottom_right,
            );

            let process = find_process_by_id(state_container.clone(), node.id).await;
            if let Some(process) = process {
                {
                    let process = process.lock().await;
                    let terminal_info = process.terminal_info.lock().await;
                    let title = format!("[{}]", terminal_info.title());
                    let title = DrawableStr::new(&title, Style::default()
                    .with_background_color(highlight_color.clone())
                    .with_foreground_color(Color::new_one_byte(0)));
                    canvas.draw_in(Box::new(&title), Rect::new(Vector2::new(1, 0), Vector2::new(canvas.size().x-2, 1)));
                }
                let mut proc_canvas = canvas.to_sub_view(Rect::new(Vector2::new(1, 1), canvas.size() - Vector2::new(2, 2)));
                let future =
                    draw_node_content(state_container.clone(), node, process, &mut proc_canvas);
                Box::pin(future).await?;
            }
        }
    };

    Ok(())
}

async fn draw_inner(state_container: StateContainer) -> anyhow::Result<()> {
    let stdout = state_container.state().stdout.clone();
    let mut stdout = stdout.lock().await;

    let state = state_container.state();

    let size: Vector2 = *state.size.read().await;
    let last_canvas = state.get_last_canvas();
    let last_canvas = last_canvas.lock().await;
    let new_canvas = state.get_current_canvas();
    let mut new_canvas = new_canvas.lock().await;
    new_canvas.set_size(size);

    {
        let state = state_container.state();
        let root = state.root_node.lock().await;
        let root = root.as_ref();
        if let Some(root) = root {
            let mut view = new_canvas.to_view();
            let future = draw_node(state_container.clone(), root, root, &mut view);
            Box::pin(future).await?;
        }
    }

    let mut to_write: Vec<u8> = Vec::new();
    to_write.extend(Into::<&[u8]>::into(ResetStyle::default()));
    to_write.extend(Into::<&[u8]>::into(SetCursorVisibility::new(false)));
    {
        let mut last_style = Style::default();

        if last_canvas.ne(&new_canvas) {
            for y in 0..new_canvas.size().y {
                to_write.extend(&Into::<Vec<u8>>::into(MoveCursor::new(y, 0)));
                let mut empty_count = 0;
                for x in 0..new_canvas.size().x {
                    let cell = new_canvas.get_cell((x, y).into());
                    let has_next = x + 1 < new_canvas.size().x;
                    let next = new_canvas.get_cell((x + 1, y).into());

                    let is_empty_optimization_enabled = true;
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
                            to_write
                                .extend(&Into::<Vec<u8>>::into(EraseCharacter::new(empty_count)));
                            to_write
                                .extend(&Into::<Vec<u8>>::into(CursorForward::new(empty_count)));
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
            state.swap_canvas();
        }
    }

    let mut cursor_position = Vector2::new(0, 0);
    let active_id = state_container
        .state()
        .active_id
        .load(std::sync::atomic::Ordering::Relaxed);
    let active_process: Option<Arc<Mutex<Process>>> =
        find_process_by_id(state_container.clone(), active_id).await;
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
                let state = state_container.state();
                let root = state.root_node.lock().await;
                let root = root.as_ref();
                if let Some(root) = root {
                    let span = get_span_dimensions(
                        root,
                        process.span_id,
                        Rect {
                            position: Vector2::new(0, 0),
                            size,
                        },
                    );
                    if let Some(span) = span {
                        to_write.extend(&Into::<Vec<u8>>::into(MoveCursor::from(
                            span.position + cursor_position + Vector2::new(1, 1),
                        )));
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

pub async fn draw(state_container: StateContainer) -> anyhow::Result<()> {
    let _ = state_container.state().draw_lock.lock().await;
    draw_inner(state_container).await
}

#[derive(Default)]
pub struct DrawMessage {
    _private: (),
}

pub async fn trigger_draw(state_container: StateContainer) {
    let state = state_container.state();
    let draw_channel = { state.draw_channel.lock().await.clone() };
    let Some(ref draw_channel) = draw_channel else {
        tracing::warn!("No draw channel");
        return;
    };
    let _ = draw_channel.send(DrawMessage::default()).await;
}

async fn channel_draw_loop(state_container: StateContainer) -> anyhow::Result<()> {
    let mut rx: tokio::sync::mpsc::Receiver<DrawMessage> = {
        let state = state_container.state();
        let mut draw_channel = state.draw_channel.lock().await;
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        *draw_channel = Some(tx);

        rx
    };

    update_size(state_container.clone()).await?;
    draw(state_container.clone()).await?;
    loop {
        rx.recv().await;
        {
            if state_container.state().draw_lock.try_lock().is_err() {
                continue;
            }
        }
        update_size(state_container.clone()).await?;
        draw(state_container.clone()).await?;
    }
}

pub async fn timeout_draw_loop(state_container: StateContainer) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(16));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        update_size(state_container.clone()).await?;
        draw(state_container.clone()).await?;
    }
}

pub async fn draw_loop(state_container: StateContainer) -> anyhow::Result<()> {
    let results = tokio::join!(
        channel_draw_loop(state_container.clone()),
        timeout_draw_loop(state_container)
    );

    results.0?;
    results.1?;

    Ok(())
}
