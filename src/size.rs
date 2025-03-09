use crate::{canvas::Vector2, state::StateContainer};

pub async fn update_size(state_container: StateContainer) -> Result<Vector2, anyhow::Error> {
    let (width, height) = crossterm::terminal::size()?;
    let size = state_container.state().size.clone();
    let size = {
        let mut size = size.write().await;
        size.y = height as isize;
        size.x = width as isize;

        *size
    };

    Ok(size)
}
