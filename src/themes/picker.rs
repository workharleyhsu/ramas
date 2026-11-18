//! The theme picker shared by every app: a list grouped by family with live
//! preview. The app feeds it keys and applies the events it returns.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::themes::catalog::Catalog;
use crate::themes::color::degrade;
use crate::themes::theme::Theme;

/// Preferred width of the picker panel, in cells.
pub const WIDTH: u16 = 38;

/// A key press, independent of the terminal library version the app uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKey {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Esc,
    Backspace,
    Char(char),
}

/// What the app must do after a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerEvent {
    /// Nothing changed that the app cares about.
    None,
    /// Show this theme (id) without saving it.
    Preview(String),
    /// Apply this theme (id), save it and close the picker.
    Confirm(String),
    /// Go back to this theme (id) and close the picker.
    Cancel(String),
}

/// Texts and glyph set of the picker, so apps can translate it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Labels<'a> {
    pub title: &'a str,
    /// Shown before the filter text.
    pub filter: &'a str,
    /// Shown in place of the filter text while it is empty.
    pub placeholder: &'a str,
    /// Key help on the last line.
    pub hint: &'a str,
    /// Shown when the filter matches nothing.
    pub no_match: &'a str,
    /// Heading of the list of theme files that failed to load.
    pub invalid: &'a str,
    /// Draw with ASCII only.
    pub ascii: bool,
}

