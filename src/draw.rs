use std::time::Duration;

use tokio::io::AsyncWriteExt;

use crate::{canvas::get_cell, escape_codes::{EraseInDisplay, MoveCursor}, StateContainer};

pub async fn draw(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = state_container.get_state().stdout.clone();
    let mut stdout = stdout.lock().await;

    let programs = state_container.get_state().processes.lock().await.clone();
    for program in programs.iter() {
        let process = program.lock().await;
        let canvas = process.canvas.lock().await;
        for y in 0..canvas.size.y {
            stdout.write(&Into::<Vec<u8>>::into(MoveCursor::new(y, 0))).await?;
            for x in 0..canvas.size.x {
                let cell = get_cell(&canvas, x, y);
                stdout.write(cell.value.as_bytes()).await?;
            }
            stdout.write("\r".as_bytes()).await?;
        }
        stdout.write(&Into::<Vec<u8>>::into(MoveCursor::from(canvas.cursor))).await?;
    }

    stdout.flush().await?;

    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

pub async fn draw_loop(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        draw(state_container.clone()).await?;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
