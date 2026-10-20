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
use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use tokio::runtime::Handle;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::{PatternItem, PatternsList, Picker, PickerBehaviour};

const BACK_NAME: &str = "..";
const FILE_SELECT_HINT: &str = "Select or type a file path:";
const DIR_SELECT_HINT: &str = "Select or type a directory path:";

pub type FileSelector = Picker<FileBehaviour>;

impl FileSelector {
    /// Creates new [`FileSelector`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, width: u16, initial_path: PathBuf) -> Self {
        let runtime = worker.borrow().runtime_handle().clone();
        let behaviour = FileBehaviour::new(Rc::clone(&app_data), runtime, initial_path, false);
        Picker::new_picker(app_data, Some(worker), width, behaviour).with_highlight_on_complete(true)
    }

    /// Returns the selected path and flag indicating if path already exists in the file system.
    pub fn selected_path(&self) -> (PathBuf, bool) {
        if let Some(path) = &self.behaviour().selected_path {
            (
                self.behaviour().current_path.join(normalize(path)),
                self.behaviour().current_exists && self.behaviour().selected_exists,
            )
        } else {
            (self.behaviour().current_path.clone(), self.behaviour().current_exists)
        }
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
        behaviour.current_path = path;
        behaviour.lister.reset();
        behaviour.loading = true;
    }

    /// Allows selecting files or directories for uploads, and only directories for downloads.
    pub fn set_transfer_picker(&mut self, is_download: bool) {
        self.behaviour_mut().pick_dir = true;
        self.behaviour_mut().directories_only = is_download;
        self.allow_empty(true);
    }
}

pub struct FileBehaviour {
    app_data: SharedAppData,
    lister: DirLister,
    current_path: PathBuf,
    current_exists: bool,
    selected_path: Option<String>,
    selected_exists: bool,
    loading: bool,
    spinner: Spinner,
    pick_dir: bool,
    directories_only: bool,
}

impl FileBehaviour {
    pub fn new(app_data: SharedAppData, runtime: Handle, initial_path: PathBuf, pick_dir: bool) -> Self {
        let use_plain_symbols = app_data.use_plain_symbols();

        Self {
            app_data,
            lister: DirLister::new(runtime, 100),
            current_path: initial_path,
            current_exists: false,
            selected_path: None,
            selected_exists: false,
            loading: true,
            spinner: Spinner::new(use_plain_symbols),
            pick_dir,
            directories_only: pick_dir,
        }
    }

    fn navigate_to_dir(&mut self, dir_path: PathBuf) -> bool {
        self.current_path.clone_from(&dir_path);
        self.current_exists = false;
        self.loading = self.lister.list_dir(dir_path, true);
        self.loading
    }

    fn navigate_up(&mut self) -> bool {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to_dir(parent.to_path_buf())
        } else {
            false
        }
    }

    fn process_dir_results(&mut self, patterns: &mut Select<PatternsList>) {
        let dir_icon = self.app_data.borrow().config.symbols.dir.ch;
        let back_icon = self.app_data.borrow().config.symbols.back.ch;

        let mut needs_highlight = false;
        while self.app_data.has_frame_budget()
            && let Some(result) = self.lister.try_recv()
        {
            match result {
                DirListResult::Init => {
                    self.loading = true;
                    patterns.items.clear();
                    if !patterns.value().is_empty() {
                        patterns.items.set_filter(Some(patterns.value().into()));
                    }
                },
                DirListResult::Entry(entry) => {
                    self.current_exists = true;

                    if self.directories_only && !entry.is_dir {
                        continue;
                    }

                    let item = if entry.is_dir {
                        directory_item(entry.name.clone(), dir_icon, back_icon)
                    } else {
                        PatternItem::fixed(entry.name.clone())
                    };

                    patterns.items.add_or_update(item);

                    if !self.pick_dir && entry.is_dir && entry.name == BACK_NAME && patterns.value_full().is_empty() {
                        patterns.items.highlight_item_by_name(BACK_NAME);
                    } else if !patterns.value().is_empty() {
                        needs_highlight = true;
                    }
                },
                DirListResult::Complete => {
                    self.loading = false;
                    self.current_exists = true;
                },
                DirListResult::Error(_) => {
                    self.loading = false;
                    self.current_exists = false;
                },
            }
        }

        if needs_highlight {
            patterns.highlight_item_by_filter_value();
        }
    }

    fn process_input_navigation(&mut self, patterns: &mut Select<PatternsList>) {
        let value = patterns.value_prefix();
        let dir = if value.is_empty() {
            self.current_path.clone()
        } else if Path::new(value).is_absolute() {
            PathBuf::from(value)
        } else {
            self.current_path.join(value)
        };

        if self.lister.list_dir(dir, value.is_empty()) {
            patterns.items.clear();
            if patterns.value().is_empty() {
                patterns.items.set_filter(None);
            }
        }
    }
}

