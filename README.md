# Lime

<img src="macos/AppIcon.svg" width="96" height="96" alt="Lime half-slice logo">

**Lite Image Editor** for Windows x64, macOS Apple silicon, and Linux x86_64/ARM64.
Layers, painting, selections, text, and effects in a workspace that puts your canvas first.

[User guide](https://eimg.github.io/lime/) · [မြန်မာလမ်းညွှန်](https://eimg.github.io/lime/mm/)

![Lime editing a night skyline](lime-screenshot.jpg)

## Download

**Lime 1.2.3** · [Release notes](https://github.com/eimg/lime/releases/tag/v1.2.3)

| Platform | Download | Install |
| --- | --- | --- |
| Windows x64 | [EXE](https://github.com/eimg/lime/releases/download/v1.2.3/Lime-1.2.3-windows-x64.exe) | Download and run. No installer. |
| macOS (Apple silicon) | [DMG](https://github.com/eimg/lime/releases/download/v1.2.3/Lime-1.2.3.dmg) | Open and drag Lime to Applications. |
| Linux x86_64 | [tar.gz](https://github.com/eimg/lime/releases/download/v1.2.3/Lime-1.2.3-linux-x86_64.tar.gz) | Extract and run `./lime`. |
| Linux ARM64 | [tar.gz](https://github.com/eimg/lime/releases/download/v1.2.3/Lime-1.2.3-linux-aarch64.tar.gz) | Extract and run `./lime`. |

[SHA-256 checksums](https://github.com/eimg/lime/releases/download/v1.2.3/SHA256SUMS)

Windows requires Windows 10 or later, DirectX, and the Microsoft Visual C++
2015–2022 x64 runtime. Linux requires glibc 2.35 or newer, a working Vulkan driver,
an X11 or Wayland session, and a file chooser portal. See the
[Linux requirements](linux/README.txt) for runtime packages.

Windows downloads are unsigned. The macOS app is Developer ID signed and notarized by Apple.

## Features

- **Layers** — masks, blend modes, opacity, position locks, and movement link toggles.
  Shadows follow their source, and enabled movement links stay together.
  Right-click a layer for more actions.
- **Selections and transforms** — rectangle, oval, freehand, magic wand, crop, scale, rotate, and perspective.
- **Painting** — pencil, brush, eraser, clone stamp, gradients, and bucket fill.
  Per-tool settings and recent colors survive app restarts.
- **Text and paths** — editable text with sizes up to 4,096 pixels, font controls,
  wrapping, and editable vector paths.
  Select letter shapes with Alpha to Selection, or preview and adjust text along
  a path while keeping the original text.
- **Color and effects** — levels, curves, blur, sharpen, drop shadows, and editable layer effects.
  Drop shadows and outside strokes work on editable text; later text edits leave
  those effect layers unchanged.
- **PDF and recovery** — import selected PDF pages, export a flattened PDF, and recover interrupted work.
  Preserved recovery copies retry at startup; an unchanged failure warns once.

## Getting started

Open an image or PDF, or create a new image. Choose a tool and adjust its options
beside the canvas. Hover the **?** for gestures and shortcuts.
Collapse the right sidebar with its header button, **Cmd/Ctrl+Shift+L**, or
**View → Toggle Right Sidebar** to give the canvas more room. The compact sidebar
keeps essential tool controls, layer thumbnails with a **+** after the last layer, and path blocks available;
each section scrolls independently without scrollbars. Use its top button to expand it.
Finish an active drag before toggling the sidebar.
Apply or cancel canvas previews before changing layer or path visibility.
Fold secondary option sections with the mouse or keyboard to leave more room for Layers and Paths;
their headers summarize active settings. Oval Select uses **1:1** for circles.
Aspect-ratio choices also reshape the current selection or crop box.
Typed numeric edits finish before Save and layer menu commands.
Delete/Backspace clears the selected area of the active raster layer to transparency.
Move uses a four-arrow cursor and always moves the active layer.
Its compact options center the layer horizontally or vertically; tool controls
appear when an image is open.
New text appears above all layers, regardless of the selected layer.
Layer → Transform and the toolbox use the same Scale Layer, Rotate, Flip, and
Perspective tools. Reselecting an active transform keeps its preview.
Use Edit → **Stroke Path…** or **Stroke Selection…** to choose
stroke settings; the Paths panel's **Stroke** button applies the shared settings
directly. **Visible Paths to Selection** combines visible closed paths.
For text requiring an input method, compose it elsewhere and paste it into Lime.

Crop and paint strokes can start outside the canvas; crop stays within the image.
Crop ignores drags whose area stays entirely outside the image.
The status bar shows the current app version. **About Lime** includes source and
issue links. Find it and **Check for Updates** in the Lime menu on macOS or Help
on Windows/Linux; both open inside the editor.
Linux menus open cascading submenus beside their parent menu, with underlined
letters for keyboard access.

Use the reset arrow in **Tool Options** or **Tools → Reset Current Tool** to restore
that tool's defaults. **Tools → Reset All Tool Settings…** resets every tool after
confirmation. Resets preserve artwork, colors, and saved gradient presets.

Save as **`.lime`** to keep layers and editable content. Use **File → Export…**
(**Cmd/Ctrl+Shift+E**) for a flattened copy. Choose PNG, JPEG, WebP, or PDF in the
export dialog; Lime remembers your last successful format and options.

[Report a problem or suggest a feature](https://github.com/eimg/lime/issues).
