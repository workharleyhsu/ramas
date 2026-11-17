//! Application state and input handling, independent of the terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use super::icons::Icons;
use super::job::ScanJob;
use super::ops::DeleteJob;
use super::rows::{self, Row};
use super::theme::Theme;
use crate::actions::marks::Marks;
use crate::analysis::{self, Stats};
use crate::classify::Classifier;
use crate::filter::Filter;
use crate::i18n::Strings;
use crate::scan::ScanOptions;
use crate::tree::{NodeId, SortKey, Tree};

/// Startup settings resolved from CLI and config.
#[derive(Debug, Clone)]
pub struct Settings {
    pub apparent: bool,
    pub read_only: bool,
    pub imported: bool,
    pub strings: &'static Strings,
    pub theme: Theme,
    pub icons: Icons,
    pub scan_opts: ScanOptions,
    pub classifier: Classifier,
}

/// Modal layer drawn above the tree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Overlay {
    /// No overlay: keys go to the tree.
    #[default]
    None,
    /// Key help.
    Help,
    /// Delete confirmation with three buttons.
    Confirm { focus: Button },
    /// Second confirmation for permanent delete: the word must be typed.
    TypeConfirm { input: String },
    /// Result of a delete run.
    Summary(DeleteSummary),
    /// Incremental search input; `origin` is restored on Esc.
    Search { input: String, origin: NodeId },
    /// Filter form.
    Filters(FilterForm),
    /// Per-category analysis of a node.
    Types(TypesView),
}

/// State of the type analysis view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypesView {
    /// Node analyzed.
    pub node: NodeId,
    /// Selected row of the category table.
    pub cursor: usize,
    /// Largest files of the opened category.
    pub list: Option<CategoryList>,
}

/// Largest files of one category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryList {
    pub cat: crate::classify::CatId,
    pub files: Vec<NodeId>,
    pub cursor: usize,
}

/// Editable state of the filter panel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilterForm {
    /// Min size, extensions, age in days.
    pub fields: [String; 3],
    /// Hide instead of dim.
    pub hide: bool,
    /// Focused row: 0..=2 text fields, 3 the mode toggle.
    pub focus: usize,
    /// Validation error of the last apply.
    pub error: bool,
}

/// Buttons of the delete dialog, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Trash,
    Permanent,
    Cancel,
}

/// What a delete run achieved.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeleteSummary {
    /// Bytes freed by successful removals.
    pub freed: u64,
    /// Number of items removed.
    pub removed: usize,
    /// Paths that failed, with the reason.
    pub failed: Vec<(std::path::PathBuf, String)>,
}

/// Screen areas from the last render, used for mouse hit testing.
#[derive(Debug, Clone, Copy, Default)]
pub struct Areas {
    /// Inner area of the tree pane (rows only).
    pub tree: Rect,
}

/// Whole TUI state.
pub struct App {
    pub tree: Tree,
    pub expanded: Vec<bool>,
    pub rows: Vec<Row>,
    pub cursor: usize,
    pub offset: usize,
    pub sort: SortKey,
    pub apparent: bool,
    pub show_panel: bool,
    pub overlay: Overlay,
    pub quit: bool,
    pub s: &'static Strings,
    pub theme: Theme,
    pub icons: Icons,
    pub message: Option<String>,
    pub areas: Areas,
    pub read_only: bool,
    pub imported: bool,
    pub scan_opts: ScanOptions,
    pub classifier: Classifier,
    /// Bumped on every structural change of the tree.
    pub generation: u64,
    /// Free bytes on the scanned filesystem, when known.
    pub free: Option<u64>,
    stats_cache: Option<((NodeId, bool, u64), Stats)>,
    pub marks: Marks,
    pub job: Option<ScanJob>,
    pub delete_job: Option<DeleteJob>,
    pub search: Option<SearchState>,
    pub filter: Filter,
    /// Match flag per node for `filter`; `None` when no filter is active.
    pub filter_match: Option<Vec<bool>>,
}

