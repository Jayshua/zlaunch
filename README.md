# zlaunch

A fast application launcher and window switcher for Linux Wayland, built with
[GPUI](https://github.com/zed-industries/zed).

<p align="center">
<video src="https://github.com/user-attachments/assets/e11cd113-798d-4b8c-84d6-36c0ff0dc3d6" width="80%" controls></video>
</p>

## Features

- **Application launching** - Fuzzy search through desktop entries with icons
- **Window switching** - Switch between open windows (Hyprland, KDE/KWin)
- **Calculator** - Evaluate math expressions, copies result to clipboard
- **Web search** - Search Google, DuckDuckGo, Wikipedia, YouTube, and more
- **Emoji picker** - Searchable emoji grid
- **Clipboard history** - Browse and paste from clipboard history
- **AI mode** - Query Gemini API with streaming responses
- **Theming** - 15 bundled themes plus custom theme support
- **Daemon architecture** - Runs in background for instant response

## Building

```bash
cargo build --release
```

The binary will be at `target/release/zlaunch`.

## Usage

Start the daemon:
```bash
zlaunch
```

Control via CLI:
```bash
zlaunch toggle  # Toggle visibility
zlaunch show    # Show launcher
zlaunch hide    # Hide launcher
zlaunch quit    # Stop daemon
```

Theme management:
```bash
zlaunch theme           # Show current theme
zlaunch theme list      # List available themes
zlaunch theme set NAME  # Set theme by name
```

## Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate items |
| `Tab` / `Shift+Tab` | Switch categories |
| `Enter` | Execute selected item |
| `Escape` | Hide launcher |
| `Backspace` | Go back (in submenus) |

## Configuration

Config file: `~/.config/zlaunch/config.toml`

```toml
theme = "dracula"
window_width = 600.0
window_height = 400.0
```

## Theming

### Bundled Themes

ayu-dark, catppuccin-latte, catppuccin-mocha, dracula, everforest,
gruvbox-dark, kanagawa, material, monokai, nord, one-dark, rose-pine,
solarized-dark, synthwave, tokyo-night

### Custom Themes

Place custom theme files in `~/.config/zlaunch/themes/`. Theme files are TOML
format.

Colors can be specified as:
- Hex: `"#3fc3aa"` or `"#3fc3aa80"`
- RGBA: `{ r = 255, g = 128, b = 64, a = 255 }`
- HSLA: `{ h = 0.5, s = 0.8, l = 0.6, a = 1.0 }`

See bundled themes in `assets/themes/` for examples.

## Compositor Support

- **Hyprland** - Window switching via IPC socket
- **KDE/KWin** - Window switching via DBus
- Other compositors should work for application launching but without window switching

## License

MIT
