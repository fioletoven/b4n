use b4n_config::themes::TextBoxModalColors;
use b4n_kube::ResourceTag;
use b4n_kube::files::TransferContext;
use b4n_tui::ResponseEvent;
use b4n_tui::widgets::{Button, CheckBox, Dialog, Selector, TextBox, ValidatorKind};
use ratatui::layout::Position;

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
        CheckBox::new(0, "Download", is_download, colors.modal.checkbox.clone()),
        CheckBox::new(1, "Overwrite files", false, colors.modal.checkbox.clone()),
    ])
    .with_textboxes(get_transfer_dialog_textboxes(app_data, is_download, &colors.modal.textbox))
    .with_selectors(vec![Selector::new(
        0,
        "Container:",
        &common::get_all_containers_from_resource_tags(tags),
        &colors.modal.selector,
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
                setup_transfer_dialog_textbox(textbox, is_download, true);
            }

            if let Some(textbox) = controls.controls_mut()[3].as_textbox_mut() {
                setup_transfer_dialog_textbox(textbox, !is_download, false);
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
    let (path, exists) = file_picker.selected_path();
    let is_download = dialog.checkbox(0).is_some_and(|cb| cb.is_checked);
    if is_download || exists {
        if let Some(textbox) = dialog.textbox_mut(0)
            && let Ok(path) = path.clone().into_os_string().into_string()
        {
            textbox.set_value(path);
        }

        if let Some(textbox) = dialog.textbox_mut(1)
            && textbox.value().is_empty()
        {
            textbox.set_value("~/");
        }
    }
}

fn get_transfer_dialog_title(is_download: bool) -> String {
    if is_download {
        "Download files from the specified container:".to_owned()
    } else {
        "Upload file to the specified container:".to_owned()
    }
}

fn get_transfer_dialog_textboxes(app_data: &SharedAppData, is_download: bool, colors: &TextBoxModalColors) -> Vec<TextBox> {
    fn get_textbox(app_data: &SharedAppData, is_download: bool, is_first: bool, colors: &TextBoxModalColors) -> TextBox {
        let mut textbox = TextBox::new(usize::from(is_download), "", 40, colors.clone())
            .with_button("  ", "select_file")
            .with_clipboard(app_data.borrow().get_clipboard())
            .with_validator(ValidatorKind::Required);
        setup_transfer_dialog_textbox(&mut textbox, !is_download, is_first);
        if is_download {
            textbox.set_value("~/");
        }

        textbox
    }

    vec![
        get_textbox(app_data, is_download, true, colors),
        get_textbox(app_data, !is_download, false, colors),
    ]
}

fn setup_transfer_dialog_textbox(textbox: &mut TextBox, has_button: bool, is_first: bool) {
    textbox.show_button(has_button);
    textbox.set_caption(if is_first { "From:     " } else { "To (dir): " });
}
