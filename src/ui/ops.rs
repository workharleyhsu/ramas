//! Marking, the delete flow and rescans.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use crossterm::event::{KeyCode, KeyEvent};

use super::app::{App, Button, DeleteSummary, Overlay};
use super::job::ScanJob;
use crate::actions::delete::{FsRemover, Guard, Method, Outcome, Target, delete_all};
use crate::actions::marks::Toggle;
use crate::i18n::fill;
use crate::scan::ScanError;
use crate::tree::{Kind, NodeId};

/// Delete running on a worker thread.
pub struct DeleteJob {
    rx: Receiver<Vec<Outcome>>,
}

impl App {
    /// `Space`: marks or unmarks the selected node, then moves down.
    pub fn toggle_mark(&mut self) {
        let id = self.selected();
        match self.marks.toggle(&self.tree, id) {
            Toggle::Root => self.message = Some(self.s.cannot_mark_root.into()),
            Toggle::Covered => {}
            Toggle::Marked { .. } | Toggle::Unmarked => self.move_by(1),
        }
    }

    /// Why deleting is currently impossible, if it is.
    pub fn delete_blocked(&self) -> Option<&'static str> {
        if self.read_only {
            Some(self.s.delete_disabled_ro)
        } else if self.imported {
            Some(self.s.delete_disabled_import)
        } else if self.marks.is_empty() {
            Some(self.s.nothing_marked)
        } else {
            None
        }
    }

    /// `d`: opens the confirmation dialog with the focus on Cancel.
    pub fn open_delete(&mut self) {
        match self.delete_blocked() {
            Some(reason) => self.message = Some(reason.into()),
            None => {
                self.overlay = Overlay::Confirm {
                    focus: Button::Cancel,
                }
            }
        }
    }

    pub(super) fn confirm_key(&mut self, key: KeyEvent) {
        let Overlay::Confirm { focus } = self.overlay.clone() else {
            return;
        };
        const ORDER: [Button; 3] = [Button::Trash, Button::Permanent, Button::Cancel];
        let i = ORDER.iter().position(|&b| b == focus).unwrap_or(2);
        match key.code {
            KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
                self.overlay = Overlay::Confirm {
                    focus: ORDER[(i + 2) % 3],
                }
            }
            KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => {
                self.overlay = Overlay::Confirm {
                    focus: ORDER[(i + 1) % 3],
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => self.overlay = Overlay::None,
            KeyCode::Enter => match focus {
                Button::Cancel => self.overlay = Overlay::None,
                Button::Trash => self.start_delete(Method::Trash),
                Button::Permanent => {
                    self.overlay = Overlay::TypeConfirm {
                        input: String::new(),
                    }
                }
            },
            _ => {}
        }
    }

    pub(super) fn type_confirm_key(&mut self, key: KeyEvent) {
        let Overlay::TypeConfirm { input } = &mut self.overlay else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.overlay = Overlay::None,
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) if input.chars().count() < 32 => input.push(c),
            KeyCode::Enter => {
                if input.trim() == self.s.confirm_word {
                    self.start_delete(Method::Permanent);
                } else {
                    input.clear();
                }
            }
            _ => {}
        }
    }

    /// Validates and deletes every marked node on a worker thread.
    pub fn start_delete(&mut self, method: Method) {
        self.overlay = Overlay::None;
        if let Some(reason) = self.delete_blocked() {
            self.message = Some(reason.into());
            return;
        }
        let targets: Vec<Target> = self
            .marks
            .ids()
            .into_iter()
            .map(|id| Target::of(&self.tree, id, false))
            .collect();
        let guard = match Guard::new(self.tree.root_path()) {
            Ok(g) => g,
            Err(e) => {
                self.overlay = Overlay::Summary(DeleteSummary {
                    failed: targets
                        .into_iter()
                        .map(|t| (t.path, e.to_string()))
                        .collect(),
                    ..DeleteSummary::default()
                });
                return;
            }
        };
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(delete_all(&targets, method, &guard, &FsRemover));
        });
        self.delete_job = Some(DeleteJob { rx });
    }

    pub(super) fn poll_delete(&mut self) {
        let Some(job) = &self.delete_job else {
            return;
        };
        let outcomes = match job.rx.try_recv() {
            Ok(o) => o,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Vec::new(),
        };
        self.delete_job = None;
        self.apply_outcomes(outcomes);
    }

    /// Removes deleted nodes from the tree (ancestors updated, no rescan)
    /// and shows the summary.
    fn apply_outcomes(&mut self, outcomes: Vec<Outcome>) {
        let keep = self.selected();
        let mut summary = DeleteSummary::default();
        for o in outcomes {
            match o.result {
                Ok(()) => {
                    if let Some(r) = self.tree.remove_subtree(o.target.node) {
                        summary.freed += r.disk;
                        summary.removed += 1;
                    }
                    self.marks.unmark(o.target.node);
                }
                Err(e) => summary.failed.push((o.target.path, e.to_string())),
            }
        }
        self.marks.prune(&self.tree);
        self.tree_changed(keep);
        self.overlay = Overlay::Summary(summary);
    }

    /// `r`: rescans the selected directory in the background.
    pub fn rescan_selected(&mut self) {
        let id = self.selected();
        if self.tree.node(id).kind != Kind::Dir {
            self.message = Some(self.s.rescan_not_dir.into());
            return;
        }
        let path = self.tree.path(id);
        self.job = Some(ScanJob::start(path, self.scan_opts.clone(), Some(id)));
    }

    pub(super) fn cancel_rescan(&mut self) {
        if let Some(job) = &self.job {
            job.cancel();
        }
    }

    pub(super) fn poll_rescan(&mut self) {
        let Some(job) = &self.job else {
            return;
        };
        let Some(result) = job.try_result() else {
            return;
        };
        let target = job.target.unwrap_or(NodeId::ROOT);
        self.job = None;
        match result {
            Ok(sub) => {
                self.tree.replace_subtree(target, &sub);
                if target == NodeId::ROOT {
                    self.imported = false;
                    self.scanned_at = crate::export::now();
                }
                self.marks.prune(&self.tree);
                self.tree_changed(target);
            }
            Err(ScanError::Cancelled) => {}
            Err(e) => self.message = Some(e.to_string()),
        }
    }

    /// Re-sorts, refreshes caches and rows after a structural change.
    fn tree_changed(&mut self, keep: NodeId) {
        self.generation += 1;
        self.search = None;
        self.tree.sort(self.sort, self.apparent);
        self.refilter();
        if let Some(diff) = &mut self.diff {
            diff.recompute(&self.tree);
        }
        self.refresh_free();
        self.rebuild_rows(keep);
    }

    /// `e`: exports the tree to a timestamped JSON file in the current
    /// directory.
    pub fn export_here(&mut self) {
        let path = self
            .export_dir
            .join(export_name(&self.tree, crate::export::now()));
        self.message = Some(
            match crate::export::write_file(&self.tree, self.scanned_at, &path) {
                Ok(()) => fill(self.s.exported, &[("path", &path.display().to_string())]),
                Err(e) => fill(self.s.export_failed, &[("err", &format!("{e:#}"))]),
            },
        );
    }

    /// Status line text for marks.
    pub fn marked_text(&self) -> String {
        let (n, bytes) = self.marked_summary();
        fill(
            self.s.marked,
            &[("n", &n.to_string()), ("size", &super::format::size(bytes))],
        )
    }
}

/// `ramas-<root name>-YYYYmmdd-HHMMSS.json`.
pub fn export_name(tree: &crate::tree::Tree, now: i64) -> String {
    let base = tree
        .root_path()
        .file_name()
        .map_or_else(|| "root".to_owned(), |n| n.to_string_lossy().into_owned());
    let base: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stamp = jiff::Timestamp::from_second(now)
        .map(|t| {
            t.to_zoned_datetime(jiff::tz::TimeZone::system())
                .strftime("%Y%m%d-%H%M%S")
                .to_string()
        })
        .unwrap_or_else(|_| now.to_string());
    format!("ramas-{base}-{stamp}.json")
}

#[cfg(test)]
mod tests;
