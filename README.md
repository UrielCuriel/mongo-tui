# mongo-tui

[![CI](https://github.com/ucuriel/mongo-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/ucuriel/mongo-tui/actions)

Un cliente de MongoDB basado en terminal (TUI) escrito en Rust.

## Características

- Interfaz visual en terminal para gestionar tus bases de datos MongoDB.
- Navegación intuitiva por colecciones y documentos.
- Multiplataforma (Linux, macOS, Windows).

## Instalación

### Binarios Precompilados

Puedes descargar la última versión para tu sistema operativo desde la sección de [Releases](https://github.com/ucuriel/mongo-tui/releases).

### Desde el código fuente

Si tienes Rust instalado:

```bash
git clone https://github.com/ucuriel/mongo-tui.git
cd mongo-tui
cargo install --path crates/tui_app
```

## Uso

Una vez instalado, simplemente ejecuta:

```bash
mongo-tui-app
```

### Opciones

```bash
mongo-tui-app --help
```

- `-t, --tick-rate <FLOAT>`: Tasa de actualización (por defecto: 1.0).
- `-f, --frame-rate <FLOAT>`: Tasa de fotogramas (por defecto: 60.0).

## Desarrollo

Este proyecto utiliza un workspace de Cargo con los siguientes crates:

- `mongo_core`: Lógica de conexión y operaciones con MongoDB.
- `tui_app`: La aplicación de interfaz de usuario (Ratatui).

Para correr el proyecto en modo desarrollo:

```bash
cargo run -p mongo-tui-app
```

## Licencia

Este proyecto está bajo la licencia MIT.
