use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::cursor::SetCursorStyle;
use crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use ratatui_core::layout::{Constraint, Direction, Flex, Layout, Rect};
use std::io::stdout;
use std::panic::{set_hook, take_hook};

/// Centers a [`Rect`] within another [`Rect`] using the provided [`Constraint`]s.
pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal]).flex(Flex::Center).areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

/// Centers horizontally a [`Rect`] within another [`Rect`] using the provided width and max height.
pub fn center_horizontal(area: Rect, width: u16, max_height: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center).areas(area);
    let top = if area.height > 2 { (area.height - 2).min(3) } else { 0 };
    let mut bottom = if area.height > 5 { (area.height - 5).min(6) } else { 0 };
    if area.height >= 7 && area.height <= 14 {
        bottom = area.height.saturating_sub(9).max(2);
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(top), Constraint::Fill(1), Constraint::Length(bottom)])
        .split(area);

    if layout[1].height > max_height {
        Rect::new(layout[1].x, layout[1].y, layout[1].width, max_height)
    } else {
        layout[1]
    }
}

/// Recalculates width for a bigger terminal screen (`> 140`).
pub fn get_proportional_width(area_width: u16, width: u16, use_proportion: bool) -> u16 {
    const MIN_SCREEN_WIDTH: u16 = 140;

    if use_proportion && area_width > MIN_SCREEN_WIDTH && area_width > width {
        let width = area_width * width / MIN_SCREEN_WIDTH;
        return area_width.min(width).saturating_sub(2);
    }

    area_width.min(width).saturating_sub(2)
}

/// Sets panic hook that additionally leaves alternate screen mode on panic.
pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}

/// Leaves alternate screen mode.
fn restore_terminal() -> Result<()> {
    stdout().execute(SetCursorStyle::DefaultUserShape)?;
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
