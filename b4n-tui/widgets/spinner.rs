/// Spinner widget for TUI.
pub struct Spinner {
    frames: Box<[char]>,
    speed: usize,
    frame: usize,
    ticks: usize,
}

impl Spinner {
    /// Creates new [`Spinner`] instance.
    pub fn new(use_plain_symbols: bool) -> Self {
        let frames = if use_plain_symbols {
            Box::new(['╵', '╰', '╶', '╭', '╷', '╮', '╴', '╯'])
        } else {
            Box::new(['⡇', '⠏', '⠛', '⠹', '⢸', '⣰', '⣤', '⣆'])
        };
        Self {
            frames,
            speed: 4,
            frame: 0,
            ticks: 0,
        }
    }

    /// Advances spinner one tick and returns character for the current frame.
    pub fn tick(&mut self) -> char {
        self.advance_frame();
        self.frames[self.frame]
    }

    fn advance_frame(&mut self) {
        self.ticks += 1;
        if self.ticks >= self.speed {
            self.ticks = 0;
            self.frame += 1;
            if self.frame >= self.frames.len() {
                self.frame = 0;
            }
        }
    }
}