impl PickerBehaviour for FileBehaviour {
    fn prompt(&self) -> Cow<'_, str> {
        let prompt_end = self.app_data.borrow().config.symbols.prompt.right;
        format!("{}{}", self.current_path.display(), prompt_end).into()
    }

    fn colors(&self) -> SelectColors {
        self.app_data.borrow().theme.colors.command_palette.clone()
    }

    #[cfg(windows)]
    fn accent_characters(&self) -> Option<&str> {
        Some("/\\")
    }

    #[cfg(not(windows))]
    fn accent_characters(&self) -> Option<&str> {
        Some("/")
    }

    #[cfg(windows)]
    fn filter_delimiters(&self) -> Vec<char> {
        vec!['\\', '/']
    }

    #[cfg(not(windows))]
    fn filter_delimiters(&self) -> Vec<char> {
        vec!['/']
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

    fn validate(&mut self, value: &str) -> Option<usize> {
        if validate_path(value, self.pick_dir) { None } else { Some(0) }
    }

    fn restores_on_cancel(&self) -> bool {
        false
    }

    fn blocks_on_error(&self) -> bool {
        false
    }

    fn navigate_into(&mut self, prefix: &str, value: &str, highlighted: Option<&str>) -> ResponseEvent {
        if let Some(highlighted) = highlighted {
            self.selected_path = Some(combine_values(prefix, highlighted));
            self.selected_exists = true;
            ResponseEvent::Accepted
        } else if value.is_empty() {
            self.selected_path = None;
            self.selected_exists = self.pick_dir;
            if self.pick_dir {
                ResponseEvent::Accepted
            } else {
                ResponseEvent::Handled
            }
        } else {
            self.selected_path = Some(combine_values(prefix, value));
            self.selected_exists = false;
            ResponseEvent::Accepted
        }
    }

    fn on_show(&mut self, patterns: &mut Select<PatternsList>) -> bool {
        self.lister.reset();
        self.lister.list_dir(self.current_path.clone(), true);
        patterns.items = PatternsList::default();

        true
    }

    fn on_reset(&mut self, patterns: &mut Select<PatternsList>) -> bool {
        if !patterns.value_prefix().is_empty() && self.navigate_to_dir(self.current_path.clone()) {
            patterns.items.clear();
        }

        patterns.reset();
        true
    }

    fn on_close(&mut self, patterns: &mut Select<PatternsList>, is_cancel: bool) -> bool {
        if is_cancel {
            return true;
        }

        if !patterns.value_full().is_empty() && patterns.has_error() {
            return false;
        }

        let dir_icon = self.app_data.borrow().config.symbols.dir.ch;
        let back_icon = self.app_data.borrow().config.symbols.back.ch;

        if let Some(item) = patterns.items.get_highlighted() {
            if item.icon().is_some_and(|i| i == dir_icon || i == back_icon) {
                if item.name() == BACK_NAME {
                    self.navigate_up();
                } else {
                    self.navigate_to_dir(self.current_path.join(normalize(patterns.value_prefix())).join(item.value()));
                }

                patterns.set_prompt(self.prompt());
                patterns.items.clear();
                patterns.reset();

                return false;
            }
        } else {
            if patterns.value() == BACK_NAME {
                self.navigate_to_dir(self.current_path.join(normalize(patterns.value_prefix()).join(BACK_NAME)));

                patterns.set_prompt(self.prompt());
                patterns.items.clear();
                patterns.reset();

                return false;
            }

            if patterns.value().is_empty() {
                if self.pick_dir && patterns.value_prefix().is_empty() {
                    return true;
                }

                if !patterns.value_prefix().is_empty() && !patterns.items.is_empty() {
                    self.navigate_to_dir(self.current_path.join(normalize(patterns.value_prefix())));

                    patterns.set_prompt(self.prompt());
                    patterns.reset();

                    if self.current_path.parent().is_some() {
                        let item = directory_item(BACK_NAME.to_owned(), dir_icon, back_icon);
                        patterns.items.add_or_update(item);
                        patterns.items.highlight_item_by_name(BACK_NAME);
                    }
                }

                return false;
            }
        }

        true
    }

    fn on_draw(&mut self, patterns: &mut Select<PatternsList>, _area: Rect) {
        self.process_dir_results(patterns);
    }

    fn has_header(&self) -> bool {
        true
    }

    fn draw_header(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect, style: Style) {
        let hint = if self.pick_dir { DIR_SELECT_HINT } else { FILE_SELECT_HINT };
        let icon = self.app_data.borrow().config.symbols.hint.ch;
        let line = format!("{} {}", if self.loading { self.spinner.tick() } else { icon }, hint);
        frame.render_widget(Paragraph::new(line).style(style), area);
    }

    fn post_process_event(&mut self, event: &TuiEvent, patterns: &mut Select<PatternsList>, _: &SharedAppData) -> ResponseEvent {
        if let TuiEvent::Key(_) = event {
            self.process_input_navigation(patterns);
        }

        ResponseEvent::NotHandled
    }
}

