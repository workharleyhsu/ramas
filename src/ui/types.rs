//! Type analysis view (`t`): category table and largest files per category.

use crossterm::event::{KeyCode, KeyEvent};

use super::app::{App, CategoryList, Overlay, TypesView};
use crate::analysis;
use crate::classify::CatId;

/// Files listed when a category is opened.
pub const CATEGORY_FILES: usize = 500;

impl App {
    /// `t`: opens the analysis of the selected node.
    pub fn open_types(&mut self) {
        self.overlay = Overlay::Types(TypesView {
            node: self.selected(),
            cursor: 0,
            list: None,
        });
    }

    /// Categories with files under the analyzed node, largest first.
    pub fn type_rows(&mut self) -> Vec<CatId> {
        self.selected_stats().ranked()
    }

    pub(super) fn types_key(&mut self, key: KeyEvent) {
        let rows = self.type_rows();
        let Overlay::Types(view) = &mut self.overlay else {
            return;
        };
        match &mut view.list {
            None => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('t') => {
                    self.overlay = Overlay::None
                }
                KeyCode::Up | KeyCode::Char('k') => view.cursor = view.cursor.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    view.cursor = (view.cursor + 1).min(rows.len().saturating_sub(1))
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(&cat) = rows.get(view.cursor) {
                        let node = view.node;
                        let files = analysis::top_in_category(
                            &self.tree,
                            node,
                            self.apparent,
                            &self.classifier,
                            cat,
                            CATEGORY_FILES,
                        );
                        if let Overlay::Types(v) = &mut self.overlay {
                            v.list = Some(CategoryList {
                                cat,
                                files,
                                cursor: 0,
                            });
                        }
                    }
                }
                _ => {}
            },
            Some(list) => match key.code {
                KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => view.list = None,
                KeyCode::Up | KeyCode::Char('k') => list.cursor = list.cursor.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    list.cursor = (list.cursor + 1).min(list.files.len().saturating_sub(1))
                }
                KeyCode::PageDown => {
                    list.cursor = (list.cursor + 10).min(list.files.len().saturating_sub(1))
                }
                KeyCode::PageUp => list.cursor = list.cursor.saturating_sub(10),
                KeyCode::Char(' ') => {
                    if let Some(&id) = list.files.get(list.cursor) {
                        list.cursor = (list.cursor + 1).min(list.files.len().saturating_sub(1));
                        let toggle = self.marks.toggle(&self.tree, id);
                        if toggle == crate::actions::marks::Toggle::Root {
                            self.message = Some(self.s.cannot_mark_root.into());
                        }
                    }
                }
                KeyCode::Char('d') => self.open_delete(),
                KeyCode::Char('q') => self.overlay = Overlay::None,
                _ => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use crossterm::event::KeyCode;

    use crate::tree::test_util::{dir, file};
    use crate::tree::{NodeId, Tree};
    use crate::ui::app::test_util::{key, press, settings};
    use crate::ui::app::{App, Button, Overlay};
    use crate::ui::render::test_util::render;

    fn app() -> App {
        let mut t = Tree::new("/r", dir());
        let m = t.add_child(NodeId::ROOT, OsStr::new("media"), &dir());
        t.add_child(m, OsStr::new("film.mkv"), &file(9000));
        t.add_child(m, OsStr::new("clip.mp4"), &file(1000));
        t.add_child(m, OsStr::new("pic.jpg"), &file(300));
        t.add_child(NodeId::ROOT, OsStr::new("song.flac"), &file(2000));
        t.add_child(NodeId::ROOT, OsStr::new("main.rs"), &file(50));
        t.aggregate();
        App::new(t, settings())
    }

    fn view(app: &App) -> &crate::ui::app::TypesView {
        match &app.overlay {
            Overlay::Types(v) => v,
            other => panic!("not in types view: {other:?}"),
        }
    }

    #[test]
    fn table_lists_categories_by_size() {
        let mut app = app();
        press(&mut app, "t");
        let rows = app.type_rows();
        let names: Vec<_> = rows
            .iter()
            .map(|&c| app.classifier.name(c).to_owned())
            .collect();
        assert_eq!(names, ["video", "audio", "image", "code"]);
        let body = render(&mut app, 100, 30).join("\n");
        assert!(body.contains("Types in /r"), "{body}");
        assert!(body.contains("video") && body.contains("9.8 KiB"), "{body}");
        assert!(body.contains("audio"), "{body}");
    }

    #[test]
    fn enter_lists_heaviest_files_of_category() {
        let mut app = app();
        press(&mut app, "t");
        app.on_key(key(KeyCode::Enter));
        let list = view(&app).list.clone().unwrap();
        let names: Vec<_> = list
            .files
            .iter()
            .map(|&i| app.tree.name(i).to_owned())
            .collect();
        assert_eq!(names, ["film.mkv", "clip.mp4"]);
        let body = render(&mut app, 100, 30).join("\n");
        assert!(body.contains("media/film.mkv"), "{body}");
        app.on_key(key(KeyCode::Esc));
        assert!(view(&app).list.is_none());
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.overlay, Overlay::None);
    }

    #[test]
    fn files_in_category_are_markable_and_deletable() {
        let mut app = app();
        press(&mut app, "t");
        app.on_key(key(KeyCode::Enter));
        press(&mut app, " ");
        let film = app.tree.find(Path::new("media/film.mkv")).unwrap();
        assert!(app.marks.is_marked(film));
        press(&mut app, "d");
        assert_eq!(
            app.overlay,
            Overlay::Confirm {
                focus: Button::Cancel
            }
        );
    }

    #[test]
    fn analysis_of_subdirectory() {
        let mut app = app();
        press(&mut app, "j"); // media (largest)
        press(&mut app, "t");
        let rows = app.type_rows();
        let names: Vec<_> = rows
            .iter()
            .map(|&c| app.classifier.name(c).to_owned())
            .collect();
        assert_eq!(names, ["video", "image"]);
    }
}
