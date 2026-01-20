# Tauri Icons

This directory should contain application icons for various platforms.

## Generating Icons

To generate icons from a source PNG (1024x1024 recommended):

```bash
pnpm tauri icon path/to/icon.png
```

This will generate all required icon sizes for different platforms.

## Required Icons

- icon.png (default icon)
- Various platform-specific sizes for Windows, macOS, Linux

## Placeholder

For development, Tauri will use default icons if custom ones are not provided.
