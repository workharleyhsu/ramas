# Temas

ramas toma todos sus colores de un tema: un archivo TOML que nombra **roles**
(fondo, texto, acento, ok / aviso / peligro…) en vez de colores sueltos.
Vienen 31 temas incluidos y puedes agregar los tuyos.

Read in English: [THEMES.md](THEMES.md).

## Elegir un tema

Pulsa **`T`**.

| Tecla | Acción |
|---|---|
| `↑` `↓` `RePág` `AvPág` `Inicio` `Fin` | Moverse. Toda la pantalla pasa al tema bajo el cursor al instante. |
| cualquier letra | Filtrar la lista por nombre, id o familia. `Retroceso` borra. |
| `Enter` | Dejar el tema y guardarlo en la configuración. |
| `Esc` | Cerrar y volver al tema que tenías. |

`*` marca el tema que estaba activo al abrir el selector. Los cinco puntos
junto a cada nombre son colores de ese tema (acento, ok, aviso, peligro y uno
más).

`Enter` escribe la elección en `~/.config/ramas/config.toml` (o el archivo
dado con `--config`):

```toml
theme = "nord"
```

`--theme <id>` elige un tema para una sola ejecución. Los nombres de ramas
0.1 siguen funcionando: `dark` (ahora `ramas`), `light` (`ramas-light`),
`high-contrast` (`high-contrast-dark`) y `gruvbox` (`gruvbox-dark`). También
`custom` con una tabla `[custom_theme]` en la configuración.

Los comentarios y las demás opciones del archivo no se tocan.

## Temas incluidos

| Familia | Temas (id) |
|---|---|
| Default | `ramas`, `ramas-light` |
| Popular | `catppuccin-mocha`, `catppuccin-macchiato`, `catppuccin-frappe`, `catppuccin-latte`, `dracula`, `everforest`, `gruvbox-dark`, `gruvbox-light`, `kanagawa`, `monokai`, `nord`, `one-dark`, `rose-pine`, `solarized-dark`, `solarized-light`, `tokyo-night` |
| Light & pastel | `paper`, `pastel-dawn`, `mint-cream`, `lavender-mist` |
| Retro & neon | `phosphor-green`, `amber`, `mono-gray`, `cyberpunk`, `synthwave` |
| Accessible | `high-contrast-dark`, `high-contrast-light`, `okabe-ito-dark`, `okabe-ito-light` |

`ramas` es el aspecto original: conserva el fondo de la terminal.
Los dos temas Okabe-Ito son seguros para el daltonismo rojo-verde; los de
alto contraste tienen 21:1 de contraste de texto.

Donde una paleta publicada tenía un color demasiado tenue sobre su propio
fondo, se lo acercó al color del texto lo justo para llegar a 3:1.

## Crear tu propio tema

1. Copia [`example-theme.toml`](example-theme.toml) a
   `~/.config/ramas/themes/<nombre>.toml` (crea la carpeta si no existe; con
   `XDG_CONFIG_HOME` definido es `$XDG_CONFIG_HOME/ramas/themes`).
2. Cambia los colores.
3. Abre ramas y pulsa `T`. Tu tema aparece bajo su `family` ("User" si no
   pones otra).

El nombre del archivo sin `.toml`, en minúsculas, es el **id** del tema. Un
archivo con el id de un tema incluido lo reemplaza.

Si el archivo tiene un error, ramas arranca igual. Aparece un aviso, y el
selector lista el archivo bajo "No se pudo cargar" con la línea y el motivo,
por ejemplo `colors.warn: missing required role`.

### Colores

| Escribes | Significa |
|---|---|
| `"#rrggbb"` | Un color de 24 bits. |
| `"terminal"` | El color por defecto de la terminal (fondo o texto). No vale en degradados. |
| `"red"`, `"lightblue"`, `"gray"`… | Uno de los 16 colores de la terminal, por nombre. |
| `"214"` | Un índice de la paleta de 256 colores. |

### Claves generales

| Clave | Valores | Por defecto |
|---|---|---|
| `name` | Texto que muestra el selector | el id |
| `family` | Título del grupo en el selector | `"User"` |
| `appearance` | `"dark"` o `"light"` | se deduce de `bg` |
| `high_contrast` | `true` o `false` | `false` |
| `assumed_bg` | Un color; solo se usa si `bg = "terminal"` | negro o blanco, según `appearance` |

### Roles obligatorios (`[colors]`)

| Rol | Se usa en |
|---|---|
| `bg` | Fondo de toda la pantalla |
| `fg` | Texto normal |
| `dim` | Texto secundario, ayudas |
| `accent` | Atajos, títulos de diálogos, el elemento activo |
| `border` | Marcos y separadores |
| `selection_fg`, `selection_bg` | La fila o app seleccionada |
| `ok` | Carga baja; lo que se achicó |
| `warn` | Carga media; avisos que no son errores |
| `danger` | Carga alta; errores; borrado |

### Roles opcionales (`[colors]`)

| Rol | Se usa en | Si falta |
|---|---|---|
| `header` | Títulos de sección | `fg` |
| `connector` | Líneas del árbol | `border` |
| `dir` | Nombres de carpetas | `accent` |
| `marked` | Elementos marcados para borrar | `accent` |
| `matched` | Coincidencias de búsqueda | `warn` |
| `status_fg`, `status_bg` | Barra de estado | `selection_fg`, `selection_bg` |
| `grew`, `shrank`, `new` | `--diff` contra un escaneo anterior | `danger`, `ok`, `accent` |
| `value` | Tamaños y otros números | `warn` |

Se aceptan para que un archivo de tema pueda traerlos, pero ramas no dibuja
nada con ellos: los roles `ram` e `icon`, y la clave general `icon_mode`.

### Degradados (`[gradients]`)

| Clave | Se usa en | Si falta |
|---|---|---|
| `size` | Barras de tamaño, de chico a grande | `[ok, warn, danger]` |
| `graph` | ramas no lo usa | `[ok, warn, danger]` |

Cada uno es una lista de dos o más colores, de menor a mayor. Los colores
intermedios se mezclan.

### Legibilidad

Los temas incluidos pasan un test de contraste: texto sobre fondo al menos
4.5:1 (7:1 en los de alto contraste), selección y barra de estado al menos
4.5:1, y cada rol de color al menos 3:1 sobre el fondo. Tus temas no se
revisan, así que puedes saltarte esos mínimos; son una buena meta si algo
cuesta leer.

## Terminales con menos colores

Los temas se escriben en 24 bits y se convierten a lo que la terminal puede
mostrar:

| Terminal | Qué pasa |
|---|---|
| `COLORTERM=truecolor` o `24bit` | Los colores tal como están escritos. |
| `TERM` contiene `256` | El color más cercano de la paleta de 256. |
| Cualquier otra | El más cercano de los 16 colores básicos; el texto nunca cae sobre el color de su propio fondo. |
| `NO_COLOR` definido, o `TERM=dumb` | Sin colores: solo negrita, tenue y video inverso. |