impl Default for Labels<'_> {
    fn default() -> Self {
        Labels {
            title: "Theme",
            filter: "Filter",
            placeholder: "type to filter",
            hint: "Up/Down  Enter save  Esc cancel",
            no_match: "No theme matches",
            invalid: "Could not load",
            ascii: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    id: String,
    name: String,
    family: String,
    /// Accent, ok, warn, danger and ram of the theme, at full depth.
    swatch: [Color; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    Family(String),
    Entry(usize),
    Blank,
    InvalidHeading,
    /// File name of a theme that failed to load.
    InvalidFile(String),
    /// One wrapped line of the reason.
    InvalidReason(String),
    NoMatch,
}

/// State of an open theme picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picker {
    entries: Vec<Entry>,
    /// `(file name, reason)` of each theme file that failed to load.
    errors: Vec<(String, String)>,
    filter: String,
    /// Index into `entries` of the highlighted theme, if any is visible.
    cursor: Option<usize>,
    /// Theme that was active when the picker opened.
    original: String,
    scroll: usize,
    /// List rows that fit in the last render; used for paging.
    page: usize,
}

impl Picker {
    /// Opens the picker on `current` (a theme id).
    pub fn new(catalog: &Catalog, current: &str) -> Self {
        let entries: Vec<Entry> = catalog
            .themes()
            .iter()
            .map(|t| Entry {
                id: t.id.clone(),
                name: t.name.clone(),
                family: t.family.clone(),
                swatch: [t.accent, t.ok, t.warn, t.danger, t.ram],
            })
            .collect();
        let cursor = entries
            .iter()
            .position(|e| e.id.eq_ignore_ascii_case(current))
            .or(if entries.is_empty() { None } else { Some(0) });
        Picker {
            entries,
            errors: catalog
                .errors()
                .iter()
                .map(|e| {
                    let file = e.source.rsplit('/').next().unwrap_or(&e.source);
                    (file.to_string(), e.message.clone())
                })
                .collect(),
            filter: String::new(),
            cursor,
            original: current.to_string(),
            scroll: 0,
            page: 10,
        }
    }

    /// Id of the highlighted theme.
    pub fn selected(&self) -> Option<&str> {
        self.cursor.map(|i| self.entries[i].id.as_str())
    }

    /// Text typed so far to narrow the list.
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Id of the theme that was active when the picker opened.
    pub fn original(&self) -> &str {
        &self.original
    }

    fn matches(&self, entry: &Entry) -> bool {
        let needle = self.filter.to_lowercase();
        needle.is_empty()
            || entry.name.to_lowercase().contains(&needle)
            || entry.id.contains(&needle)
            || entry.family.to_lowercase().contains(&needle)
    }

    /// Indices of the entries the filter lets through, in list order.
    fn visible(&self) -> Vec<usize> {
        (0..self.entries.len())
            .filter(|i| self.matches(&self.entries[*i]))
            .collect()
    }

    fn step(&mut self, delta: isize) {
        let visible = self.visible();
        if visible.is_empty() {
            self.cursor = None;
            return;
        }
        let at = self
            .cursor
            .and_then(|c| visible.iter().position(|i| *i == c))
            .unwrap_or(0) as isize;
        let last = visible.len() as isize - 1;
        self.cursor = Some(visible[(at.saturating_add(delta)).clamp(0, last) as usize]);
    }

    /// After the filter changed: keep the cursor if still visible, else jump
    /// to the first match.
    fn refilter(&mut self) {
        let visible = self.visible();
        if !self.cursor.is_some_and(|c| visible.contains(&c)) {
            self.cursor = visible.first().copied();
        }
        self.scroll = 0;
    }

    /// Handles a key and says what the app must do.
    pub fn on_key(&mut self, key: PickerKey) -> PickerEvent {
        let before = self.cursor;
        match key {
            PickerKey::Esc => return PickerEvent::Cancel(self.original.clone()),
            PickerKey::Enter => {
                return match self.selected() {
                    Some(id) => PickerEvent::Confirm(id.to_string()),
                    None => PickerEvent::None,
                };
            }
            PickerKey::Up => self.step(-1),
            PickerKey::Down => self.step(1),
            PickerKey::PageUp => self.step(-(self.page.max(1) as isize)),
            PickerKey::PageDown => self.step(self.page.max(1) as isize),
            PickerKey::Home => self.step(isize::MIN),
            PickerKey::End => self.step(isize::MAX),
            PickerKey::Backspace => {
                self.filter.pop();
                self.refilter();
            }
            PickerKey::Char(c) if !c.is_control() => {
                self.filter.push(c);
                self.refilter();
            }
            PickerKey::Char(_) => {}
        }
        match self.selected() {
            Some(id) if self.cursor != before => PickerEvent::Preview(id.to_string()),
            _ => PickerEvent::None,
        }
    }

    fn rows(&self, width: usize) -> Vec<Row> {
        let mut rows = Vec::new();
        let mut family: Option<&str> = None;
        for i in self.visible() {
            let entry = &self.entries[i];
            if family != Some(entry.family.as_str()) {
                if family.is_some() {
                    rows.push(Row::Blank);
                }
                rows.push(Row::Family(entry.family.clone()));
                family = Some(&entry.family);
            }
            rows.push(Row::Entry(i));
        }
        if rows.is_empty() {
            rows.push(Row::NoMatch);
        }
        if !self.errors.is_empty() {
            rows.push(Row::Blank);
            rows.push(Row::InvalidHeading);
            for (file, reason) in &self.errors {
                rows.push(Row::InvalidFile(file.clone()));
                rows.extend(
                    wrap(reason, width.saturating_sub(2))
                        .into_iter()
                        .map(Row::InvalidReason),
                );
            }
        }
        rows
    }

    /// Where the picker goes inside `full`: a panel on the right edge, so
    /// most of the app stays visible for the preview.
    pub fn area(full: Rect) -> Rect {
        let width = WIDTH.min(full.width);
        Rect::new(full.x + full.width - width, full.y, width, full.height)
    }

    /// Draws the picker over `area` with the theme being previewed.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, labels: &Labels) {
        let area = area.intersection(buf.area);
        if area.width < 8 || area.height < 4 {
            return;
        }
        let (tl, tr, bl, br, h, v) = if labels.ascii {
            ('+', '+', '+', '+', '-', '|')
        } else {
            ('╭', '╮', '╰', '╯', '─', '│')
        };
        let text = theme.text();
        let border = theme.fg(theme.border);
        // Wipe whatever the app drew underneath.
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.reset();
                cell.set_style(text);
            }
        }
        let inner_w = usize::from(area.width - 2);
        let right = area.right() - 1;
        let bottom = area.bottom() - 1;
        let horizontal: String = std::iter::repeat_n(h, inner_w).collect();
        buf.set_string(area.x, area.y, format!("{tl}{horizontal}{tr}"), border);
        buf.set_string(area.x, bottom, format!("{bl}{horizontal}{br}"), border);
        for y in area.y + 1..bottom {
            buf.set_string(area.x, y, v.to_string(), border);
            buf.set_string(right, y, v.to_string(), border);
        }
        let title = format!(" {} ", labels.title);
        buf.set_stringn(
            area.x + 2,
            area.y,
            title,
            inner_w.saturating_sub(2),
            theme.fg(theme.accent).add_modifier(Modifier::BOLD),
        );

        let x = area.x + 2;
        let width = inner_w.saturating_sub(2);
        // Filter line, list, hint line.
        let cursor = if labels.ascii { "_" } else { "▏" };
        let prompt = format!("{}: ", labels.filter);
        buf.set_stringn(x, area.y + 1, &prompt, width, theme.dimmed());
        let typed_x = x + prompt.chars().count().min(width) as u16;
        let typed_w = width.saturating_sub(prompt.chars().count());
        if self.filter.is_empty() {
            let hint = format!("{cursor}{}", labels.placeholder);
            buf.set_stringn(typed_x, area.y + 1, hint, typed_w, theme.dimmed());
        } else {
            let typed = format!("{}{cursor}", self.filter);
            buf.set_stringn(typed_x, area.y + 1, typed, typed_w, text);
        }
        buf.set_stringn(x, bottom - 1, labels.hint, width, theme.dimmed());

        let list_top = area.y + 2;
        let list_h = usize::from((bottom - 1).saturating_sub(list_top));
        if list_h == 0 {
            return;
        }
        self.page = list_h.saturating_sub(1).max(1);
        let rows = self.rows(width);
        // Keep the cursor row (and its family heading when possible) in view.
        if let Some(at) = self
            .cursor
            .and_then(|c| rows.iter().position(|r| *r == Row::Entry(c)))
        {
            if at < self.scroll + 1 {
                self.scroll = at.saturating_sub(1);
            }
            if at >= self.scroll + list_h {
                self.scroll = at + 1 - list_h;
            }
            // On the last theme, also bring up what follows it (the list of
            // files that failed to load), as far as the cursor allows.
            if !rows[at + 1..].iter().any(|r| matches!(r, Row::Entry(_))) {
                self.scroll = rows.len().saturating_sub(list_h).min(at.saturating_sub(1));
            }
        }
        self.scroll = self.scroll.min(rows.len().saturating_sub(list_h));

        let dot = if labels.ascii { "o" } else { "●" };
        for (line, row) in rows.iter().skip(self.scroll).take(list_h).enumerate() {
            let y = list_top + line as u16;
            match row {
                Row::Blank => {}
                Row::Family(name) => {
                    buf.set_stringn(x, y, name, width, theme.heading());
                }
                Row::NoMatch => {
                    buf.set_stringn(x, y, labels.no_match, width, theme.dimmed());
                }
                Row::InvalidHeading => {
                    let style = theme.fg(theme.danger).add_modifier(Modifier::BOLD);
                    buf.set_stringn(x, y, labels.invalid, width, style);
                }
                Row::InvalidFile(file) => {
                    buf.set_stringn(x, y, file, width, theme.fg(theme.danger));
                }
                Row::InvalidReason(line) => {
                    buf.set_stringn(x + 2, y, line, width.saturating_sub(2), theme.dimmed());
                }
                Row::Entry(i) => {
                    let entry = &self.entries[*i];
                    let selected = self.cursor == Some(*i);
                    let style = if selected { theme.selected() } else { text };
                    if selected {
                        buf.set_style(Rect::new(x, y, width as u16, 1), style);
                    }
                    let mark = if entry.id.eq_ignore_ascii_case(&self.original) {
                        '*'
                    } else {
                        ' '
                    };
                    let swatch_w = entry.swatch.len();
                    let name_w = width.saturating_sub(swatch_w + 1);
                    buf.set_stringn(x, y, format!("{mark} {}", entry.name), name_w, style);
                    if theme.colored() && width > swatch_w + 8 {
                        let start = x + (width - swatch_w) as u16;
                        for (k, color) in entry.swatch.iter().enumerate() {
                            let fg = degrade(*color, theme.depth);
                            let mut s = Style::new();
                            if fg != Color::Reset {
                                s = s.fg(fg);
                            }
                            buf.set_string(start + k as u16, y, dot, s);
                        }
                    }
                }
            }
        }
    }
}

