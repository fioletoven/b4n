use b4n_config::themes::TextColors;
use b4n_kube::{ALL_NAMESPACES, EVENTS, PODS};
use kube::discovery::Scope;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::core::{AppData, ResourcesInfo};
use crate::ui::presentation::{ContentPosition, Selection};

#[cfg(test)]
#[path = "./utils.tests.rs"]
mod utils_tests;

/// Returns name of the namespace that can be displayed on the header pane breadcrumbs.
pub fn get_breadcrumbs_namespace<'a>(scope: Option<&Scope>, data: &'a ResourcesInfo, kind: &str) -> &'a str {
    let scope = if let Some(scope) = scope { scope } else { &data.scope };
    if *scope == Scope::Namespaced || kind == PODS {
        let force_all = kind != PODS && kind != EVENTS && data.is_all_namespace();
        let namespace = if force_all { ALL_NAMESPACES } else { data.namespace.as_str() };
        return namespace;
    }

    ""
}

/// Returns formatted text as left breadcrumbs:\
/// \> `context` \> \[ `namespace` \> \] `kind` \> \[ `name` \> \] `count` \>
pub fn get_left_breadcrumbs<'a>(
    app_data: &AppData,
    scope: Option<&Scope>,
    namespace: Option<&str>,
    kind: &str,
    name: Option<&str>,
    count: usize,
    is_filtered: bool,
) -> Line<'a> {
    let colors = &app_data.theme.colors.header;
    let context = get_context_color(app_data);
    let data = &app_data.current;

    let mut path = vec![
        Span::styled("", Style::new().fg(context.bg).bg(app_data.theme.colors.text.bg)),
        Span::styled(format!(" {} ", data.context), &context),
    ];

    let namespace = namespace.unwrap_or_else(|| get_breadcrumbs_namespace(scope, data, kind));
    let scope = if let Some(scope) = scope { scope } else { &data.scope };
    if !namespace.is_empty() && (*scope == Scope::Namespaced || kind == PODS) {
        path.append(&mut vec![
            Span::styled("", Style::new().fg(context.bg).bg(colors.namespace.bg)),
            Span::styled(format!(" {namespace} "), &colors.namespace),
            Span::styled("", Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
        ]);
    } else {
        path.push(Span::styled("", Style::new().fg(context.bg).bg(colors.resource.bg)));
    }

    path.push(Span::styled(format!(" {kind} "), &colors.resource));

    if name.is_some() {
        path.append(&mut vec![
            Span::styled("", Style::new().fg(colors.resource.bg).bg(colors.name.bg)),
            Span::styled(format!(" {} ", name.as_ref().unwrap()), &colors.name),
            Span::styled("", Style::new().fg(colors.name.bg).bg(colors.count.bg)),
        ]);
    } else {
        path.push(Span::styled("", Style::new().fg(colors.resource.bg).bg(colors.count.bg)));
    }

    let count_icon = if is_filtered {
        ""
    } else if data.resource.is_container() {
        ""
    } else {
        ""
    };

    path.append(&mut vec![
        Span::styled(format!(" {count_icon}{count} "), &colors.count),
        Span::styled("", Style::new().fg(colors.count.bg).bg(app_data.theme.colors.text.bg)),
    ]);

    Line::from(path)
}

/// Returns formatted text as right breadcrumbs:\
/// \< `text` \<
pub fn get_right_breadcrumbs<'a>(text: String, colors: &TextColors, bg: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled("", Style::new().fg(colors.bg).bg(bg)),
        Span::styled(text, colors),
        Span::styled("", Style::new().fg(colors.bg).bg(bg)),
    ])
    .right_aligned()
}

/// Returns kubernetes version text together with its colors.
pub fn get_version_text(data: &AppData) -> (String, &TextColors) {
    let colors;
    let text;

    if data.is_connected {
        colors = &data.theme.colors.header.info;
        text = format!(" {} ", &data.current.version);
    } else {
        colors = &data.theme.colors.header.disconnected;
        text = format!(
            "  {} ",
            if data.current.version.is_empty() {
                "connecting…"
            } else {
                &data.current.version
            }
        );
    }

    (text, colors)
}

