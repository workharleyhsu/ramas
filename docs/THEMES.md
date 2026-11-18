# Themes

ramas takes every color from a theme: one TOML file that names **roles**
(background, text, accent, ok / warn / danger…) instead of loose colors.
31 themes are built in, and you can add your own.

Leer en español: [THEMES.es.md](THEMES.es.md).

## Choosing a theme

Press **`T`**.

| Key | Action |
|---|---|
| `↑` `↓` `PgUp` `PgDn` `Home` `End` | Move. The whole screen switches to the theme under the cursor at once. |
| any letter | Filter the list by name, id or family. `Backspace` deletes. |
| `Enter` | Keep the theme and save it in the config file. |
| `Esc` | Close and go back to the theme you had. |

`*` marks the theme that was active when you opened the picker. The five dots
next to each name are colors of that theme (accent, ok, warn, danger and one
more).

`Enter` writes the choice to `~/.config/ramas/config.toml` (or the file
given with `--config`):

```toml
theme = "nord"
```

`--theme <id>` picks a theme for one run. The names used by ramas 0.1 still
work: `dark` (now `ramas`), `light` (`ramas-light`), `high-contrast`
(`high-contrast-dark`) and `gruvbox` (`gruvbox-dark`). So does `custom` with
a `[custom_theme]` table in the config.

Comments and other settings in the file are kept.

## Included themes

| Family | Themes (id) |
|---|---|
| Default | `ramas`, `ramas-light` |
| Popular | `catppuccin-mocha`, `catppuccin-macchiato`, `catppuccin-frappe`, `catppuccin-latte`, `dracula`, `everforest`, `gruvbox-dark`, `gruvbox-light`, `kanagawa`, `monokai`, `nord`, `one-dark`, `rose-pine`, `solarized-dark`, `solarized-light`, `tokyo-night` |
| Light & pastel | `paper`, `pastel-dawn`, `mint-cream`, `lavender-mist` |
| Retro & neon | `phosphor-green`, `amber`, `mono-gray`, `cyberpunk`, `synthwave` |
| Accessible | `high-contrast-dark`, `high-contrast-light`, `okabe-ito-dark`, `okabe-ito-light` |

`ramas` is the original look: it keeps the terminal's own background.
The two Okabe-Ito themes are safe for red-green color blindness; the
high-contrast ones have 21:1 text contrast.

Where a published palette had a color too faint to read over its own
background, it was moved toward the text color just enough to reach 3:1.

## Making your own theme

1. Copy [`example-theme.toml`](example-theme.toml) to
   `~/.config/ramas/themes/<name>.toml` (create the folder if needed; with
   `XDG_CONFIG_HOME` set, it is `$XDG_CONFIG_HOME/ramas/themes`).
2. Edit the colors.
3. Start ramas and press `T`. Your theme is listed under its `family`
   ("User" unless you set one).

The file name without `.toml`, in lowercase, is the theme's **id**. A file
with the id of an included theme replaces it.

If the file has a mistake ramas still starts. You get a notice, and the
picker lists the file under "Could not load" with the line and the reason,
for example `colors.warn: missing required role`.

### Colors

| You write | Meaning |
|---|---|
| `"#rrggbb"` | A 24-bit color. |
| `"terminal"` | The terminal's own default (background or text). Not allowed in gradients. |
| `"red"`, `"lightblue"`, `"gray"`… | One of the 16 terminal colors, by name. |
| `"214"` | An index of the 256-color palette. |

### Top-level keys

| Key | Values | Default |
|---|---|---|
| `name` | Text shown in the picker | the id |
| `family` | Group heading in the picker | `"User"` |
| `appearance` | `"dark"` or `"light"` | guessed from `bg` |
| `high_contrast` | `true` or `false` | `false` |
| `assumed_bg` | A color; only used when `bg = "terminal"` | black or white, by `appearance` |

### Required roles (`[colors]`)

| Role | Used for |
|---|---|
| `bg` | Background of the whole screen |
| `fg` | Normal text |
| `dim` | Secondary text, hints |
| `accent` | Shortcuts, dialog titles, the active element |
| `border` | Frames and separators |
| `selection_fg`, `selection_bg` | The selected row or app |
| `ok` | Low load; things that shrank |
| `warn` | Medium load; notices that are not errors |
| `danger` | High load; errors; deletion |

### Optional roles (`[colors]`)

| Role | Used for | When missing |
|---|---|---|
| `header` | Section headings | `fg` |
| `connector` | Tree lines | `border` |
| `dir` | Directory names | `accent` |
| `marked` | Items marked for deletion | `accent` |
| `matched` | Search matches | `warn` |
| `status_fg`, `status_bg` | Status bar | `selection_fg`, `selection_bg` |
| `grew`, `shrank`, `new` | `--diff` against an older scan | `danger`, `ok`, `accent` |
| `value` | Sizes and other numbers | `warn` |

These are accepted so that a theme file can carry them, but ramas does not
draw anything with them: the roles `ram` and `icon`, and the top-level key
`icon_mode`.

### Gradients (`[gradients]`)

| Key | Used for | When missing |
|---|---|---|
| `size` | Size bars, from small to large | `[ok, warn, danger]` |
| `graph` | Not used by ramas | `[ok, warn, danger]` |

Each is a list of two or more colors, from low to high. Colors in between
are blended.

### Readability

The included themes are tested for contrast: text over background at least
4.5:1 (7:1 for high-contrast themes), the selection and the status bar at
least 4.5:1, and every colored role at least 3:1 over the background. Your
own themes are not checked, so you are free to break these; they are a good
target if something is hard to read.

## Terminals with fewer colors

Themes are written in 24-bit color and converted to what the terminal can
show:

| Terminal | What happens |
|---|---|
| `COLORTERM=truecolor` or `24bit` | Colors as written. |
| `TERM` contains `256` | Nearest color of the 256-color palette. |
| Anything else | Nearest of the 16 basic colors; text never lands on the color of its own background. |
| `NO_COLOR` set, or `TERM=dumb` | No colors: bold, dim and reverse video only. |
