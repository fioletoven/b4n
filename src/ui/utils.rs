use anyhow::Result;
use ratatui::crossterm::{
    terminal::{disable_raw_mode, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::{
    io::stdout,
    panic::{set_hook, take_hook},
};

/// Sets panic hook that additionally leaves alternate screen mode on panic
pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}

/// Leaves alternate screen mode
fn restore_terminal() -> Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
