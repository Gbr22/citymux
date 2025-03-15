use crate::canvas::{Canvas, Surface, Cell, Rect, Vector2};

#[test]
fn canvas_view_read() {
    let mut canvas = Canvas::new_filled(Vector2::new(100, 100), Cell::new('#'));
    let view = canvas.to_sub_view(Rect::new(Vector2::new(10, 10), Vector2::new(50, 50)));
    assert_eq!(view.get_cell(Vector2::new(10, 10)), Cell::new('#'));
    assert_eq!(view.get_cell(Vector2::new(0, 0)), Cell::new('#'));
    assert_eq!(view.get_cell(Vector2::new(-1, -1)), Cell::new(' '));
    assert_eq!(view.get_cell(Vector2::new(50, 50)), Cell::new(' '));
    assert_eq!(view.get_cell(Vector2::new(51, 51)), Cell::new(' '));
}


#[test]
fn canvas_view_write() {
    let mut canvas = Canvas::new(Vector2::new(100, 100));
    let mut view = canvas.to_sub_view(Rect::new(Vector2::new(10, 10), Vector2::new(50, 50)));
    view.set_cell(Vector2::new(0, 0), Cell::new('A'));
    println!("{:?}", &canvas);
    assert_eq!(canvas.get_cell(Vector2::new(10, 10)), Cell::new('A'));
}
