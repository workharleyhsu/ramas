//! Search (`/`, `n`, `N`) and filters (`f`) in the TUI.

use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent};

use super::app::{App, FilterForm, Overlay, SearchState};
use super::format;
use crate::filter::Filter;
use crate::search::Finder;
use crate::tree::NodeId;

/// Number of best hits whose branches are expanded when a search is kept.
pub const EXPAND_HITS: usize = 100;

impl App {
    /// `/`: starts typing a query.
    pub fn open_search(&mut self) {
        let input = self
            .search
            .as_ref()
            .map(|s| s.query.clone())
            .unwrap_or_default();
        self.overlay = Overlay::Search {
            input,
            origin: self.selected(),
        };
    }

    pub(super) fn search_key(&mut self, key: KeyEvent) {
        let Overlay::Search { input, origin } = &mut self.overlay else {
            return;
        };
        let origin = *origin;
        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                self.search = None;
                self.reveal(origin);
            }
            KeyCode::Enter => {
                self.overlay = Overlay::None;
                self.expand_hits();
            }
            KeyCode::Backspace => {
                input.pop();
                let q = input.clone();
                self.run_search(&q);
            }
            KeyCode::Char(c) => {
                input.push(c);
                let q = input.clone();
                self.run_search(&q);
            }
            _ => {}
        }
    }

    /// Recomputes hits for `query` and jumps to the best one.
    pub fn run_search(&mut self, query: &str) {
        if query.is_empty() {
            self.search = None;
            return;
        }
        let hits: Vec<NodeId> = Finder::new(query)
            .find_all(&self.tree)
            .into_iter()
            .filter(|&id| self.is_shown(id))
            .collect();
        let hit_set: HashSet<NodeId> = hits.iter().copied().collect();
        let first = hits.first().copied();
        self.search = Some(SearchState {
            query: query.to_owned(),
            hits,
            hit_set,
            idx: 0,
        });
        if let Some(id) = first {
            self.reveal(id);
        }
    }

    /// Expands the branches of the best hits and keeps the current one selected.
    fn expand_hits(&mut self) {
        let Some(search) = &self.search else {
            return;
        };
        let current = search.hits.get(search.idx).copied();
        let ids: Vec<NodeId> = search.hits.iter().take(EXPAND_HITS).copied().collect();
        self.expanded.resize(self.tree.len(), false);
        for id in ids {
            let ancestors: Vec<NodeId> = self.tree.ancestors(id).collect();
            for a in ancestors {
                self.expanded[a.index()] = true;
            }
        }
        self.rebuild_rows(current.unwrap_or(self.selected()));
    }

    /// `n` / `N`: next or previous hit, wrapping around.
    pub fn next_hit(&mut self, step: i64) {
        let Some(search) = &mut self.search else {
            return;
        };
        if search.hits.is_empty() {
            self.message = Some(self.s.search_none.into());
            return;
        }
        let n = search.hits.len() as i64;
        search.idx = (search.idx as i64 + step).rem_euclid(n) as usize;
        let id = search.hits[search.idx];
        self.reveal(id);
    }

    /// Whether `id` is a search hit.
    pub fn is_hit(&self, id: NodeId) -> bool {
        self.search
            .as_ref()
            .is_some_and(|s| s.hit_set.contains(&id))
    }

    /// `f`: opens the filter form prefilled with the active filter.
    pub fn open_filters(&mut self) {
        let f = &self.filter;
        self.overlay = Overlay::Filters(FilterForm {
            fields: [
                f.min_size
                    .map(format::size)
                    .unwrap_or_default()
                    .replace(' ', ""),
                f.exts.join(", "),
                f.older_than_days.map(|d| d.to_string()).unwrap_or_default(),
            ],
            hide: f.hide,
            focus: 0,
            error: false,
        });
    }

    pub(super) fn filter_key(&mut self, key: KeyEvent) {
        let Overlay::Filters(form) = &mut self.overlay else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.overlay = Overlay::None,
            KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % 4,
            KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + 3) % 4,
            KeyCode::Delete => *form = FilterForm::default(),
            KeyCode::Char(' ') if form.focus == 3 => form.hide = !form.hide,
            KeyCode::Backspace if form.focus < 3 => {
                form.fields[form.focus].pop();
            }
            KeyCode::Char(c) if form.focus < 3 => form.fields[form.focus].push(c),
            KeyCode::Enter => {
                let form = form.clone();
                match parse_form(&form) {
                    Some(filter) => {
                        self.overlay = Overlay::None;
                        self.set_filter(filter);
                    }
                    None => {
                        if let Overlay::Filters(f) = &mut self.overlay {
                            f.error = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Applies a filter and refreshes the view.
    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
        self.refilter();
        let keep = self.selected();
        self.rebuild_rows(keep);
    }

    /// Recomputes match flags for the active filter.
    pub fn refilter(&mut self) {
        self.filter_match = if self.filter.is_empty() {
            None
        } else {
            Some(self.filter.evaluate(&self.tree, self.apparent, now()))
        };
    }
}

/// Converts the form into a filter; `None` if a field is invalid.
pub fn parse_form(form: &FilterForm) -> Option<Filter> {
    let [size, exts, age] = &form.fields;
    let min_size = match size.trim() {
        "" => None,
        s => Some(format::parse_size(s)?),
    };
    let older_than_days = match age.trim() {
        "" => None,
        s => Some(s.parse().ok()?),
    };
    Some(Filter {
        min_size,
        exts: Filter::parse_exts(exts),
        older_than_days,
        hide: form.hide,
    })
}

fn now() -> i64 {
    crate::export::now()
}

#[cfg(test)]
mod tests;
