---
title: "Find your way around Lime"
description: "Find the toolbox, canvas, Tool Options, Layers, Paths, and workspace controls."
translationKey: "start/workspace"
sourceRevision: "1.2.3-guide-8"
translatedRevision: "1.2.3-guide-8"
---

## Read the workspace {#layout}

![Lime with a night skyline open, tools at the left and Tool Options and Layers at the right.](../../../assets/workspace.jpg)

The screenshot shows Lime's desktop workspace. Panel sizes and the selected tool may differ on your computer.

- **Toolbox, left:** choose an editing tool. The foreground and background color swatches sit at its top.
- **Canvas, center:** view and edit the image. Each open document has a tab. The checkerboard represents transparency, not a printed background.
- **Tool Options, right:** adjust the selected tool. Hover its **?** for essential gestures and shortcuts. Fold secondary option sections using their headers.
- **Layers / Paths, right:** choose the active layer or switch to **Paths** to manage curves. Most pixel edits affect the active layer, even when other layers are visible.
- **Application menus:** open, save, export, undo, and other commands live in the menus. There is no top editing toolbar.

## Make room for the image {#steps}

1. Resize the docked panels to suit your work, or fold the toolbox.
2. Choose **View → Toggle Right Sidebar**, use its header button, or press **Cmd/Ctrl+Shift+L** to fold Tool Options and Layers/Paths together.
3. Use the top button on the compact sidebar to expand it. Finish an active drag before folding; a resting transform preview can remain open.
4. Hold **Space** and drag to pan. Choose **Zoom** to zoom around a point without changing image pixels.

Lime remembers personal folding choices. Tabs keep their own zoom and pan. A folded sidebar retains the selected Layers/Paths tab and drafts.

The compact sidebar matches the folded toolbox width. It keeps essential tool controls at the top, layer thumbnails in the larger middle section, and path blocks below. Each section scrolls independently without a visible scrollbar; subtle edge fades indicate more content. Use **Return** on the canvas to apply or **Escape** to cancel Crop, Gradient, and transform previews; when editing a numeric field, these keys finish or restore that field first. Click the current tool heading to expand all options. Move shows two vertically stacked buttons: **Center Horizontally** and **Center Vertically**. When no image is open, every tool keeps its heading and hides its compact controls. Flip uses an **H/V** toggle. Gradient's single color-mode button toggles between **F→B** (foreground to background) and **F→A** (foreground to transparent). Text offers size and a clickable color swatch.

Use **+** immediately after the last layer thumbnail to open the New Layer dialog. It scrolls with the list and appears only when an image is open. Click a layer thumbnail to select it, double-click to rename, right-click for its context menu, or drag to reorder it. Hover for its name and status. An eye appears on the active or hidden layer and when hovering over other thumbnails. Click the empty area of a path block to select it; its eye shows or hides the path without changing the selected path. Hover for the path number and anchor count. Compact and expanded panels remember their own scroll positions.

Use **Tab/Shift+Tab** to focus compact buttons and **Return/Space** to activate them. On a numeric field, Return/Space starts editing; Return finishes the edit and Escape restores it. Hover a shortened value to see it in full. Long numeric edits scroll to keep the caret visible. The Path tool’s **Stroke** button immediately uses the current stroke settings. Apply or cancel a canvas preview before changing layer or path visibility.

## Use the menus {#menus}

On macOS, the menus are in the system menu bar. On Windows, they use the native window menu. Linux has an in-window menu bar: **F10** activates it, **Alt** plus an underlined letter opens a menu, arrows navigate, and **Enter** chooses. Submenus open beside the parent menu. **Right** or **Enter** opens a submenu; **Left** or **Escape** closes one level. **Escape** at the top level dismisses the menu.

## Next steps {#related}

- [Make your first edit](../../start/first-edit/)
- [Layers and transparency](../../basics/layers/)
- [Keyboard shortcuts](../../reference/shortcuts/)