fn get_context_color(app_data: &AppData) -> TextColors {
    app_data
        .config
        .contexts
        .as_ref()
        .and_then(|contexts| contexts.get(&app_data.current.context))
        .map_or(app_data.theme.colors.header.context, |f| *f)
}

#[derive(Default)]
pub struct CharPosition {
    pub char: usize,
    pub index: usize,
}

#[derive(Default)]
pub struct PositionSet {
    pub x_prev: CharPosition,
    pub x: CharPosition,
}

pub fn get_char_position(lines: &[String], position: ContentPosition) -> Option<PositionSet> {
    let line = lines.get(position.y)?;
    let mut result_set = PositionSet::default();

    for (char_idx, (byte_idx, _)) in line.char_indices().enumerate() {
        if char_idx + 1 == position.x {
            result_set.x_prev = CharPosition {
                char: char_idx,
                index: byte_idx,
            };
        }

        if char_idx == position.x {
            result_set.x = CharPosition {
                char: char_idx,
                index: byte_idx,
            };
            return Some(result_set);
        }
    }

    None
}

pub fn char_to_index(line: &str, char_idx: usize) -> Option<usize> {
    line.char_indices().nth(char_idx).map(|(byte_idx, _)| byte_idx)
}

pub trait VecStringExt {
    /// Appends the content of the next line to the line at `line_no` and removes the next line.
    fn join_lines(&mut self, line_no: usize);

    /// Removes and returns the specified `range` from the vector of `String`s.
    fn remove_text(&mut self, range: &Selection) -> Vec<String>;
}

impl VecStringExt for Vec<String> {
    fn join_lines(&mut self, line_no: usize) {
        if line_no + 1 < self.len() {
            let (left, right) = self.split_at_mut(line_no + 1);
            left[line_no].push_str(&right[0]);
            self.remove(line_no + 1);
        }
    }

    fn remove_text(&mut self, range: &Selection) -> Vec<String> {
        let (start, end) = range.sorted();
        let start_line = start.y.min(self.len().saturating_sub(1));
        let end_line = end.y.min(self.len().saturating_sub(1));
        let is_eol = self[end_line].chars().count() <= end.x;

        if start_line == end_line {
            if let Some(start) = char_to_index(&self[end_line], start.x) {
                if let Some(end) = char_to_index(&self[end_line], end.x + 1) {
                    let removed = self[end_line].drain(start..end).collect();
                    vec![removed]
                } else {
                    let removed = self[end_line].drain(start..).collect();
                    if is_eol {
                        self.join_lines(end_line);
                    }
                    vec![removed]
                }
            } else {
                Vec::default()
            }
        } else {
            let mut removed = Vec::new();
            let mut remove_start = false;

            if let Some(start) = char_to_index(&self[start_line], start.x) {
                removed.push(self[start_line].drain(start..).collect());
                remove_start = start == 0;
            }

            let last = if let Some(end) = char_to_index(&self[end_line], end.x + 1) {
                self[end_line].drain(..end).collect()
            } else {
                self[end_line].drain(..).collect()
            };

            if is_eol {
                self.join_lines(end_line);
            }

            removed.append(&mut remove_lines(
                self,
                start_line.saturating_add(1),
                end_line.saturating_sub(1),
            ));

            if remove_start {
                self.remove(start_line);
            } else {
                self.join_lines(start_line);
            }

            removed.push(last);
            if is_eol {
                removed.push(String::new());
            }

            removed
        }
    }
}

fn remove_lines(lines: &mut Vec<String>, from: usize, to: usize) -> Vec<String> {
    if from <= to && from < lines.len() {
        let to = to.min(lines.len());
        lines.drain(from..=to).collect()
    } else {
        Vec::default()
    }
}
