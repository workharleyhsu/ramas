# ramas

Analizador de uso de disco en árbol, interactivo, para la terminal de Linux:
una alternativa visual a `ncdu`. Escanea un directorio en paralelo y lo
muestra como un árbol expandible con barras proporcionales, panel de detalle,
borrado en lote seguro, búsqueda difusa, filtros, análisis por tipo, temas y
exportación/diff en JSON.

[Read in English](README.md)

```text
╭ /home/ana ─────────────────────── orden: tamaño · uso en disco ╮╭ Detalle ─────────────────────────────────╮
│▾ /home/ana                            5.8 GiB 100.0% ██████████││Ruta       /home/ana/.cache/cargo-build   │
│├── ▾ Videos/                          4.1 GiB  70.7% ███████▏  ││Tipo       directorio                     │
││   ├── · trip-2025.mkv                3.3 GiB  80.6% ████████  ││En disco   900 MiB                        │
││   └── · talk.mp4                     820 MiB  19.4% ██        ││Aparente   900 MiB                        │
│├── ▾ .cache/                          1.1 GiB  19.1% █▉        ││Archivos   1                              │
││   ├── ▸ cargo-build/                 900 MiB  78.9% ███████▉  ││Subdirs    0                              │
││   └── · thumbnails.cache             240 MiB  21.1% ██▏       ││Modificado 2025-10-09 05:53               │
│├── ▸ Music/                           410 MiB   6.9% ▋         ││Permisos   drwxr-xr-x (0755)              │
│├── ▸ code/                            150 MiB   2.5% ▎         ││                                          │
│├── ▸ Pictures/                       45.0 MiB   0.8% ▏         ││Archivos más pesados                      │
│└── · notes.md                        20.5 KiB   0.0%           ││  900 MiB deps.tmp                        │
│                                                                ││                                          │
│                                                                ││Por tipo                                  │
│                                                                ││caché/temp  ████████████   900 MiB      1 │
│                                                                ││                                          │
│                                                                ││                                          │
│                                                                ││                                          │
│                                                                ││                                          │
╰────────────────────────────────────────────────────────────────╯╰──────────────────────────────────────────╯
 Total 5.8 GiB · 0 elementos marcados — 0 B a liberar
 ↑↓ mover  ←→ plegar  espacio marcar  d borrar  / buscar  f filtrar  t tipos  ? ayuda  q salir
```

## Funciones

- **Escaneo paralelo rápido** (rayon + `std::fs`) con pantalla de progreso
  cancelable. Uso en disco (bloques) por defecto y tamaño aparente con `a`.
  Hard links contados una vez, symlinks nunca seguidos, `-x` no cruza sistemas
  de archivos, se saltan `/proc`, `/sys`, `/dev` y otros pseudo sistemas de
  archivos. Los errores de permisos se marcan en el nodo y se cuentan, sin abortar.
- **Árbol compacto en arena**: unos 85 MiB por millón de entradas; la UI solo
  dibuja la ventana visible y se mantiene fluida con árboles enormes.
- **Árbol + panel lateral**: conectores, iconos, tamaño, porcentaje del padre y
  barra con gradiente por fila; el panel muestra ruta, tamaños, conteos, fecha,
  permisos, los 10 archivos más pesados y el desglose por tipo.
- **Borrado en lote con resguardos**: marcá con `Espacio`, revisá en un diálogo
  que arranca en *Cancelar*, mandá a la papelera o borrá en forma permanente
  escribiendo una palabra de confirmación. El árbol se actualiza sin reescanear.
- **Búsqueda y filtros**: búsqueda difusa `/` que expande las ramas con
  coincidencias; filtros combinables por tamaño mínimo, extensión y antigüedad,
  atenuando u ocultando.
- **Análisis por tipo** (`t`): video, imágenes, audio, documentos, comprimidos,
  código, caché/temporales y otros, configurable; abrí una categoría para
  listar y marcar sus archivos más pesados.
