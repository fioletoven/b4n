use b4n_common::truncate_left;
use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_list::Row;
use b4n_tasks::dir_lister::{DirListResult, DirLister};
use b4n_tui::table::Table;
use b4n_tui::widgets::{ErrorHighlightMode, Select, Spinner};
use b4n_tui::{ResponseEvent, TuiEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tokio::runtime::Handle;

use crate::core::{SharedAppData, SharedBgWorker};
use crate::ui::widgets::{PatternItem, PatternsList, Picker, PickerBehaviour};

const PROMPT_LEN: usize = 30;
const PROMPT_END: &str = " ";
const DIR_ICON: &str = "";
const BACK_ICON: &str = "󰕍";
const FILE_SELECT_HINT: &str = "Select or type a file path:";

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

    /// Sets the current directory path.
    pub fn set_current_path(&mut self, path: PathBuf) {
        if self.behaviour().current_path == path {
            return;
        }

        let behaviour = self.behaviour_mut();
        behaviour.prompt = truncate_prompt(&path);
        behaviour.current_path = path;
        behaviour.lister.reset();
        behaviour.loading = true;
    }
}

pub struct FileBehaviour {
    app_data: SharedAppData,
    lister: DirLister,
    current_path: PathBuf,
    prompt: String,
    loading: bool,
    spinner: Spinner,
}

impl FileBehaviour {
    pub fn new(app_data: SharedAppData, runtime: Handle, initial_path: PathBuf) -> Self {
        let prompt = truncate_prompt(&initial_path);

        Self {
            app_data,
            lister: DirLister::new(runtime, 100),
            current_path: initial_path,
            prompt,
            loading: true,
            spinner: Spinner::default(),
        }
    }

    fn navigate_to_dir(&mut self, dir_path: PathBuf) -> bool {
        self.prompt = truncate_prompt(&dir_path);
        self.current_path = dir_path.clone();
        self.lister.list_dir(dir_path, true)
    }

    fn navigate_up(&mut self) -> bool {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to_dir(parent.to_path_buf())
        } else {
            false
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
                        item.set_icon(Some(if entry.name == ".." { BACK_ICON } else { DIR_ICON }));
                        item.set_sort_value(Some(format!("...-{}", entry.name)));
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

    fn process_input_navigation(&mut self, patterns: &mut Select<PatternsList>) -> bool {
        let value = patterns.value_full();
        if value.is_empty() {
            return false;
        }

        let input_path = if Path::new(value).is_absolute() {
            PathBuf::from(value)
        } else {
            self.current_path.join(value)
        };

        let is_full = value.ends_with(['\\', '/']);
        let target_dir = if is_full {
            Some(input_path)
        } else {
            input_path.parent().map(|parent| parent.to_path_buf())
        };

        let has_prefix = is_full || !patterns.value_prefix().is_empty();
        if let Some(dir) = target_dir
            && self.lister.list_dir(dir, !has_prefix)
        {
            patterns.items.clear();
            if is_full {
                patterns.items.set_filter(None);
            }

            return true;
        }

        false
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

    fn filter_delimiters(&self) -> Vec<char> {
        vec!['\\', '/']
    }

    fn highlight_exact(&self) -> bool {
        true
    }

    fn reset_key_command(&self) -> KeyCommand {
        KeyCommand::CommandPaletteReset
    }

    fn cancel_response(&self) -> ResponseEvent {
        ResponseEvent::Handled
    }

    fn load_items(&mut self) -> PatternsList {
        self.lister.reset();
        self.lister.list_dir(self.current_path.clone(), true);
        PatternsList::default()
    }

    fn add_item(&self, _item: &str) {}

    fn remove_item(&self, _item: &str) -> bool {
        false
    }

    fn can_remove(&self, _item: Option<&PatternItem>) -> bool {
        false
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

    fn on_reset(&mut self, patterns: &mut Select<PatternsList>) -> bool {
        if !patterns.value_prefix().is_empty() && self.navigate_to_dir(self.current_path.clone()) {
            patterns.items.clear();
            patterns.reset();
        }

        true
    }

    fn on_close(&mut self, patterns: &mut Select<PatternsList>, is_cancel: bool) -> bool {
        if is_cancel {
            return true;
        };

        let Some(item) = patterns.items.get_highlighted() else {
            return true;
        };

        if item.icon().is_some_and(|i| i == DIR_ICON || i == BACK_ICON) {
            if item.name() == ".." {
                self.navigate_up();
            } else {
                let path = self.current_path.join(normalize(patterns.value_prefix())).join(item.value());
                self.navigate_to_dir(path);
            }

            patterns.set_prompt(self.prompt());
            patterns.items.clear();
            patterns.reset();

            false
        } else {
            true
        }
    }

    fn on_draw(&mut self, patterns: &mut Select<PatternsList>, _area: Rect) {
        self.process_dir_results(patterns);
    }

    fn has_header(&self) -> bool {
        true
    }

    fn draw_header(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect, style: Style) {
        let line = format!(
            "{} {}",
            if self.loading { self.spinner.tick() } else { '' },
            FILE_SELECT_HINT
        );
        frame.render_widget(Paragraph::new(line).style(style), area);
    }

    fn post_process_event(&mut self, event: &TuiEvent, patterns: &mut Select<PatternsList>, _: &SharedAppData) -> ResponseEvent {
        if let TuiEvent::Key(_) = event {
            self.process_input_navigation(patterns);
        }

        ResponseEvent::NotHandled
    }
}

fn truncate_prompt(path: &Path) -> String {
    let prompt = format!("{}{}", path.display(), PROMPT_END);
    if prompt.len() > PROMPT_LEN {
        format!("…{}", truncate_left(&prompt, PROMPT_LEN.saturating_sub(1)))
    } else {
        prompt
    }
}

fn normalize(path: &str) -> PathBuf {
    Path::new(path).components().collect()
}
