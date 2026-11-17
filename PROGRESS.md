# PROGRESS — `ramas`

Protocolo: una tarea por iteración → `scripts/ci.sh` (fmt, clippy -D warnings,
test) → commit → actualizar este archivo.

## Fase 0 — Base
- [x] 0.1 Crate, licencias, lints, `scripts/ci.sh`, SPEC/PROGRESS
- [x] 0.2 Esqueleto de módulos (`scan`, `tree`, `ui`, `actions`, `config`, `export`) y CLI mínima
- Criterio: compila y los tests corren. ✔ (fase 0 cerrada)

## Fase 1 — Motor de escaneo
- [x] 1.1 Árbol en arena (nodos, nombres en blob, agregados, orden, rutas)
- [x] 1.2 Walker paralelo: tamaños disco/aparente, no seguir symlinks, errores por nodo
- [x] 1.3 Hard links una vez, `-x`, salto de pseudo-FS
- [x] 1.4 Progreso (atómicos) y cancelación
- [x] 1.5 Benchmark simple (`examples/bench_scan.rs`)
  - 2026-11-17, `/usr` (398 737 entradas, btrfs, NVMe): frío 19.9 s; caliente 0.17-0.20 s
    (~2M entradas/s). `du -sx /usr` caliente: 0.63 s. Orden por tamaño: 0.05 s.
    Arena ~48 MiB (capacidad del Vec; ~72 B/nodo + nombres). Conteo = `os.walk` + raíz.
- Criterio: tests contra árboles temporales con tamaños conocidos; benchmark anotado. ✔ (fase 1 cerrada)

## Fase 2 — TUI mínima
- [x] 2.1 Terminal guard + panic hook, loop de eventos, hilo de escaneo, pantalla de progreso
- [x] 2.2 Vista aplanada (filas visibles), panel de árbol con conectores, barras gradiente
- [x] 2.3 Navegación (flechas, hjkl, PgUp/PgDn/Home/End, Enter), `s`, `a`, `q`
- Criterio: navegación verificada con `TestBackend`. ✔ (fase 2 cerrada; smoke pty OK, terminal restaurada)

## Fase 3 — Panel lateral y barra de estado
- [x] 3.1 Panel de detalle (ruta, tamaños, conteos, mtime, permisos, top 10, desglose)
- [x] 3.2 Barra de estado (total, libre, marcados, errores) + ayuda contextual + `p` + `?`
- Criterio: el detalle coincide con el nodo seleccionado. ✔ (fase 3 cerrada)

## Fase 4 — Marcado y borrado en lote
- [x] 4.1 Conjunto de marcas con deduplicación padre/hijo
- [x] 4.2 Validación de seguridad y borrado (papelera/permanente) con resultados por ruta
- [x] 4.3 Diálogos (confirmación, texto `delete`/`borrar`, resumen), actualización del árbol, `--read-only`
- [x] 4.4 `r` reescanear nodo
- Criterio: tests de papelera, permanente, dedup y rutas prohibidas. ✔ (fase 4 cerrada; smoke pty de borrado permanente en copia del fixture OK)

## Fase 5 — Búsqueda y filtros
- [x] 5.1 `/` búsqueda difusa, `n/N`, expansión automática
- [x] 5.2 `f` panel de filtros (tamaño, extensión, antigüedad), atenuar/ocultar
- Criterio: tests de coincidencia y expansión automática. ✔ (fase 5 cerrada)

## Fase 6 — Análisis por tipo
- [x] 6.1 Clasificador configurable
- [x] 6.2 Vista `t` y lista por categoría marcable
- Criterio: categorías correctas sobre árbol de prueba. ✔ (fase 6 cerrada)

## Fase 7 — Mouse, temas, iconos, idioma
- [x] 7.1 Mouse (clic, doble clic, scroll)
- [x] 7.2 Temas (4 + custom), `T`, `NO_COLOR`
- [x] 7.3 Iconos Nerd Font / Unicode / ASCII (`--no-icons`)
- [x] 7.4 i18n EN/ES (`--lang`, `language`)
- Criterio: todos los temas renderizan; fallback sin Nerd Font. ✔ (fase 7 cerrada)

