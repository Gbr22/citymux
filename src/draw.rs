use std::time::Duration;

use tokio::io::AsyncWriteExt;

use crate::{canvas::{Canvas, Cell, Vector2}, escape_codes::{EraseInDisplay, MoveCursor, SetCursorVisibility}, StateContainer};

pub async fn draw(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = state_container.get_state().stdout.clone();
    let mut stdout = stdout.lock().await;

    let programs = state_container.get_state().processes.lock().await.clone();
    let size: Vector2 = state_container.get_state().size.read().await.clone();
    let mut new_canvas = Canvas::new_filled(size, Cell::new("#"));
    let mut cursor_position = Vector2::new(0, 0);

    for program in programs.iter() {
        let process = program.lock().await;
        let mut terminal = process.terminal_info.lock().await;
        let title = terminal.title.clone();
        let canvas = &mut terminal.canvas;
        canvas.set_size(size - Vector2::new(5, 5));
        {
            let mut terminal = process.terminal.lock().await;
            if terminal.size() != canvas.size() {
                terminal.set_size(canvas.size())?;
            }
        }
        let offset = Vector2::new(0,1);
        let title = format!("[{}]", title);
        let outline = Canvas::new_filled(canvas.size()+Vector2::new(2, 2), Cell::new("*"));
        new_canvas.put_canvas(&outline, offset - Vector2::new(1, 1));
        let title = title.into();
        new_canvas.put_canvas(&title, Vector2::new(outline.size().x / 2 - title.size().x / 2, 0));
        new_canvas.put_canvas(canvas, offset);
        cursor_position = terminal.cursor + offset;
    }

    let mut to_write: Vec<u8> = Vec::new();
    {
        let state = state_container.get_state();
        let mut last_canvas = state.last_canvas.lock().await;
        if last_canvas.ne(&new_canvas) {
            for y in 0..new_canvas.size().y {
                to_write.extend(&Into::<Vec<u8>>::into(MoveCursor::new(y, 0)));
                for x in 0..new_canvas.size().x {
                    let cell = new_canvas.get_cell(x, y);
                    to_write.extend(cell.value.as_bytes());
                }
                to_write.extend("\r".as_bytes());
            }
            *last_canvas = new_canvas;
        }
    }

    to_write.extend(&Into::<Vec<u8>>::into(MoveCursor::from(cursor_position)));
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
