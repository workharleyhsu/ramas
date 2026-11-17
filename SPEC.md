# SPEC — `ramas`

Analizador de uso de disco en árbol para la terminal (Linux), alternativa visual
a ncdu. TUI en Rust: escanea un directorio y lo muestra como un árbol ramificado
e interactivo, con borrado en lote seguro, búsqueda, filtros, análisis por tipo,
temas y exportación/importación/diff.

## Stack y decisiones

| Área | Elección | Motivo |
|---|---|---|
| Edición | Rust estable, edición 2024 | vigente |
| TUI | `ratatui` 0.30 + `crossterm` 0.29 (misma versión que usa ratatui) | pedido; mouse incluido |
| Escaneo | **`rayon` + `std::fs`** (no `jwalk`) | ver abajo |
| CLI | `clap` (derive) + `clap_mangen` para la página man | pedido |
| Serialización | `serde` + `serde_json`; config con `toml` | pedido |
| Papelera | `trash` (freedesktop en Linux; sin C) | pedido |
| Espacio libre | `rustix::fs::statvfs` | backend linux_raw, sin C ni `unsafe` propio |
| Búsqueda difusa | `nucleo-matcher` | rápido, sin dependencias de C |
| Rutas de config | `dirs` | `~/.config/ramas/config.toml` |
| Errores | `anyhow` (binario) + `thiserror` (tipos de dominio) | pedido |
| Tests | `tempfile`, `insta` (snapshots de render) | |

### Por qué `rayon` + `std::fs` y no `jwalk`
- Control total de un único `symlink_metadata` por entrada (bloques, tamaño
  aparente, inodo, dispositivo, mtime, modo) sin metadatos extra.
- Chequeo de dispositivo para `-x` y salto de pseudo-FS antes de descender.
- Error por nodo (permiso denegado) marcado en el árbol sin abortar.
- Cancelación con un `AtomicBool` y progreso con atómicos, consultados en cada
  directorio; sin adaptarse al modelo de callbacks de jwalk.
- Recursión con `rayon::scope`/`par_iter` sobre subdirectorios: cada tarea
  devuelve su subárbol local; una fusión de un solo hilo lo vuelca en la arena,
  y ahí se deduplican los hard links, así la atribución es determinista.

### `unsafe`
Ninguno: `#![forbid(unsafe_code)]` a nivel de crate (`[lints.rust]`).
Nota: en edición 2024 `std::env::set_var` es `unsafe`; los tests que necesitan
variables de entorno (papelera) lanzan un proceso hijo con `Command::env`.

### Lints
`clippy::unwrap_used` y `clippy::expect_used` denegados fuera de tests
(`clippy.toml` los permite en tests).

## Modelo de datos
- Árbol en arena: `Vec<Node>`, `NodeId(u32)`, enlaces `parent`, `first_child`,
  `next_sibling`. Nombres en un único blob de bytes (offset + longitud).
- Por nodo: tamaño en disco (`st_blocks * 512`), tamaño aparente (`st_len`),
  cantidad de archivos y directorios descendientes, `mtime`, modo, `(dev, ino)`,
  tipo (archivo, dir, symlink, otro) y flags (error, hard link duplicado).
- Los agregados de directorio se calculan en la fusión.
- Ordenar = reenlazar listas de hermanos; no se realoca.
- Borrar = desenlazar subárbol y restar tamaños/contadores en los ancestros.

## Especificación funcional
(Copia del prompt original; es el contrato.)

### Escaneo
- `ramas [RUTA]` (por defecto: directorio actual). Escaneo en hilo aparte con
  pantalla de progreso (archivos, tamaño acumulado, ruta actual) y cancelable.
- Tamaño en disco (bloques) por defecto y tamaño aparente como alternativa.
- Hard links contados una sola vez. No seguir symlinks. Flag `-x` para no
  cruzar sistemas de archivos. Saltar `/proc`, `/sys`, `/dev` y similares.
- Errores de permisos: no abortar; marcar el nodo y contarlos en la barra de estado.
- Árbol compacto en arena; debe soportar millones de entradas sin congelar la UI.

### Layout
- Izquierda (~60%): árbol con conectores `├── └── │`, expandir/colapsar,
  ordenado por tamaño descendente. Cada nodo: icono, nombre, tamaño legible,
  % respecto al padre y barra proporcional con gradiente por peso.