## Fase 8 — Exportar / importar / diff
- [x] 8.1 `--export`, `--import`, tecla `e`
- [x] 8.2 `--diff`
- Criterio: round-trip JSON idéntico; diff detecta cambios. ✔ (fase 8 cerrada; `--export` → `--import` + `e` da un archivo byte-idéntico, verificado con `cmp`)

## Fase 9 — Rendimiento y pulido
- [x] 9.1 Árbol sintético 1M, perfilado y resultados
  - 2026-11-17, release, 12 hilos, tmpfs. **Escaneo en disco** (1000 dirs × 1000 archivos vacíos, 1 001 001 entradas,
    `examples/bench_scan.rs`): 0.28–0.36 s (~3M entradas/s); `du -s` 0.85 s. Orden por tamaño 0.05 s.
    Arena 85 MiB tras `shrink_to_fit` (antes 100 MiB por capacidad sobrante). Pico RSS del bench 245 MiB
    (árbol temporal del walker + arena durante la fusión).
  - **TUI real** sobre ese árbol (pty, 12 teclas: navegar, buscar, ordenar, vista de tipos, salir):
    RSS máximo 216 MiB, sin retrasos perceptibles, terminal restaurada.
  - **Operaciones de UI** (`examples/bench_ui.rs`, árbol sintético en memoria 1M): `App::new` 58 ms; primer
    frame con estadísticas de la raíz 62 ms; frames siguientes 0.7–0.8 ms (solo se renderiza la ventana
    visible); expandir dir de 1k hijos <0.1 ms; orden por nombre 237 ms; búsqueda difusa por tecla 67 ms;
    filtro ext+tamaño 22 ms; export JSON 216 ms (119 MiB); import 618 ms.
  - Conclusión: fluido; el costo O(n) por tecla (búsqueda, orden por nombre) queda <250 ms con 1M.
    Mejora posible no hecha: walker que fusione por directorio para bajar el pico transitorio (~130 MiB).
- Criterio: UI fluida y memoria razonable, anotado aquí. ✔ (fase 9 cerrada)

## Fase 10 — Distribución
- [x] 10.1 Binario musl estático + `ldd` + contenedores Alpine/Debian
  - `scripts/build-static.sh [--containers]`: `static-pie linked`, `ldd` → "statically linked", 2.8 MiB.
  - Alpine 3.24 y Debian 13 (trixie) vía docker (imágenes locales): `--version`, `--export`, `--diff`
    y la TUI sobre `/etc` funcionan. Con podman el pull de debian falló por credenciales guardadas
    de docker.io; se usó docker, que la tenía local.
- [x] 10.2 README (EN/ES), `install.sh`, `--help`, página man
  - `README.md` (inglés) + `README.es.md` (español), capturas generadas con `examples/screenshots.rs`
    (TestBackend, árbol sintético determinista), tabla de teclas, seguridad, config, formato.
  - `install.sh [--prefix DIR] [--uninstall]`: usa `dist/ramas` o compila (musl si está el target).
    Probado en un prefijo del scratchpad.
  - `--help` con teclas, reglas de borrado, config y ejemplos; `man/ramas.1` generado con
    `ramas --man` y un test que falla si quedó desactualizado.
- Criterio: `ldd` estático y arranque en Alpine y Debian. ✔ (fase 10 cerrada)

## Decisiones
- UI en inglés por defecto, español con `--lang es` (decisión del usuario).
- Escaneo con rayon + std::fs (ver SPEC).
- 1.2 y 1.3 en un solo commit: hard links, `-x` y pseudo-FS viven en el mismo walker.
- Pseudo-FS: rutas fijas `/proc`, `/sys`, `/dev` + `statfs.f_type` (proc, sysfs, cgroup, debugfs...) solo en puntos de montaje. Se muestran con tamaño 0 y flag SKIP_PSEUDO.
- Hard link duplicado: tamaño 0 en el árbol, flag HARDLINK; el primero en orden DFS se lleva el tamaño.
- `-x`: no se testea con un montaje real (requiere root); test unitario de `skip_flag`.