/// Committed or in-progress search.
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub hits: Vec<NodeId>,
    pub hit_set: std::collections::HashSet<NodeId>,
    pub idx: usize,
}

/// Number of files listed in the detail panel.
pub const TOP_FILES: usize = 10;

impl App {
    /// Builds the state for a scanned tree: sorted by size, root expanded.
    pub fn new(mut tree: Tree, settings: Settings) -> Self {
        tree.sort(SortKey::Size, settings.apparent);
        let mut expanded = vec![false; tree.len()];
        expanded[0] = true;
        let mut app = App {
            tree,
            expanded,
            rows: Vec::new(),
            cursor: 0,
            offset: 0,
            sort: SortKey::Size,
            apparent: settings.apparent,
            show_panel: true,
            overlay: Overlay::None,
            quit: false,
            s: settings.strings,
            theme: settings.theme,
            icons: settings.icons,
            message: None,
            areas: Areas::default(),
            read_only: settings.read_only,
            imported: settings.imported,
            scan_opts: settings.scan_opts,
            classifier: settings.classifier,
            generation: 0,
            free: None,
            stats_cache: None,
            marks: Marks::default(),
            job: None,
            delete_job: None,
            search: None,
            filter: Filter::default(),
            filter_match: None,
        };
        app.refresh_free();
        app.rebuild_rows(NodeId::ROOT);
        app
    }

    /// Re-reads the free space of the scanned filesystem.
    pub fn refresh_free(&mut self) {
        self.free = rustix::fs::statvfs(self.tree.root_path())
            .ok()
            .map(|v| v.f_bavail.saturating_mul(v.f_frsize));
    }

    /// Statistics of the selected node, cached until selection, metric or
    /// tree change.
    pub fn selected_stats(&mut self) -> &Stats {
        let key = (self.selected(), self.apparent, self.generation);
        let entry = match self.stats_cache.take() {
            Some((k, s)) if k == key => (k, s),
            _ => {
                let s = analysis::stats(&self.tree, key.0, key.1, &self.classifier, TOP_FILES);
                (key, s)
            }
        };
        &self.stats_cache.insert(entry).1
    }

    /// Number of marked items and the bytes they free.
    pub fn marked_summary(&self) -> (usize, u64) {
        (
            self.marks.len(),
            self.marks.total(&self.tree, self.apparent),
        )
    }

    /// Node under the cursor.
    pub fn selected(&self) -> NodeId {
        self.rows.get(self.cursor).map_or(NodeId::ROOT, |r| r.id)
    }

    /// Whether a node passes the active filter.
    pub fn matches_filter(&self, id: NodeId) -> bool {
        self.filter_match
            .as_ref()
            .is_none_or(|m| m.get(id.index()).copied().unwrap_or(true))
    }

    /// Whether a node gets a row (non-matching nodes may be hidden).
    pub fn is_shown(&self, id: NodeId) -> bool {
        !self.filter.hide || id == NodeId::ROOT || self.matches_filter(id)
    }

    /// Whether a background job (rescan, delete) is running.
    pub fn busy(&self) -> bool {
        self.job.is_some() || self.delete_job.is_some()
    }

    /// Periodic work between events: collects finished jobs.
    pub fn tick(&mut self) {
        self.poll_rescan();
        self.poll_delete();
    }

    /// Handles a mouse event.
    pub fn on_mouse(&mut self, _ev: crossterm::event::MouseEvent) {}

    /// Whether a directory is expanded.
    pub fn is_open(&self, id: NodeId) -> bool {
        rows::is_open(&self.expanded, id)
    }

    /// Rows that fit in the tree pane.
    pub fn page(&self) -> usize {
        usize::from(self.areas.tree.height).max(1)
    }