/// Splits `text` into lines of at most `width` characters, at spaces when
/// possible.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        loop {
            let used = line.chars().count();
            let gap = usize::from(used > 0);
            if used + gap + word.len() <= width {
                if gap == 1 {
                    line.push(' ');
                }
                line.extend(word.iter());
                break;
            }
            if used > 0 {
                lines.push(std::mem::take(&mut line));
                continue;
            }
            // A word longer than the line: cut it.
            lines.push(word.drain(..width).collect());
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::ColorDepth;

    /// A fixed handful of themes, so these tests do not depend on how the
    /// embedded catalog grows.
    fn catalog() -> Catalog {
        let all = Catalog::builtin();
        let mut small = Catalog::default();
        for id in [
            "nord",
            "dracula",
            "paper",
            "gruvbox-dark",
            "high-contrast-dark",
        ] {
            small.push(all.get_or_plain(id));
        }
        small
    }

    fn text(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    fn draw(picker: &mut Picker, w: u16, h: u16, theme: &Theme, labels: &Labels) -> Buffer {
        let full = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(full);
        buf.set_string(0, 0, "x".repeat(usize::from(w)), Style::new());
        picker.render(Picker::area(full), &mut buf, theme, labels);
        buf
    }

    #[test]
    fn opens_on_the_current_theme() {
        let p = Picker::new(&catalog(), "dracula");
        assert_eq!(p.selected(), Some("dracula"));
        assert_eq!(p.original(), "dracula");
        // An unknown current theme falls on the first entry.
        assert_eq!(Picker::new(&catalog(), "nope").selected(), Some("nord"));
    }

    #[test]
    fn arrows_preview_and_stop_at_the_ends() {
        let mut p = Picker::new(&catalog(), "nord");
        assert_eq!(p.on_key(PickerKey::Up), PickerEvent::None);
        assert_eq!(
            p.on_key(PickerKey::Down),
            PickerEvent::Preview("dracula".into())
        );
        let ids = catalog()
            .ids()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let last = ids.last().unwrap().clone();
        assert_eq!(p.on_key(PickerKey::End), PickerEvent::Preview(last.clone()));
        assert_eq!(p.on_key(PickerKey::Down), PickerEvent::None);
        assert_eq!(p.on_key(PickerKey::PageDown), PickerEvent::None);
        assert_eq!(
            p.on_key(PickerKey::Home),
            PickerEvent::Preview("nord".into())
        );
        assert_eq!(p.selected(), Some("nord"));
    }

    #[test]
    fn enter_confirms_and_esc_restores_the_original() {
        let mut p = Picker::new(&catalog(), "dracula");
        p.on_key(PickerKey::Down);
        let chosen = p.selected().unwrap().to_string();
        assert_ne!(chosen, "dracula");
        assert_eq!(
            p.clone().on_key(PickerKey::Enter),
            PickerEvent::Confirm(chosen)
        );
        assert_eq!(
            p.on_key(PickerKey::Esc),
            PickerEvent::Cancel("dracula".into())
        );
    }

    #[test]
    fn typing_filters_and_previews_the_first_match() {
        let mut p = Picker::new(&catalog(), "nord");
        assert_eq!(
            p.on_key(PickerKey::Char('V')),
            PickerEvent::Preview("gruvbox-dark".into())
        );
        assert_eq!(p.filter(), "V");
        // Still matching: the cursor stays, so nothing to preview again.
        assert_eq!(p.on_key(PickerKey::Char('b')), PickerEvent::None);
        for c in "zzz".chars() {
            p.on_key(PickerKey::Char(c));
        }
        assert_eq!(p.selected(), None);
        assert_eq!(p.on_key(PickerKey::Enter), PickerEvent::None);
        assert_eq!(p.on_key(PickerKey::Down), PickerEvent::None);
        for _ in 0..5 {
            p.on_key(PickerKey::Backspace);
        }
        assert_eq!(p.filter(), "");
        // The cursor stays where the filter left it.
        assert_eq!(p.selected(), Some("gruvbox-dark"));
        // Families match too.
        p.on_key(PickerKey::Char('a'));
        p.on_key(PickerKey::Char('c'));
        p.on_key(PickerKey::Char('c'));
        assert_eq!(p.selected(), Some("high-contrast-dark"));
        assert_eq!(p.on_key(PickerKey::Char('\u{7}')), PickerEvent::None);
    }

    #[test]
    fn renders_families_names_and_the_selection() {
        let theme = catalog().get_or_plain("gruvbox-dark");
        let mut p = Picker::new(&catalog(), "dracula");
        let buf = draw(&mut p, 80, 20, &theme, &Labels::default());
        let lines = text(&buf);
        let body = lines.join("\n");
        for needle in [
            "Theme",
            "Filter: ",
            "Popular",
            "* Dracula",
            "Gruvbox Dark",
            "Enter save",
        ] {
            assert!(body.contains(needle), "{needle}\n{body}");
        }
        // Only the right panel is touched.
        assert!(lines[0].starts_with(&"x".repeat(usize::from(80 - WIDTH))));
        let row = lines.iter().position(|l| l.contains("* Dracula")).unwrap() as u16;
        let x = 80 - WIDTH + 2;
        assert_eq!(buf[(x, row)].bg, theme.selection_bg);
        assert_eq!(buf[(x, row - 1)].bg, theme.bg);
        // Swatches carry the listed theme's own colors.
        let listed = catalog().get_or_plain("dracula");
        assert_eq!(buf[(80 - 2 - 5, row)].fg, listed.accent);
    }

    #[test]
    fn ascii_mode_uses_ascii_only_and_no_color_has_no_colors() {
        let theme = catalog().get_or_plain("dracula").at(ColorDepth::NoColor);
        let labels = Labels {
            ascii: true,
            ..Labels::default()
        };
        let mut p = Picker::new(&catalog(), "dracula");
        let buf = draw(&mut p, 60, 16, &theme, &labels);
        assert!(text(&buf).join("").is_ascii());
        for cell in &buf.content {
            assert_eq!(cell.fg, Color::Reset);
            assert_eq!(cell.bg, Color::Reset);
        }
    }

    #[test]
    fn broken_user_themes_are_listed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.toml"), "nope").unwrap();
        let catalog = Catalog::with_user_dir(dir.path());
        let theme = catalog.get_or_plain("dracula");
        let mut p = Picker::new(&catalog, "dracula");
        // The failed files come after the last theme.
        p.on_key(PickerKey::End);
        let buf = draw(&mut p, 120, 30, &theme, &Labels::default());
        let body = text(&buf).join("\n");
        assert!(body.contains("Could not load"), "{body}");
        assert!(body.contains(" bad.toml "), "{body}");
        // The reason is shown whole, wrapped, without the directory.
        assert!(!body.contains(&dir.path().display().to_string()), "{body}");
        assert!(body.contains("line 1:"), "{body}");
        // In a short panel too, End brings the whole reason into view.
        let buf = draw(&mut p, 120, 12, &theme, &Labels::default());
        let body = text(&buf).join("\n");
        assert!(body.contains("line 1:"), "{body}");
    }

    #[test]
    fn wraps_long_text() {
        assert_eq!(wrap("one two three", 7), ["one two", "three"]);
        assert_eq!(wrap("abcdefghij kl", 4), ["abcd", "efgh", "ij", "kl"]);
        assert_eq!(wrap("", 5), Vec::<String>::new());
        assert_eq!(wrap("a b", 0), ["a", "b"]);
    }

    #[test]
    fn scrolls_to_keep_the_cursor_visible_and_survives_tiny_areas() {
        let theme = catalog().get_or_plain("dracula");
        let mut p = Picker::new(&catalog(), "nord");
        p.on_key(PickerKey::End);
        let buf = draw(&mut p, 50, 7, &theme, &Labels::default());
        assert!(text(&buf).join("\n").contains("High Contrast Dark"));
        p.on_key(PickerKey::Home);
        let buf = draw(&mut p, 50, 7, &theme, &Labels::default());
        assert!(text(&buf).join("\n").contains("Nord"));
        for (w, h) in [(1, 1), (7, 3), (8, 4), (9, 5), (200, 4)] {
            let _ = draw(&mut p, w, h, &theme, &Labels::default());
        }
        // An area outside the buffer is ignored.
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        p.render(
            Rect::new(40, 40, 20, 10),
            &mut buf,
            &theme,
            &Labels::default(),
        );
    }
}
