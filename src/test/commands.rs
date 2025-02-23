use crate::{canvas::{Canvas, Cell, Color, Style, TerminalCommand, TerminalInfo, Vector2}, encoding::{CsiSequence, OscSequence}};

#[test]
fn erase_character_simple() {
    let mut info = TerminalInfo::new(Vector2::new(8, 8));
    info.execute_command(TerminalCommand::string("A"));
    info.execute_command(TerminalCommand::string("B"));
    info.execute_command(TerminalCommand::string("C"));
    info.execute_command(TerminalCommand::csi("1G"));
    info.execute_command(TerminalCommand::csi("2X"));
    assert_eq!(info.cursor, Vector2::new(0, 0));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    expected_canvas.set_cell(Vector2::new(2, 0), Cell::new('C'));
    assert_eq!(info.canvas, expected_canvas);
}

#[test]
fn erase_character_beyond_edge_of_screen() {
    let mut info = TerminalInfo::new(Vector2::new(8, 8));
    info.execute_command(TerminalCommand::csi(format!("{}G",info.canvas.size().x)));
    assert_eq!(info.cursor, Vector2::new(7, 0));
    info.execute_command(TerminalCommand::csi("2D"));
    assert_eq!(info.cursor, Vector2::new(5, 0));
    info.execute_command(TerminalCommand::string("A"));
    info.execute_command(TerminalCommand::string("B"));
    info.execute_command(TerminalCommand::string("C"));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    expected_canvas.set_cell(Vector2::new(5, 0), Cell::new('A'));
    expected_canvas.set_cell(Vector2::new(6, 0), Cell::new('B'));
    expected_canvas.set_cell(Vector2::new(7, 0), Cell::new('C'));
    assert_eq!(info.canvas, expected_canvas);
    info.execute_command(TerminalCommand::csi("D"));
    assert_eq!(info.cursor, Vector2::new(6, 0));
    info.execute_command(TerminalCommand::csi("10X"));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    expected_canvas.set_cell(Vector2::new(5, 0), Cell::new('A'));
    assert_eq!(info.canvas, expected_canvas);
    assert_eq!(info.cursor, Vector2::new(6, 0));
}

#[test]
fn erase_character_reset_pending_wrap_state() {
    let mut info = TerminalInfo::new(Vector2::new(8, 8));
    info.execute_command(TerminalCommand::csi(format!("{}G",info.canvas.size().x)));
    assert_eq!(info.cursor, Vector2::new(7, 0));
    info.execute_command(TerminalCommand::string("A"));
    assert_eq!(info.cursor, Vector2::new(7, 0));
    info.execute_command(TerminalCommand::csi("X"));
    let expected_canvas = Canvas::new(Vector2::new(8, 8));
    assert_eq!(info.canvas, expected_canvas);
    assert_eq!(info.cursor, Vector2::new(7, 0));
    info.execute_command(TerminalCommand::string("X"));
    assert_eq!(info.cursor, Vector2::new(7, 0));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    expected_canvas.set_cell(Vector2::new(7, 0), Cell::new('X'));
    assert_eq!(info.canvas, expected_canvas);
}

#[test]
fn erase_character_sgr_state() {
    let mut info = TerminalInfo::new(Vector2::new(8, 8));
    info.execute_command(TerminalCommand::string("A"));
    info.execute_command(TerminalCommand::string("B"));
    info.execute_command(TerminalCommand::string("C"));
    info.execute_command(TerminalCommand::csi("1G"));
    info.execute_command(TerminalCommand::csi("41m"));
    info.execute_command(TerminalCommand::csi("2X"));
    assert_eq!(info.cursor, Vector2::new(0, 0));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    expected_canvas.set_cell(Vector2::new(2, 0), Cell::new('C'));
    let empty_red = Cell::new_styled(
        " ",
         Style::default()
            .with_background_color(Color::new_one_byte(1))
    );
    expected_canvas.set_cell(Vector2::new(0, 0), empty_red.clone());
    expected_canvas.set_cell(Vector2::new(1, 0), empty_red.clone());
    assert_eq!(info.canvas, expected_canvas);
}