- Derecha (~40%): ruta completa, tamaño en disco y aparente, archivos/subdirs,
  mtime, permisos, top 10 archivos más pesados y desglose por tipo.
- Abajo: barra de estado (total, espacio libre, marcados, errores) y ayuda
  contextual. En terminales angostas el panel lateral se oculta (tecla `p`).

### Navegación
`↑/↓`, `→` expandir/entrar, `←` colapsar/ir al padre, `Enter` alternar,
`PgUp/PgDn/Home/End`, `hjkl`. Mouse: clic selecciona, doble clic expande,
scroll desplaza. `s` cicla orden (tamaño, nombre, fecha, ítems), `a` aparente/
en disco, `r` reescanear nodo, `?` ayuda, `q` salir.

### Marcado y borrado
- `Espacio` marca/desmarca; marcar un dir implica su contenido; nunca padre e
  hijo a la vez (marcar un padre quita las marcas de sus descendientes; marcar
  algo bajo un padre marcado no hace nada).
- Barra de estado: "N elementos marcados — X a liberar".
- `d`: diálogo con rutas, tamaños, total y [Papelera] [Borrar permanente]
  [Cancelar], foco inicial en Cancelar. Permanente exige escribir `delete`
  (inglés) o `borrar` (español).
- Tras borrar: actualizar árbol y ancestros sin reescanear; resumen (liberado,
  fallidos con motivo). `--read-only` deshabilita el borrado.

### Búsqueda y filtros
`/` búsqueda difusa incremental, `n/N`, expansión automática de ramas con
coincidencias. `f` filtros combinables: tamaño mínimo, extensión, antigüedad;
los nodos que no coinciden se atenúan (o se ocultan, alternable).

### Análisis por tipo
Categorías por extensión (video, imagen, audio, documentos, comprimidos,
código, cachés/temporales, otros), configurable en TOML. Vista `t`: tabla
del nodo seleccionado; `Enter` lista los archivos más pesados de la categoría
(marcables).

### Temas e iconos
Temas: `dark` (defecto), `light`, `high-contrast`, `gruvbox`, y `custom`
desde TOML. `T` cicla. Iconos Nerd Font opcionales (`icons = "nerd"`),
fallback Unicode por defecto; `--no-icons` = ASCII. Respeta `NO_COLOR`.

### Idioma
UI en inglés por defecto; español con `--lang es` o `language = "es"`.
Todas las cadenas de UI en `src/i18n.rs`.

### Exportar / importar / diff
`--export f.json`, `--import f.json` (borrado deshabilitado hasta reescanear),
`--diff anterior.json` (resalta creció/achicó/nuevo con delta), tecla `e`.

## Reglas de seguridad (innegociables)
- Tests de borrado SOLO sobre `tempfile`.
- Nunca borrar fuera de la raíz escaneada: se canonicaliza el **padre** y se le
  une el nombre (así un symlink se valida como sí mismo, no como su destino);
  el resultado debe estar estrictamente dentro de la raíz canónica.
- Nunca seguir symlinks al borrar (`remove_file` sobre el enlace;
  `remove_dir_all` de std no sigue symlinks).
- Rechazar `/`, `$HOME` y la raíz del escaneo.
- Antes de borrar: la ruta existe y `(dev, ino)` coincide con lo escaneado.
- Sin `unsafe`; sin `unwrap/expect` fuera de tests.
- Terminal restaurada siempre (guard RAII + panic hook).

## Arquitectura
```
src/main.rs        CLI, panic hook, despacho de modos
src/cli.rs         definición clap (también para man)
src/i18n.rs        cadenas EN/ES
src/config.rs      TOML
src/tree/          arena, orden, agregados, borrado de subárbol
src/scan/          walker paralelo, hard links, pseudo-FS, progreso
src/classify.rs    extensión -> categoría
src/search.rs      búsqueda difusa
src/filter.rs      filtros
src/actions/       marcas y borrado seguro
src/export/        JSON y diff
src/ui/            App (estado + update puro), vistas, temas, iconos, mouse
```
Lógica separada del render: `App::update(evento)` muta estado; las funciones
de render reciben `&App`. Tests con `TestBackend` sin terminal real.
