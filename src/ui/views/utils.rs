use kube::discovery::Scope;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::{
    core::{AppData, ResourcesInfo},
    kubernetes::{ALL_NAMESPACES, resources::PODS},
    ui::colors::TextColors,
};

/// Returns name of the namespace that can be displayed on the header pane breadcrumbs.
pub fn get_breadcumbs_namespace<'a>(data: &'a ResourcesInfo, kind: &str) -> &'a str {
    if data.scope == Scope::Namespaced || kind == PODS {
        let force_all = kind != PODS && data.is_all_namespace();
        let namespace = if force_all { ALL_NAMESPACES } else { data.namespace.as_str() };
        return namespace;
    }

    ""
}

/// Returns formatted text as left breadcrumbs:\
/// \> `context` \> \[ `namespace` \> \] `kind` \> \[ `name` \> \] `count` \>
pub fn get_left_breadcrumbs<'a>(data: &AppData, kind: &str, name: Option<&str>, count: usize, is_filtered: bool) -> Line<'a> {
    let colors = &data.theme.colors.header;
    let data = &data.current;

    let mut path = vec![
        Span::styled("", Style::new().fg(colors.context.bg)),
        Span::styled(format!(" {} ", data.context), &colors.context),
    ];

    if data.scope == Scope::Namespaced || kind == PODS {
        path.append(&mut vec![
            Span::styled("", Style::new().fg(colors.context.bg).bg(colors.namespace.bg)),
            Span::styled(format!(" {} ", get_breadcumbs_namespace(data, kind)), &colors.namespace),
            Span::styled("", Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
        ]);
    } else {
        path.push(Span::styled("", Style::new().fg(colors.context.bg).bg(colors.resource.bg)));
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
    } else if name.is_some() {
        ""
    } else {
        ""
    };

    path.append(&mut vec![
        Span::styled(format!(" {}{} ", count_icon, count), &colors.count),
        Span::styled("", Style::new().fg(colors.count.bg)),
    ]);

    Line::from(path)
}

/// Returns formatted text as right breadcrumbs:\
/// \< `text` \<
pub fn get_right_breadcrumbs<'a>(text: String, colors: &TextColors) -> Line<'a> {
    Line::from(vec![
        Span::styled("", Style::new().fg(colors.bg)),
        Span::styled(text, colors),
        Span::styled("", Style::new().fg(colors.bg)),
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
