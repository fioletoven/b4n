use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_list::Row;
use b4n_tasks::dir_lister::{DirListResult, DirLister};
use b4n_tui::ResponseEvent;
use b4n_tui::table::Table;
use b4n_tui::widgets::{ErrorHighlightMode, Select};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::runtime::Handle;

use crate::core::{SharedAppData, SharedBgWorker};
use crate::ui::widgets::{PatternItem, PatternsList, Picker, PickerBehaviour};

const PROMPT_END: &str = " ";
const DIR_ICON: &str = "";

pub type FileSelector = Picker<FileBehaviour>;

impl FileSelector {
    /// Creates new [`FileSelector`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, width: u16, initial_path: PathBuf) -> Self {
        let runtime = worker.borrow().runtime_handle().clone();
        let behaviour = FileBehaviour::new(Rc::clone(&app_data), runtime, initial_path);
        Picker::new_picker(app_data, Some(worker), width, behaviour).with_highlight_on_complete(true)
    }

    /// Gets the current directory path.
    pub fn current_path(&self) -> &PathBuf {
        &self.behaviour().current_path
    }
}

pub struct FileBehaviour {
    app_data: SharedAppData,
    current_path: PathBuf,
    lister: DirLister,
    loading: bool,
    prompt: String,
}

impl FileBehaviour {
    pub fn new(app_data: SharedAppData, runtime: Handle, initial_path: PathBuf) -> Self {
        let prompt = format!("{}{}", initial_path.display(), PROMPT_END);

        Self {
            app_data,
            current_path: initial_path,
            lister: DirLister::new(runtime, 100).with_parent(true),
            loading: true,
            prompt,
        }
    }

    fn navigate_to_dir(&mut self, dir_path: PathBuf) {
        self.prompt = format!("{}{}", dir_path.display(), PROMPT_END);
        self.current_path = dir_path.clone();
        self.lister.list_dir(dir_path);
    }

    fn navigate_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to_dir(parent.to_path_buf());
        }
    }

    fn process_dir_results(&mut self, patterns: &mut Select<PatternsList>) {
        let mut updated = false;

        while let Some(result) = self.lister.try_recv() {
            updated = true;
            match result {
                DirListResult::Init => {
                    patterns.items.clear();
                    self.loading = true;
                },
                DirListResult::Entry(entry) => {
                    let mut item = PatternItem::fixed(entry.name.clone());
                    if entry.is_dir {
                        item.set_icon(Some(DIR_ICON));
                        item.set_sort_value(Some(format!("0-{}", item.value())));
                    }

                    patterns.items.add_or_update(item);
                },
                DirListResult::Complete => {
                    self.loading = false;
                    if patterns.value().is_empty() {
                        patterns.highlight_first();
                    }
                },
                DirListResult::Error(e) => {
                    tracing::error!("Error listing directory: {}", e);
                    self.loading = false;
                },
            }
        }

        if updated {
            patterns.update_items_filter();
        }
    }
}

impl PickerBehaviour for FileBehaviour {
    fn prompt(&self) -> &str {
        &self.prompt
    }

    fn colors(&self) -> SelectColors {
        self.app_data.borrow().theme.colors.command_palette.clone()
    }

    fn accent_characters(&self) -> Option<&str> {
        Some("/\\")
    }

    fn highlight_exact(&self) -> bool {
        true
    }

    fn reset_key_command(&self) -> KeyCommand {
        KeyCommand::CommandPaletteReset
    }

    fn cancel_response(&self) -> ResponseEvent {
        ResponseEvent::Cancelled
    }

    fn load_items(&mut self) -> PatternsList {
        self.lister.list_dir(self.current_path.clone());
        PatternsList::default()
    }

    fn add_item(&self, _item: &str) {}

    fn remove_item(&self, _item: &str) -> bool {
        false
    }

    fn can_remove(&self, _item: Option<&PatternItem>) -> bool {
        false
    }

    fn on_close(&mut self, patterns: &mut Select<PatternsList>, is_cancel: bool) -> bool {
        if is_cancel {
            return true;
        };

        let Some(item) = patterns.items.get_highlighted() else {
            return true;
        };

        if item.icon().is_some_and(|i| i == DIR_ICON) {
            if item.name() == ".." {
                self.navigate_up();
            } else {
                let path = self.current_path.join(item.value());
                self.navigate_to_dir(path);
            }

            patterns.set_prompt(self.prompt());
            patterns.reset();
            patterns.items.clear();

            false
        } else {
            true
        }
    }

    fn error_mode(&self) -> ErrorHighlightMode {
        ErrorHighlightMode::Value
    }

    fn validate(&mut self, _value: &str) -> Option<usize> {
        None
    }

    fn restores_on_cancel(&self) -> bool {
        false
    }

    fn blocks_on_error(&self) -> bool {
        false
    }

    fn has_header(&self) -> bool {
        true
    }

    fn draw_header(&self, frame: &mut ratatui::Frame<'_>, area: Rect, style: Style) {
        frame.render_widget(Paragraph::new("Select file").style(style), area);
    }

    fn on_draw(&mut self, patterns: &mut Select<PatternsList>, _area: Rect) {
        self.process_dir_results(patterns);
    }
}
