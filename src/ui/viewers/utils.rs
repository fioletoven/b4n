use kube::discovery::Scope;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::{
    core::{AppData, ResourcesInfo},
    kubernetes::{
        ALL_NAMESPACES,
        resources::{EVENTS, PODS},
    },
    ui::colors::TextColors,
};

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
