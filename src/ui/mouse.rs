//! Mouse input: click selects, double click toggles, wheel scrolls.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use super::app::{App, Overlay};

/// Max delay between the two clicks of a double click.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// Rows moved per wheel step.
pub const SCROLL_STEP: usize = 3;

impl App {
    /// Handles a mouse event at the current time.
    pub fn on_mouse(&mut self, ev: MouseEvent) {
        self.on_mouse_at(ev, Instant::now());
    }

    /// Handles a mouse event at `now` (testable clock).
    pub fn on_mouse_at(&mut self, ev: MouseEvent, now: Instant) {
        if self.busy() {
            return;
        }
        match &self.overlay {
            Overlay::None => {}
            Overlay::Types(_) => {
                // The wheel moves the cursor of the analysis view.
                let code = match ev.kind {
                    MouseEventKind::ScrollDown => KeyCode::Down,
                    MouseEventKind::ScrollUp => KeyCode::Up,
                    _ => return,
                };
                self.types_key(KeyEvent::new(code, KeyModifiers::NONE));
                return;
            }
            _ => return,
        }
        match ev.kind {
            MouseEventKind::ScrollDown => self.scroll(SCROLL_STEP as i64),
            MouseEventKind::ScrollUp => self.scroll(-(SCROLL_STEP as i64)),
            MouseEventKind::Down(MouseButton::Left) => self.click(ev.column, ev.row, now),
            _ => {}
        }
    }

    /// Scrolls the view, dragging the cursor along to keep it visible.
    fn scroll(&mut self, delta: i64) {
        let page = self.page();
        let max_offset = self.rows.len().saturating_sub(page) as i64;
        self.offset = (self.offset as i64 + delta).clamp(0, max_offset) as usize;
        let last = self.offset + page - 1;
        self.cursor = self
            .cursor
            .clamp(self.offset, last.min(self.rows.len().saturating_sub(1)));
    }

    fn click(&mut self, x: u16, y: u16, now: Instant) {
        let area = self.areas.tree;
        if x < area.x || x >= area.x + area.width || y < area.y || y >= area.y + area.height {
            return;
        }
        let row = self.offset + usize::from(y - area.y);
        if row >= self.rows.len() {
            return;
        }
        self.message = None;
        self.cursor = row;
        let double = self
            .last_click
            .is_some_and(|(t, r)| r == row && now.duration_since(t) <= DOUBLE_CLICK);
        if double {
            self.toggle();
            self.last_click = None;
        } else {
            self.last_click = Some((now, row));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::test_util::*;
    use crate::ui::render::test_util::render;

    fn ev(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn click(app: &mut App, row: u16, at: Instant) {
        app.on_mouse_at(ev(MouseEventKind::Down(MouseButton::Left), 5, row), at);
    }

    #[test]
    fn click_selects_row() {
        let mut app = app();
        render(&mut app, 100, 20); // tree inner area starts at y = 1
        click(&mut app, 3, Instant::now()); // root, a, b
        assert_eq!(name(&app), "b");
        click(&mut app, 15, Instant::now() + DOUBLE_CLICK * 2);
        assert_eq!(name(&app), "b", "click below the rows is ignored");
    }

    #[test]
    fn double_click_expands_and_slow_clicks_do_not() {
        let mut app = app();
        render(&mut app, 100, 20);
        let t0 = Instant::now();
        click(&mut app, 2, t0); // a
        click(&mut app, 2, t0 + Duration::from_millis(150));
        let a = app.selected();
        assert!(app.is_open(a));
        assert_eq!(app.rows.len(), 6);
        let t1 = t0 + Duration::from_secs(5);
        click(&mut app, 2, t1);
        click(&mut app, 2, t1 + Duration::from_secs(1));
        assert!(app.is_open(a), "too slow: no toggle");
        click(&mut app, 2, t1 + Duration::from_millis(1100));
        assert!(!app.is_open(a), "fast second click collapses");
    }

    #[test]
    fn wheel_scrolls_view_and_drags_cursor() {
        let mut app = app();
        press(&mut app, "jl"); // 6 rows
        render(&mut app, 100, 7); // 3 visible rows
        app.move_to(0);
        render(&mut app, 100, 7);
        app.on_mouse_at(ev(MouseEventKind::ScrollDown, 5, 2), Instant::now());
        assert_eq!(app.offset, 3);
        assert_eq!(app.cursor, 3, "cursor dragged into view");
        app.on_mouse_at(ev(MouseEventKind::ScrollUp, 5, 2), Instant::now());
        assert_eq!(app.offset, 0);
        assert_eq!(app.cursor, 2);
    }

    #[test]
    fn mouse_ignored_under_dialogs() {
        let mut app = app();
        render(&mut app, 100, 20);
        app.overlay = Overlay::Help;
        click(&mut app, 3, Instant::now());
        assert_eq!(app.cursor, 0);
    }
}