#[test]
fn cursor_backwards_pending_wrap_is_unset() {
    let mut info = TerminalInfo::new(Vector2::new(8, 8));
    info.execute_command(TerminalCommand::csi(format!("{}G",info.canvas.size().x)));
    info.execute_command(TerminalCommand::string("A"));
    info.execute_command(TerminalCommand::csi("D"));
    info.execute_command(TerminalCommand::string("X"));
    info.execute_command(TerminalCommand::string("Y"));
    info.execute_command(TerminalCommand::string("Z"));
    assert_eq!(info.cursor, Vector2::new(1, 1));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    expected_canvas.set_cell(Vector2::new(6, 0), Cell::new('X'));
    expected_canvas.set_cell(Vector2::new(7, 0), Cell::new('Y'));
    expected_canvas.set_cell(Vector2::new(0, 1), Cell::new('Z'));
    assert_eq!(info.canvas, expected_canvas);
}

#[test]
fn insert_character_no_scroll_region_fits_on_screen() {
    let mut info = TerminalInfo::new(Vector2::new(8, 8));
    info.execute_command(TerminalCommand::string("A"));
    info.execute_command(TerminalCommand::string("B"));
    info.execute_command(TerminalCommand::string("C"));
    info.execute_command(TerminalCommand::csi("1G"));
    info.execute_command(TerminalCommand::csi("2@"));
    info.execute_command(TerminalCommand::string("X"));
    assert_eq!(info.cursor, Vector2::new(1, 0));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    expected_canvas.set_cell(Vector2::new(0, 0), Cell::new('X'));
    expected_canvas.set_cell(Vector2::new(1, 0), Cell::new(' '));
    expected_canvas.set_cell(Vector2::new(2, 0), Cell::new('A'));
    expected_canvas.set_cell(Vector2::new(3, 0), Cell::new('B'));
    expected_canvas.set_cell(Vector2::new(4, 0), Cell::new('C'));
    assert_eq!(info.canvas, expected_canvas);
}

#[test]
fn insert_character_sgr_state() {
    let mut info = TerminalInfo::new(Vector2::new(8, 8));
    info.execute_command(TerminalCommand::string("A"));
    info.execute_command(TerminalCommand::string("B"));
    info.execute_command(TerminalCommand::string("C"));
    info.execute_command(TerminalCommand::csi("1G"));
    info.execute_command(TerminalCommand::csi("41m"));
    info.execute_command(TerminalCommand::csi("2@"));
    assert_eq!(info.cursor, Vector2::new(0, 0));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    let empty_red = Cell::new_styled(
        " ",
         Style::default()
            .with_background_color(Color::new_one_byte(1))
    );
    expected_canvas.set_cell(Vector2::new(0, 0), empty_red.clone());
    expected_canvas.set_cell(Vector2::new(1, 0), empty_red.clone());
    expected_canvas.set_cell(Vector2::new(2, 0), Cell::new('A'));
    expected_canvas.set_cell(Vector2::new(3, 0), Cell::new('B'));
    expected_canvas.set_cell(Vector2::new(4, 0), Cell::new('C'));
    assert_eq!(info.canvas, expected_canvas);
}

#[test]
fn insert_character_shifting_content_off_the_screen() {
    let mut info = TerminalInfo::new(Vector2::new(8, 8));
    info.execute_command(TerminalCommand::csi(format!("{}G",info.canvas.size().x)));
    info.execute_command(TerminalCommand::csi("2D"));
    info.execute_command(TerminalCommand::string("A"));
    info.execute_command(TerminalCommand::string("B"));
    info.execute_command(TerminalCommand::string("C"));
    info.execute_command(TerminalCommand::csi("2D"));
    info.execute_command(TerminalCommand::csi("2@"));
    info.execute_command(TerminalCommand::string("X"));
    assert_eq!(info.cursor, Vector2::new(6, 0));
    let mut expected_canvas = Canvas::new(Vector2::new(8, 8));
    expected_canvas.set_cell(Vector2::new(5, 0), Cell::new('X'));
    expected_canvas.set_cell(Vector2::new(6, 0), Cell::new(' '));
    expected_canvas.set_cell(Vector2::new(7, 0), Cell::new('A'));
    assert_eq!(info.canvas, expected_canvas);
}