    /// Recomputes all rows and puts the cursor back on `keep` (or the
    /// closest visible ancestor).
    pub fn rebuild_rows(&mut self, keep: NodeId) {
        self.expanded.resize(self.tree.len(), false);
        let rows = rows::build(&self.tree, &self.expanded, &|id| self.is_shown(id));
        self.rows = rows;
        self.cursor = self.row_of(keep).unwrap_or(0);
    }

    /// Row index of a node, walking up to the nearest shown ancestor.
    fn row_of(&self, id: NodeId) -> Option<usize> {
        std::iter::once(id)
            .chain(self.tree.ancestors(id))
            .find_map(|n| self.rows.iter().position(|r| r.id == n))
    }

    /// Handles a key press.
    pub fn on_key(&mut self, key: KeyEvent) {
        self.message = None;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if self.job.is_some() {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.cancel_rescan();
            }
            return;
        }
        if self.delete_job.is_some() {
            return;
        }
        match &self.overlay {
            Overlay::None => self.browse_key(key),
            Overlay::Help => self.overlay = Overlay::None,
            Overlay::Confirm { .. } => self.confirm_key(key),
            Overlay::TypeConfirm { .. } => self.type_confirm_key(key),
            Overlay::Search { .. } => self.search_key(key),
            Overlay::Filters(_) => self.filter_key(key),
            Overlay::Types(_) => self.types_key(key),
            Overlay::Summary(_) => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
                    self.overlay = Overlay::None;
                }
            }
        }
    }

    fn browse_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::PageUp => self.move_by(-(self.page() as i64)),
            KeyCode::PageDown => self.move_by(self.page() as i64),
            KeyCode::Home | KeyCode::Char('g') => self.move_to(0),
            KeyCode::End | KeyCode::Char('G') => self.move_to(self.rows.len().saturating_sub(1)),
            KeyCode::Right | KeyCode::Char('l') => self.expand_or_enter(),
            KeyCode::Left | KeyCode::Char('h') => self.collapse_or_parent(),
            KeyCode::Enter => self.toggle(),
            KeyCode::Char('s') => self.cycle_sort(),
            KeyCode::Char('a') => self.toggle_apparent(),
            KeyCode::Char('p') => self.show_panel = !self.show_panel,
            KeyCode::Char('?') => self.overlay = Overlay::Help,
            KeyCode::Char(' ') => self.toggle_mark(),
            KeyCode::Char('u') => self.marks.clear(),
            KeyCode::Char('d') => self.open_delete(),
            KeyCode::Char('r') => self.rescan_selected(),
            KeyCode::Char('/') => self.open_search(),
            KeyCode::Char('n') => self.next_hit(1),
            KeyCode::Char('N') => self.next_hit(-1),
            KeyCode::Char('f') => self.open_filters(),
            KeyCode::Char('t') => self.open_types(),
            KeyCode::Esc => self.search = None,
            KeyCode::Char('q') => self.quit = true,
            _ => {}
        }
    }

    /// Moves the cursor by `delta` rows, clamped.
    pub fn move_by(&mut self, delta: i64) {
        let max = self.rows.len().saturating_sub(1) as i64;
        let target = (self.cursor as i64 + delta).clamp(0, max);
        self.move_to(target as usize);
    }

    /// Puts the cursor on row `i` (clamped).
    pub fn move_to(&mut self, i: usize) {
        self.cursor = i.min(self.rows.len().saturating_sub(1));
    }

    /// `→`: expands a closed directory, or steps into an open one.
    pub fn expand_or_enter(&mut self) {
        let id = self.selected();
        if !self.tree.has_children(id) {
            return;
        }
        if self.is_open(id) {
            if self
                .rows
                .get(self.cursor + 1)
                .is_some_and(|r| r.depth > self.rows[self.cursor].depth)
            {
                self.cursor += 1;
            }
        } else {
            self.expand(self.cursor);
        }
    }

    /// `←`: collapses an open directory, or moves to the parent.
    pub fn collapse_or_parent(&mut self) {
        let id = self.selected();
        if self.is_open(id) && self.tree.has_children(id) && id != NodeId::ROOT {
            self.collapse(self.cursor);
        } else if let Some(p) = self.tree.parent(id)
            && let Some(i) = self.rows[..self.cursor].iter().rposition(|r| r.id == p)
        {
            self.cursor = i;
        }
    }

    /// `Enter`: toggles the directory under the cursor.
    pub fn toggle(&mut self) {
        let id = self.selected();
        if !self.tree.has_children(id) {
            return;
        }
        if self.is_open(id) {
            self.collapse(self.cursor);
        } else {
            self.expand(self.cursor);
        }
    }

    /// Expands row `i`, inserting its visible descendants below it.
    pub fn expand(&mut self, i: usize) {
        let Some(row) = self.rows.get(i).copied() else {
            return;
        };
        self.expanded.resize(self.tree.len(), false);
        self.expanded[row.id.index()] = true;
        let mut sub = Vec::new();
        rows::descendants(
            &self.tree,
            row.id,
            row.depth + 1,
            &self.expanded,
            &|id| self.is_shown(id),
            &mut sub,
        );
        self.rows.splice(i + 1..i + 1, sub);
    }

    /// Collapses row `i`, removing its descendants from the view.
    pub fn collapse(&mut self, i: usize) {
        let Some(row) = self.rows.get(i).copied() else {
            return;
        };
        self.expanded[row.id.index()] = false;
        let end = rows::subtree_end(&self.rows, i);
        self.rows.drain(i + 1..end);
        if self.cursor > i && self.cursor < end {
            self.cursor = i;
        } else if self.cursor >= end {
            self.cursor -= end - i - 1;
        }
    }

    /// Expands every ancestor of `id` and selects it.
    pub fn reveal(&mut self, id: NodeId) {
        self.expanded.resize(self.tree.len(), false);
        for a in self.tree.ancestors(id).collect::<Vec<_>>() {
            self.expanded[a.index()] = true;
        }
        self.rebuild_rows(id);
    }

    fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.resort();
    }

    fn toggle_apparent(&mut self) {
        self.apparent = !self.apparent;
        self.refilter();
        if self.sort == SortKey::Size {
            self.resort();
        } else {
            self.rebuild_rows(self.selected());
        }
    }

    /// Re-sorts the tree with the current key and keeps the selection.
    pub fn resort(&mut self) {
        let keep = self.selected();
        self.tree.sort(self.sort, self.apparent);
        self.rebuild_rows(keep);
    }

    /// Label of the current sort key.
    pub fn sort_label(&self) -> &'static str {
        match self.sort {
            SortKey::Size => self.s.sort_size,
            SortKey::Name => self.s.sort_name,
            SortKey::Mtime => self.s.sort_mtime,
            SortKey::Items => self.s.sort_items,
        }
    }

    /// Size of a node under the current metric.
    pub fn size_of(&self, id: NodeId) -> u64 {
        self.tree.node(id).size(self.apparent)
    }

    /// Keeps the cursor inside the scrolled window of `height` rows.
    pub fn fix_offset(&mut self, height: usize) {
        let height = height.max(1);
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + height {
            self.offset = self.cursor + 1 - height;
        }
        let max_offset = self.rows.len().saturating_sub(height);
        self.offset = self.offset.min(max_offset);
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;
    use crate::cli::IconMode;

    pub fn settings() -> Settings {
        Settings {
            apparent: false,
            read_only: false,
            imported: false,
            strings: &crate::i18n::EN,
            theme: Theme::dark(),
            icons: Icons {
                mode: IconMode::Unicode,
            },
            scan_opts: ScanOptions::default(),
            classifier: Classifier::default(),
        }
    }

    pub fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    pub fn press(app: &mut App, keys: &str) {
        for c in keys.chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
    }

    pub fn app() -> App {
        App::new(crate::tree::test_util::sample(), settings())
    }

    pub fn name(app: &App) -> String {
        app.tree.name(app.selected()).to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;

    #[test]
    fn starts_on_root_with_children_visible_by_size() {
        let app = app();
        assert_eq!(app.selected(), NodeId::ROOT);
        let names: Vec<_> = app
            .rows
            .iter()
            .skip(1)
            .map(|r| app.tree.name(r.id).to_owned())
            .collect();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn arrows_and_vim_keys_move() {
        let mut app = app();
        app.on_key(key(KeyCode::Down));
        assert_eq!(name(&app), "a");
        press(&mut app, "jj");
        assert_eq!(name(&app), "c");
        press(&mut app, "j");
        assert_eq!(name(&app), "c", "clamped at the end");
        press(&mut app, "k");
        assert_eq!(name(&app), "b");
        app.on_key(key(KeyCode::Home));
        assert_eq!(app.cursor, 0);
        app.on_key(key(KeyCode::End));
        assert_eq!(name(&app), "c");
    }

    #[test]
    fn right_expands_then_enters_left_collapses_then_parent() {
        let mut app = app();
        press(&mut app, "j");
        app.on_key(key(KeyCode::Right));
        assert_eq!(app.rows.len(), 6);
        assert_eq!(name(&app), "a");
        app.on_key(key(KeyCode::Right));
        assert_eq!(name(&app), "y", "largest child first");
        app.on_key(key(KeyCode::Left));
        assert_eq!(name(&app), "a", "file: go to parent");
        app.on_key(key(KeyCode::Left));
        assert_eq!(app.rows.len(), 4, "collapsed");
        app.on_key(key(KeyCode::Left));
        assert_eq!(app.selected(), NodeId::ROOT);
    }

    #[test]
    fn enter_toggles_and_hl_work() {
        let mut app = app();
        press(&mut app, "j");
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.rows.len(), 6);
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.rows.len(), 4);
        press(&mut app, "ll");
        assert_eq!(name(&app), "y");
        press(&mut app, "hh");
        assert_eq!(app.rows.len(), 4);
    }

    #[test]
    fn collapse_keeps_cursor_below_consistent() {
        let mut app = app();
        press(&mut app, "jl"); // expand a: root a y x b c
        app.move_to(4); // b
        app.collapse(1);
        assert_eq!(name(&app), "b");
    }

    #[test]
    fn page_keys_move_by_page() {
        let mut app = app();
        press(&mut app, "jl");
        app.areas.tree.height = 2;
        app.on_key(key(KeyCode::PageDown));
        assert_eq!(app.cursor, 3);
        app.on_key(key(KeyCode::PageUp));
        assert_eq!(app.cursor, 1);
    }

    #[test]
    fn sort_cycles_and_keeps_selection() {
        let mut app = app();
        press(&mut app, "jj"); // b
        press(&mut app, "s");
        assert_eq!(app.sort, SortKey::Name);
        assert_eq!(name(&app), "b");
        let names: Vec<_> = app
            .rows
            .iter()
            .skip(1)
            .map(|r| app.tree.name(r.id).to_owned())
            .collect();
        assert_eq!(names, ["a", "b", "c"]);
        press(&mut app, "sss");
        assert_eq!(app.sort, SortKey::Size);
    }

    #[test]
    fn apparent_toggle_and_quit() {
        let mut app = app();
        press(&mut app, "a");
        assert!(app.apparent);
        press(&mut app, "q");
        assert!(app.quit);
    }

    #[test]
    fn offset_follows_cursor() {
        let mut app = app();
        press(&mut app, "jl");
        app.move_to(5);
        app.fix_offset(3);
        assert_eq!(app.offset, 3);
        app.move_to(0);
        app.fix_offset(3);
        assert_eq!(app.offset, 0);
    }
}
