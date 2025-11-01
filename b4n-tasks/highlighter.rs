use b4n_config::{SyntaxData, themes::from_syntect_color};
use ratatui::style::Style;
use std::thread::JoinHandle;
use syntect::{easy::HighlightLines, parsing::SyntaxSet};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot::Sender,
};

/// Possible errors from fetching or styling resource's YAML.
#[derive(thiserror::Error, Debug)]
pub enum HighlightError {
    /// Specified start index is out of bound.
    #[error("specified start index is out of bound")]
    StartOutOfBound,

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
    Partial {
        start: usize,
        lines: Vec<String>,
        response: Sender<Result<HighlightResponse, HighlightError>>,
    },
}

pub struct HighlightResponse {
    pub plain: Vec<String>,
    pub styled: Vec<Vec<(Style, String)>>,
}

pub struct BgHighlighter {
    thread: Option<JoinHandle<Result<(), HighlightError>>>,
    request_tx: Option<UnboundedSender<HighlightRequest>>,
}

impl BgHighlighter {
    /// Creates new [`BgHighlighter`] instance.\
    /// **Note** that it immediately starts the background thread.
    pub fn new(data: SyntaxData) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel::<HighlightRequest>();
        let thread = std::thread::spawn(move || highlighter_task(&data, request_rx));

        Self {
            thread: Some(thread),
            request_tx: Some(request_tx),
        }
    }

    /// Returns unbounded channel sender for [`HighlightRequest`]s.
    pub fn get_sender(&self) -> Option<UnboundedSender<HighlightRequest>> {
        self.request_tx.clone()
    }

    /// Returns `true` if [`BgHighlighter`] is running.
    pub fn is_running(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }
}

impl Drop for BgHighlighter {
    fn drop(&mut self) {
        let _ = self.request_tx.take();
    }
}

fn highlighter_task(data: &SyntaxData, mut rx: UnboundedReceiver<HighlightRequest>) -> Result<(), HighlightError> {
    let syntax = data
        .syntax_set
        .find_syntax_by_extension("yaml")
        .ok_or(HighlightError::SyntaxNotFound)?;

    while let Some(request) = rx.blocking_recv() {
        let highlighter = HighlightLines::new(syntax, &data.yaml_theme);
        match request {
            HighlightRequest::Full { lines, response } => {
                let styled = highlight_all(highlighter, &data.syntax_set, &lines);
                let _ = response.send(match styled {
                    Ok(styled) => Ok(HighlightResponse { plain: lines, styled }),
                    Err(err) => Err(err.into()),
                });
            },
            HighlightRequest::Partial {
                start,
                mut lines,
                response,
            } => {
                if start >= lines.len() {
                    return Err(HighlightError::StartOutOfBound);
                }

                let styled = highlight_all(highlighter, &data.syntax_set, &lines);
                let _ = response.send(match styled {
                    Ok(mut styled) => Ok(HighlightResponse {
                        plain: lines.drain(start..).collect(),
                        styled: styled.drain(start..).collect(),
                    }),
                    Err(err) => Err(err.into()),
                });
            },
        }
    }

    Ok(())
}

fn highlight_all(
    mut highlighter: HighlightLines<'_>,
    syntax_set: &SyntaxSet,
    lines: &[String],
) -> Result<Vec<Vec<(Style, String)>>, syntect::Error> {
    lines
        .iter()
        .map(|line| {
            Ok(highlighter
                .highlight_line(line, syntax_set)?
                .into_iter()
                .map(|segment| (convert_style(segment.0), segment.1.to_owned()))
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>, syntect::Error>>()
}

fn convert_style(style: syntect::highlighting::Style) -> Style {
    Style::default()
        .fg(from_syntect_color(style.foreground))
        .bg(from_syntect_color(style.background))
}
