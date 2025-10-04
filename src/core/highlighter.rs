use ratatui::style::Style;
use std::thread::JoinHandle;
use syntect::easy::HighlightLines;
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot::Sender,
};

use crate::{core::SyntaxData, ui::colors::from_syntect_color};

/// Possible errors from fetching or styling resource's YAML.
#[derive(thiserror::Error, Debug)]
pub enum HighlightError {
    /// YAML syntax definition not found.
    #[error("YAML syntax definition not found")]
    SyntaxNotFound,

    /// Cannot highlight YAML syntax.
    #[error("cannot highlight YAML syntax")]
    SyntaxHighlightingError(#[from] syntect::Error),
}

pub enum HighlightRequest {
    Full {
        lines: Vec<String>,
        response: Sender<Result<HighlightResponse, HighlightError>>,
    },
    FromLine {
        start: usize,
        end: usize,
        lines: Vec<String>,
        response: Sender<Result<HighlightResponse, HighlightError>>,
    },
}

pub enum HighlightResponse {
    Full {
        lines: Vec<Vec<(Style, String)>>,
    },
    FromLine {
        start: usize,
        end: usize,
        lines: Vec<Vec<(Style, String)>>,
    },
}

pub struct BgHighlighter {
    thread: Option<JoinHandle<Result<(), HighlightError>>>,
    request_tx: UnboundedSender<HighlightRequest>,
}

impl BgHighlighter {
    pub fn new(data: SyntaxData) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel::<HighlightRequest>();
        let thread = std::thread::spawn(move || highlighter_task(data, request_rx));

        Self {
            thread: Some(thread),
            request_tx,
        }
    }

    pub fn get_sender(&self) -> UnboundedSender<HighlightRequest> {
        self.request_tx.clone()
    }
}

fn highlighter_task(data: SyntaxData, mut rx: UnboundedReceiver<HighlightRequest>) -> Result<(), HighlightError> {
    let syntax = data
        .syntax_set
        .find_syntax_by_extension("yaml")
        .ok_or(HighlightError::SyntaxNotFound)?;

    let mut highlighter = HighlightLines::new(syntax, &data.yaml_theme);
    let mut all_lines = Vec::new();

    while let Some(request) = rx.blocking_recv() {
        match request {
            HighlightRequest::Full { lines, response } => {
                all_lines = lines;
                highlighter = HighlightLines::new(syntax, &data.yaml_theme);
                let styled = all_lines
                    .iter()
                    .map(|line| {
                        Ok(highlighter
                            .highlight_line(line, &data.syntax_set)?
                            .into_iter()
                            .map(|segment| (convert_style(segment.0), segment.1.to_owned()))
                            .collect::<Vec<_>>())
                    })
                    .collect::<Result<Vec<_>, syntect::Error>>();
                let _ = response.send(match styled {
                    Ok(styled) => Ok(HighlightResponse::Full { lines: styled }),
                    Err(err) => Err(err.into()),
                });
            },
            HighlightRequest::FromLine {
                start,
                end,
                lines,
                response,
            } => todo!(),
        }
    }

    Ok(())
}

fn convert_style(style: syntect::highlighting::Style) -> Style {
    Style::default()
        .fg(from_syntect_color(style.foreground))
        .bg(from_syntect_color(style.background))
}