- 2.1-2.3 en un commit: el loop, la vista y la navegación dependen entre sí.
- `i18n.rs` creado ya en fase 2 (tablas EN/ES completas) para no reescribir cadenas después; 7.4 solo cablea `--lang`/config.
- Smoke TUI: `drive.py` (pty.fork + pyte en venv del scratchpad), `script` no está instalado.
- Tamaños en unidades binarias (KiB, MiB, GiB).

- Clasificador (`classify.rs`) adelantado a fase 3 porque el panel necesita el desglose por tipo; 6.1 solo agrega la carga desde TOML. `ts` = código (TypeScript), no video.
- Fechas en zona local vía `jiff` (ya era dependencia transitiva de `trash`); sin tzdata cae a UTC.
- Estadísticas del panel (top 10 + desglose) cacheadas por (nodo, métrica, generación del árbol).

- Borrado en hilo aparte (popup "Borrando…"); el árbol se actualiza con `remove_subtree` sin reescanear. "Liberado" usa tamaño en disco.
- `Espacio` marca y baja una fila (marcado en lote rápido). `u` limpia marcas. `Esc` ya no sale (solo `q`), evita salidas accidentales.
- Se rechaza también cualquier ancestro de `$HOME` (lo contiene). Rutas con `..` rechazadas.
- Reescanear la raíz quita el modo importado.

- Búsqueda: `nucleo-matcher`, smart case, solo nombres (no la ruta). Resultados por puntaje; al teclear se salta al mejor y se expanden sus ancestros; con Enter se expanden las ramas de los 100 mejores. `Esc` vuelve al nodo de origen.
- Filtros: un archivo coincide si cumple todo; un directorio si contiene un archivo que coincide (o, con solo tamaño mínimo, si su total alcanza). Modo atenuar/ocultar. Los tamaños del filtro siguen la métrica activa (`a`).

- Vista `t`: lista hasta 500 archivos por categoría; `Espacio` marca, `d` abre el mismo diálogo de borrado.
- Incidente: el commit de fase 5 entró con un test roto (un `sed` no aplicó y la cadena `&&` cortó antes del CI). Arreglado en 73b7b5e; `scripts/ci.sh` ahora devuelve el código de salida real y los commits se encadenan a él.

- Iconos: Unicode por defecto; `nerd` opcional; `ascii` (`--no-icons`) también cambia bordes, barras (`#`) y spinner. Test: sin glifos de uso privado (U+E000–F8FF) fuera de modo nerd.
- Temas con fondo explícito (light, high-contrast, gruvbox) pintan todo el frame; dark usa el fondo de la terminal. `NO_COLOR` no vacío = sin colores (solo modificadores).
- Prioridad de opciones: flag CLI > config TOML > valor por defecto.

- Formato JSON plano v1 (registro por nodo con índice del padre): sin límite de recursión de serde_json. Tamaños propios por nodo; totales recalculados al importar. Nombres no UTF-8 en `nb` (bytes). Hijos en orden canónico por nombre: la salida no depende del orden en pantalla.
- `--export` escanea sin TUI (progreso en stderr si es terminal) y sale. `e` escribe `ramas-<raíz>-<fecha>.json` en el directorio actual.
- `--diff`: emparejado por ruta relativa desde las raíces (permite comparar un árbol movido). Columna delta en el árbol, línea Δ en el panel, "diff vs" en la barra.

## Bloqueos
(ninguno)

## Estado final
Todas las fases (0-10) completas y verificadas: 144 tests, `cargo fmt --check` y
`cargo clippy --all-targets -- -D warnings` limpios, binario musl estático probado en
Alpine 3.24 y Debian 13.

Pendientes conocidos (no bloquean ningún criterio):
- `-x` solo tiene test unitario (un montaje real requiere root).
- Pico de memoria transitorio del escaneo (~130 MiB extra por millón de entradas).
- Sin repositorio remoto ni releases publicados.

LOOP_COMPLETE
