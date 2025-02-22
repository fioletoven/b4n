use anyhow::Result;
use ratatui::{
    crossterm::{
        ExecutableCommand,
        terminal::{LeaveAlternateScreen, disable_raw_mode},
    },
    layout::{Constraint, Flex, Layout, Rect},
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
