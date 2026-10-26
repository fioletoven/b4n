use b4n_config::symbols::Symbols;
use b4n_config::themes::TextBoxModalColors;
use b4n_kube::ResourceTag;
use b4n_kube::files::TransferContext;
use b4n_tui::ResponseEvent;
use b4n_tui::widgets::{Button, CheckBox, Dialog, Selector, TextBox, ValidatorKind};
use ratatui::layout::Position;
use std::path::PathBuf;

use crate::core::SharedAppData;
use crate::ui::views::common;
use crate::ui::widgets::FileSelector;

/// Creates new transfer files dialog.
pub fn new_transfer_dialog(
    is_download: bool,
    app_data: &SharedAppData,
    tags: &[ResourceTag],
    position: Option<Position>,
) -> Dialog {
    let colors = &app_data.borrow().theme.colors;
    let symbols = &app_data.borrow().config.symbols;
    Dialog::new(
        get_transfer_dialog_title(is_download),
        vec![
            Button::new(
                "Transfer",
                ResponseEvent::Action("transfer_file"),
                colors.modal.btn_accent.clone(),
            ),
            Button::new("Cancel", ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
        ],
    )
    .with_colors(colors.modal.text)
    .with_checkboxes(vec![
        CheckBox::new(0, "Download", is_download, colors.modal.checkbox.clone(), symbols),
        CheckBox::new(1, "Overwrite files", false, colors.modal.checkbox.clone(), symbols),
    ])
    .with_textboxes(get_transfer_dialog_textboxes(
        app_data,
        is_download,
        &colors.modal.textbox,
        symbols,
    ))
    .with_selectors(vec![Selector::new(
        0,
        "Container:",
        &common::get_all_containers_from_resource_tags(tags),
        &colors.modal.selector,
        symbols,
    )])
    .with_highlighted_position(position)
    .with_on_change(|message, controls| {
        let is_checked = controls.checkbox(0).is_some_and(|c| c.is_checked);
        let is_download = message.starts_with("Download");
        let is_ok = (is_checked && is_download) || (!is_checked && !is_download);
        if !is_ok {
            *message = get_transfer_dialog_title(is_checked);
            controls.controls_mut().swap(2, 3);

            if let Some(textbox) = controls.controls_mut()[2].as_textbox_mut() {
                update_textbox_caption(textbox, true);
            }

            if let Some(textbox) = controls.controls_mut()[3].as_textbox_mut() {
                update_textbox_caption(textbox, false);
            }
        }
    })
}

/// Returns file transfer context for background task.
pub fn get_transfer_dialog_context(dialog: &Dialog) -> Option<TransferContext> {
    let is_download = dialog.checkbox(0).is_some_and(|cb| cb.is_checked);
    let overwrite_files = dialog.checkbox(1).is_some_and(|cb| cb.is_checked);
    let container = dialog.selector(0).map(|s| s.selected().to_owned())?;
    let first = dialog.textbox(0).map(|tb| tb.value().to_owned())?;
    let second = dialog.textbox(1).map(|tb| tb.value().to_owned())?;

    if is_download {
        Some(TransferContext::download(second, first, container, overwrite_files))
    } else {
        Some(TransferContext::upload(first, second, container, overwrite_files))
    }
}

/// Updates transfer dialog textboxes with the selected path from a file picker.
pub fn update_transfer_dialog_paths(dialog: &mut Dialog, file_picker: &FileSelector) {
    let is_download = dialog.checkbox(0).is_some_and(|cb| cb.is_checked);
    let is_dir = is_download != file_picker.is_remote() || file_picker.is_selected_dir();
    let (path, exists) = file_picker.selected_path();

    if file_picker.is_remote() {
        if let Some(textbox) = dialog.textbox_mut(1)
            && let Ok(mut path) = path.into_os_string().into_string()
        {
            if is_dir && !path.ends_with('/') {
                path.push('/');
            }

            textbox.set_value(path);
        }

        return;
    }

    if (is_download || exists)
        && let Some(textbox) = dialog.textbox_mut(0)
        && let Ok(path) = path.clone().into_os_string().into_string()
    {
        let mut path = path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned();
        if is_dir && !path.ends_with(std::path::is_separator) {
            path.push(std::path::MAIN_SEPARATOR);
        }

        textbox.set_value(path);
    }
}

/// Returns the path for the file picker based on the dialog's textbox value and whether it's remote.
pub fn get_path_for_file_picker(dialog: &Dialog, remote: bool) -> PathBuf {
    let path = dialog.textbox(usize::from(remote)).map(TextBox::value).unwrap_or_default();
    if remote {
        common::get_remote_path_for_file_picker(path)
    } else {
        common::get_path_for_file_picker(path)
    }
}

fn get_transfer_dialog_title(is_download: bool) -> String {
    if is_download {
        "Download files from the specified container:".to_owned()
    } else {
        "Upload files to the specified container:".to_owned()
    }
}

fn get_transfer_dialog_textboxes(
    app_data: &SharedAppData,
    is_download: bool,
    colors: &TextBoxModalColors,
    symbols: &Symbols,
) -> Vec<TextBox> {
    fn get_textbox(
        app_data: &SharedAppData,
        is_download: bool,
        is_first: bool,
        colors: &TextBoxModalColors,
        symbols: &Symbols,
    ) -> TextBox {
        let mut textbox = TextBox::new(usize::from(is_download), "", 40, colors.clone(), symbols)
            .with_button(
                format!(" {} ", symbols.dir_rev.ch),
                if is_download { "select_remote_file" } else { "select_file" },
            )
            .with_clipboard(app_data.borrow().get_clipboard())
            .with_validator(ValidatorKind::Required);
        update_textbox_caption(&mut textbox, is_first);
        if is_download {
            textbox.set_value("~/");
        }

        textbox
    }

    vec![
        get_textbox(app_data, is_download, true, colors, symbols),
        get_textbox(app_data, !is_download, false, colors, symbols),
    ]
}

fn update_textbox_caption(textbox: &mut TextBox, is_first: bool) {
    textbox.set_caption(if is_first { "From:     " } else { "To (dir): " });
}
