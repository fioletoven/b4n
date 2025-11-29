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

    /// Inserts specified `text` at `position` to the vector of `String`s.
    fn insert_text(&mut self, position: ContentPosition, text: Vec<String>) -> ContentPosition;
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

        if start_line == end_line {
            remove_line(self, end_line, start.x, end.x)
        } else {
            remove_lines(self, start_line, end_line, start.x, end.x)
        }
    }

    fn insert_text(&mut self, position: ContentPosition, mut text: Vec<String>) -> ContentPosition {
        match text.len() {
            0 => position,
            1 => insert_line(self, position, text.swap_remove(0)),
            _ => insert_lines(self, position, text),
        }
    }
}

fn remove_line(lines: &mut Vec<String>, line_no: usize, start: usize, end: usize) -> Vec<String> {
    let is_eol = lines[line_no].chars().count() <= end;
    if let Some(start) = char_to_index(&lines[line_no], start) {
        if let Some(end) = char_to_index(&lines[line_no], end + 1) {
            let removed = lines[line_no].drain(start..end).collect();
            vec![removed]
        } else {
            let removed = lines[line_no].drain(start..).collect();
            if is_eol {
                lines.join_lines(line_no);
                vec![removed, String::new()]
            } else {
                vec![removed]
            }
        }
    } else if is_eol {
        lines.join_lines(line_no);
        vec![String::new(), String::new()]
    } else {
        Vec::default()
    }
}

fn remove_lines(lines: &mut Vec<String>, start_line: usize, end_line: usize, start: usize, end: usize) -> Vec<String> {
    let is_eol = lines[end_line].chars().count() <= end;
    let mut removed = Vec::new();
    let mut remove_start = false;

    if let Some(start) = char_to_index(&lines[start_line], start) {
        removed.push(lines[start_line].drain(start..).collect());
        remove_start = start == 0;
    }

    let last = if let Some(end) = char_to_index(&lines[end_line], end + 1) {
        lines[end_line].drain(..end).collect()
    } else {
        lines[end_line].drain(..).collect()
    };

    if is_eol {
        lines.join_lines(end_line);
    }

    removed.append(&mut drain_lines(
        lines,
        start_line.saturating_add(1),
        end_line.saturating_sub(1),
    ));

    if remove_start {
        lines.remove(start_line);
    } else {
        lines.join_lines(start_line);
    }

    removed.push(last);
    if is_eol {
        removed.push(String::new());
    }

    removed
}

fn drain_lines(lines: &mut Vec<String>, from: usize, to: usize) -> Vec<String> {
    if from <= to && from < lines.len() {
        let to = to.min(lines.len());
        lines.drain(from..=to).collect()
    } else {
        Vec::default()
    }
}

fn insert_line(lines: &mut Vec<String>, position: ContentPosition, text: String) -> ContentPosition {
    let text_len = text.chars().count();
    if lines.is_empty() || position.y >= lines.len() {
        lines.push(text);
        return ContentPosition::new(text_len, lines.len().saturating_sub(1));
    }

    if lines.len() == 1 && lines[0].is_empty() {
        lines[0] = text;
        return ContentPosition::new(text_len, 0);
    }

    if let Some(line) = lines.get_mut(position.y) {
        if let Some(x) = char_to_index(line, position.x) {
            line.insert_str(x, &text);
            return ContentPosition::new(x + text_len, position.y);
        } else {
            line.push_str(&text);
            return ContentPosition::new(line.chars().count(), position.y);
        }
    }

    position
}

fn insert_lines(lines: &mut Vec<String>, position: ContentPosition, mut text: Vec<String>) -> ContentPosition {
    if lines.is_empty() || (lines.len() == 1 && lines[0].is_empty()) {
        *lines = text;
        return get_end_position(lines);
    }

    if position.y >= lines.len() {
        lines.append(&mut text);
        return get_end_position(lines);
    }

    let first_line = text.swap_remove(0);
    let mut middle_lines = if text.len() > 1 { text.split_off(1) } else { Vec::default() };
    let last_line = text.swap_remove(0);
    let last_line_len = last_line.chars().count();
    let insert_at = position.y + 1;

    let last_line = if let Some(x) = char_to_index(&lines[position.y], position.x) {
        let mut rest = lines[position.y][x..].to_string();
        lines[position.y].truncate(x);
        lines[position.y].push_str(&first_line);
        rest.insert_str(0, &last_line);
        rest
    } else if lines[position.y].chars().count() == position.x {
        lines[position.y].push_str(&first_line);
        last_line
    } else {
        last_line
    };

    middle_lines.push(last_line);

    if insert_at < lines.len() {
        let new_y = insert_at + middle_lines.len().saturating_sub(1);
        lines.splice(insert_at..insert_at, middle_lines);
        ContentPosition::new(last_line_len, new_y)
    } else {
        lines.append(&mut middle_lines);
        ContentPosition::new(last_line_len, lines.len().saturating_sub(1))
    }
}

fn get_end_position(lines: &[String]) -> ContentPosition {
    ContentPosition::new(
        lines.last().map(|l| l.chars().count()).unwrap_or_default(),
        lines.len().saturating_sub(1),
    )
}
