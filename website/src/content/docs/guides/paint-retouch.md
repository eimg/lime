---
title: "Paint and retouch"
description: "Paint controlled strokes, erase pixels, and clone a clean source area."
translationKey: guides/paint-retouch
sourceRevision: "1.2.3-tool-feedback-1"
translatedRevision: "1.2.3-tool-feedback-1"
---

## Paint a controlled stroke {#steps}

1. Select a visible raster layer. For a separate painted overlay, choose **Layer → New Layer…** first.
2. [Choose a foreground color](../../basics/colors/) using the toolbox swatch. Choose **Brush** for a soft edge or **Pencil** for a hard edge.
3. Set brush size, opacity, and hardness where available. `[` and `]` change size. Select **Light** or **Strong** under **Smoothing** if you want a steadier Brush trace.
4. Drag to paint. Release to commit one stroke; press Escape before releasing to discard it. Hold Shift to see a straight-line guide from the previous stroke endpoint, then click to paint that line. The guide follows zoom and pan without changing pixels.
5. Use **Eraser** (`E`) to remove alpha, or **Blur Brush** / **Sharpen Brush** from **Tools** to retouch local detail.

## Expected result {#result}

Pixels change only on the active layer, inside the canvas and current selection. One stroke is one undo step. Brush settings are remembered separately for each tool across app restarts.

## Clone a clean area {#clone}

Choose **Clone Stamp** (`Shift+C`). Alt-click an inside-canvas source on the visible raster layer, then paint over the destination. Turn on **Sample All Layers** to sample the visible composition while still painting only the active raster layer.

**Aligned** preserves the source-to-destination offset across strokes. Turn it off to start each stroke from the chosen source. **Show Source** marks the sampled position. Pick a new source after switching tabs/layers or changing layer/canvas geometry; changing sampling also clears it.

## Control repeated coverage {#coverage}

Opacity caps a stroke relative to the pixels at its start. Painting repeatedly within that stroke does not exceed the cap. With **Buildup** on, **Flow** accumulates each dab toward it. A new stroke can add another pass. Blur/Sharpen call the limit **Strength**.

If painting does nothing, check the selection, layer visibility, Alpha Lock, and zero opacity/flow. Alpha Lock confines painting to existing alpha and prevents Eraser from removing pixels.

## Related guides {#related}

- [Painting tool reference](../../reference/painting/)
- [Painting changes nothing](../../troubleshooting/painting/)
- [Layers and transparency](../../basics/layers/)
