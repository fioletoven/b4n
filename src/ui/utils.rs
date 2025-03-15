use anyhow::Result;
use ratatui::{
    crossterm::{
        ExecutableCommand,
        terminal::{LeaveAlternateScreen, disable_raw_mode},
    },
    layout::{Constraint, Direction, Flex, Layout, Rect},
};
use std::{
    io::stdout,
    panic::{set_hook, take_hook},
};

/// Centers a [`Rect`] within another [`Rect`] using the provided [`Constraint`]s.
pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal]).flex(Flex::Center).areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

/// Centers horizontally a [`Rect`] within another [`Rect`] using the provided width and max height.
pub fn center_horizontal(area: Rect, width: u16, max_height: usize) -> Rect {
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

    if usize::from(layout[1].height) > max_height {
        Rect::new(layout[1].x, layout[1].y, layout[1].width, max_height as u16)
    } else {
        layout[1]
    }
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
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
