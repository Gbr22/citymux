use renterm::vector::Vector2;

use crate::state::StateContainer;

pub async fn update_size(state_container: StateContainer) -> Result<Vector2, anyhow::Error> {
    let (width, height) = crossterm::terminal::size()?;
    let size = state_container.state().size.clone();
    let size = {
        let mut size = size.write().await;
        size.y = height as i32;
        size.x = width as i32;

        size.to_owned()
    };

    Ok(size)
}
