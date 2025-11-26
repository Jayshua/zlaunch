# zlaunch

A fast application launcher and window switcher for Linux, built with [GPUI](https://github.com/zed-industries/zed).

## Features

- **Application launching** - Fuzzy search through desktop entries with icons
- **Window switching** - Quickly switch between open windows
- **Daemon architecture** - Runs in background for instant response
- **Compositor support** - Native integration with Hyprland and KDE/KWin

## Usage

Run the daemon:
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

## Building

```bash
cargo build --release
```

## Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate items |
| `Enter` | Launch/switch |
| `Escape` | Hide |

## License

MIT