- **Temas e iconos**: oscuro, claro, alto contraste, gruvbox y uno
  personalizado desde la configuración; iconos Nerd Font, Unicode o ASCII puro;
  respeta `NO_COLOR`.
- **Interfaz en inglés y español** (`--lang es`).
- **Exportar / importar / diff**: guardá un escaneo en JSON, abrilo después o
  comparalo con uno nuevo para ver qué creció, se achicó o es nuevo.
- **Binario estático**: sin dependencias de C, compila para
  `x86_64-unknown-linux-musl`.

## Instalación

```sh
git clone <este repo> && cd ramas
./install.sh                    # ~/.local/bin/ramas y su página man
./install.sh --prefix /usr/local
./install.sh --uninstall
```

`scripts/build-static.sh` compila el binario estático en `dist/` y lo verifica
con `ldd` (`--containers` además lo ejecuta en Alpine y Debian).

## Uso

```sh
ramas                         # escanea el directorio actual
ramas -x /                    # el sistema de archivos raíz, sin cruzar montajes
ramas --read-only ~/Descargas # navegar con el borrado deshabilitado
ramas --export home.json ~    # escanear sin TUI y guardar el resultado
ramas --import home.json      # abrir un escaneo guardado (sin borrado)
ramas --diff home.json ~      # resaltar qué creció, se achicó o es nuevo
```

`ramas --help` lista todas las opciones, teclas y ajustes; `man ramas` tiene lo mismo.

## Teclas

| Tecla | Acción |
|---|---|
| `↑ ↓ / k j` | Mover |
| `→ / l` | Expandir / entrar |
| `← / h` | Colapsar / ir al padre |
| `Enter` | Alternar expansión |
| `RePág AvPág Inicio Fin` | Página arriba/abajo, primero, último |
| `Espacio` | Marcar / desmarcar (baja una fila) |
| `u` | Quitar todas las marcas |
| `d` | Borrar lo marcado (diálogo) |
| `r` | Reescanear el directorio seleccionado |
| `/` | Búsqueda difusa incremental |
| `n / N` | Coincidencia siguiente / anterior |
| `f` | Filtros: tamaño mínimo, extensiones, antigüedad; atenuar u ocultar |
| `t` | Análisis por tipo del nodo seleccionado |
| `s` | Ciclar orden: tamaño, nombre, fecha, ítems |
| `a` | Alternar tamaño aparente / en disco |
| `p` | Mostrar / ocultar el panel lateral |
| `T` | Ciclar tema |
| `e` | Exportar JSON al directorio actual |
| `?` | Ayuda |
| `q` | Salir |

Mouse: clic selecciona, doble clic expande o colapsa, la rueda desplaza.

## Borrado seguro

```text
╭ /home/ana ─────────────────────── orden: tamaño · uso en disco ╮╭ Detalle ─────────────────────────────────╮
│▾ /home/ana                            5.8 GiB 100.0% ██████████││Ruta       /home/ana/code                 │
│├── ▾ Videos/                          4.1 GiB  70.7% ███████▏  ││Tipo       directorio                     │
││   ├── ✔ trip-2025.mkv                3.3 GiB  80.6% ████████  ││En disco   150 MiB                        │
││   └── · talk.mp4                     820 MiB  19.4% ██        ││Aparente   150 MiB                        │
│├── ╭ Borrar elementos marcados ───────────────────────────────────────────────────────────────────────╮    │
│├── │   3.3 GiB  /home/ana/Videos/trip-2025.mkv                                                        │    │
│├── │   410 MiB  /home/ana/Music                                                                       │    │
│├── │                                                                                                  │    │
│└── │ Total a liberar: 3.7 GiB                                                                         │    │
│    │                                                                                                  │    │
│    │ [ Papelera ]  [ Borrar permanente ]  [ Cancelar ]                                                │    │
│    │                                                                                                  │    │
│    ╰──────────────────────────────────────────────────────────────────────────────────────────────────╯    │
│                                                                ││Por tipo                                  │
╰────────────────────────────────────────────────────────────────╯╰──────────────────────────────────────────╯
 Total 5.8 GiB · 2 elementos marcados — 3.7 GiB a liberar
 ←→/Tab elegir  Enter confirmar  Esc cancelar
```

