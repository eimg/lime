---
title: "Keyboard shortcuts"
description: "Platform modifiers, tool keys, canvas actions, and text-editing exceptions."
translationKey: reference/shortcuts
sourceRevision: "1.2.3-tool-feedback-1"
translatedRevision: "1.2.3-tool-feedback-1"
---

Use **Cmd** on macOS and **Ctrl** on Windows/Linux wherever this page says **Primary**. Plain tool letters work when focus is on the workspace; typing in a field or text layer keeps those letters as text.

## Files and workspace {#workspace}

| Action | Shortcut |
| --- | --- |
| New image / Open | Primary+N / Primary+O |
| Open as Layers | Primary+L |
| Save / Save As | Primary+S / Primary+Shift+S |
| Save Project As | Primary+Alt+S |
| Export | Primary+Shift+E |
| Close tab | Primary+W |
| Next / Previous tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Undo | Primary+Z |
| Redo | Cmd+Shift+Z on macOS; Ctrl+Y elsewhere |
| Toggle right sidebar | Primary+Shift+L |
| Toggle grid / Rulers and guides | Primary+G / Primary+R |
| Toggle layer boundary | Primary+B |
| Fit view / 100% view | Primary+Shift+J / 1 |
| Zoom in / out | + / − |
| Pan | Hold Space and drag |

## Tools and selection {#tools}

| Tool or action | Shortcut |
| --- | --- |
| Move / Crop / Zoom / Eyedropper | M / C / Z / K |
| Rectangle / Oval / Free Select / Magic Wand | R / O / F / W |
| Eraser / Bucket Fill / Gradient | E / B / G |
| Text / Path | T / P |
| Scale Layer / Rotate / Flip / Perspective | Shift+S / Shift+R / Shift+F / Shift+P |
| Clone Stamp / Pick clone source | Shift+C / Alt-click |
| Brush smaller / larger | [ / ] |
| Select All / Select None / Invert Selection | Primary+A / Primary+Shift+A / Primary+Shift+I |
| Clear selected pixels to transparency | Delete / Backspace |
| Add / Subtract / Intersect selection | Shift / Cmd/Ctrl / Shift+Cmd/Ctrl while selecting |
| Swap foreground and background | X |

Pencil, Brush, Blur Brush, and Sharpen Brush have no plain tool key. Choose them from the toolbox or Tools menu. Default Colors has no shortcut.

## Layer and effect commands {#layers}

| Action | Shortcut |
| --- | --- |
| New layer / Duplicate layer | Primary+Shift+N / Primary+Shift+D |
| Fit layer / Layer to canvas size | Primary+F / Primary+I |
| Crop image to content | Primary+Shift+C |
| Drop Shadow / Outside Stroke | Primary+D / Primary+Shift+O |
| Repeat / Reopen last effect | Primary+Alt+F / Primary+Alt+Shift+F |
| Move nudge / larger nudge | Arrow / Shift+Arrow with Move active |

## Context matters {#context}

Return on the canvas applies Crop/Gradient/transform previews, closes Free Select, or finishes Path editing without closing the path. Cmd/Ctrl-click the first Path node to close it deliberately. Escape cancels those previews; in Path it ends editing without closing. In a numeric field, Return finishes the field and Escape restores its prior value instead.

Text is different: Return adds a line; Escape or Primary+Enter commits typing. Undo steps through typing before image history. A focused sidebar toggle uses Return/Space to fold, and a focused Tool Options section header uses them to expand/collapse.

Delete/Backspace edits characters while typing in a field or text layer. With Path active, it deletes anchors instead of pixels. Otherwise it runs **Edit → Clear to Transparency** on the active raster layer; without a selection limit, it clears the whole layer. Alpha Lock protects pixels from clearing.

## Related guides {#related}

- [Workspace](../../start/workspace/)
- [Previews and undo](../../basics/previews/)