fn directory_item(name: String, dir_icon: char, back_icon: char) -> PatternItem {
    let is_back = name == BACK_NAME;
    let sort_value = if is_back { String::new() } else { format!("...-{name}") };
    let mut item = PatternItem::fixed(name);
    item.set_icon(Some(if is_back { back_icon } else { dir_icon }));
    item.set_sort_value(Some(sort_value));
    item
}

fn normalize(path: &str) -> PathBuf {
    Path::new(path).components().collect()
}

fn combine_values(prefix: &str, highlighted: &str) -> String {
    let mut result = String::with_capacity(prefix.len() + highlighted.len());
    result.push_str(prefix);
    result.push_str(highlighted);
    result
}

fn validate_path(path_str: &str, allow_empty: bool) -> bool {
    if allow_empty && path_str.is_empty() {
        return true;
    }

    if path_str.is_empty() || path_str.contains('\0') {
        return false;
    }

    validate_path_components(Path::new(path_str))
}

#[cfg(windows)]
fn validate_path_components(path: &Path) -> bool {
    use std::path::Prefix;

    let mut has_only_prefix = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                has_only_prefix = matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_));
            },
            Component::RootDir => {
                has_only_prefix = false;
            },
            Component::Normal(part) => {
                let Some(part_str) = part.to_str() else {
                    return false;
                };

                if part_str.len() > 255 || part_str.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')) {
                    return false;
                }
            },
            Component::CurDir | Component::ParentDir => {},
        }
    }

    !has_only_prefix
}

#[cfg(not(windows))]
fn validate_path_components(path: &Path) -> bool {
    path.components().all(|component| match component {
        Component::Normal(part) => part.to_str().is_some_and(|part_str| part_str.len() <= 255),
        _ => true,
    })
}