Cada ruta marcada se verifica justo antes de borrarla:

- se reconstruye desde su **padre** canonicalizado más su propio nombre, así un
  symlink se evalúa como sí mismo y nunca como su destino;
- tiene que estar estrictamente dentro de la raíz escaneada;
- se rechazan `/`, `$HOME` (y cualquier directorio que lo contenga) y la raíz
  del escaneo;
- tiene que seguir existiendo con el mismo dispositivo e inodo que se escanearon.

Los symlinks se borran, nunca se siguen. Marcar un directorio cubre todo su
contenido, así que nunca quedan marcados a la vez un padre y su hijo.
`--read-only` deshabilita el borrado por completo, igual que un escaneo
importado hasta que reescanees su raíz con `r`.

## Análisis por tipo

```text
╭ /home/ana ─────────────────────── orden: tamaño · uso en disco ╮╭ Detalle ─────────────────────────────────╮
│▾╭ Tipos en /home/ana ────────────────────────────────────────────────────────────────────────────────────╮ │
│├│ Categoría           Tamaño Cantidad                                                                    │ │
│├│ video              4.1 GiB        2  ████████████████████████████▎             70.7%                   │ │
│├│ caché/temp         1.1 GiB        2  ███████▋                                  19.1%                   │ │
│├│ audio              410 MiB        1  ██▊                                        6.9%                   │ │
│├│ comprimidos        150 MiB        1  █                                          2.5%                   │ │
│└│ imágenes          45.0 MiB        3  ▎                                          0.8%                   │ │
│ │ código             128 KiB        1                                             0.0%                   │ │
│ │ documentos        20.5 KiB        1                                             0.0%                   │ │
│ │                                                                                                        │ │
╰─╰────────────────────────────────────────────────────────────────────────────────────────────────────────╯─╯
 Total 5.8 GiB · 0 elementos marcados — 0 B a liberar
 Enter: archivos más pesados  Esc: volver
```

## Configuración

`~/.config/ramas/config.toml` (o `--config ARCHIVO`); todas las claves son
opcionales. Las opciones de línea de comandos tienen prioridad.

```toml
language = "es"            # en | es
icons = "nerd"             # nerd | unicode | ascii
theme = "gruvbox"          # dark | light | high-contrast | gruvbox | custom

[categories]               # una clave existente reemplaza su lista, una nueva agrega categoría
video = ["mp4", "mkv", "webm"]
models = ["gguf", "safetensors"]

[custom_theme]             # se usa con theme = "custom"
base = "dark"              # los colores no indicados salen de este tema
accent = "#ff8800"
selected_bg = "blue"
```

Claves del tema personalizado: `fg bg dim accent border connector dir
selected_fg selected_bg size marked error warn bar_low bar_mid bar_high
status_fg status_bg grew shrank new matched`.

## Formato de exportación

Un objeto JSON con un encabezado y una lista plana de nodos en orden de
profundidad, cada uno con el índice de su padre, así cualquier profundidad
hace round-trip. Los tamaños son por nodo; los totales de directorio se
recalculan al importar. Exportar un escaneo importado da un archivo idéntico
byte a byte.

## Desarrollo

```sh
scripts/ci.sh                                   # fmt --check, clippy -D warnings, tests
cargo run --release --example bench_scan -- /usr
cargo run --release --example bench_ui          # árbol sintético de 1M entradas
cargo run --example screenshots -- es           # las capturas de este README
```

Los tests de borrado solo tocan directorios temporales; el de papelera corre
en un proceso hijo con un `XDG_DATA_HOME` aislado.

## Licencia

MIT OR Apache-2.0.
