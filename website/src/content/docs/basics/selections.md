---
title: "Selections and soft edges"
description: "Limit edits, combine selections, soften edges, and paint an outline around a selection."
translationKey: "basics/selections"
sourceRevision: "1.2.3-selection-move-1"
translatedRevision: "1.2.3-selection-move-1"
---

## What a selection changes {#meaning}

A selection limits where painting, fills, and many effects can change pixels. It does not choose the active layer. A soft selection can give a pixel partial coverage, so an edit fades near the boundary.

The selection outline marks a boundary; it does not show every fractional edge pixel. Zoom in to judge the result. Selection-only changes can be undone without marking an otherwise saved project as modified.

## Make and combine selections {#steps}

1. Choose **Rectangle Select**, **Oval Select**, **Free Select**, or **Magic Wand** under **Select**, or use the toolbox.
2. Drag a rectangle/oval, close a free outline, or click a connected region with the wand.
3. Hold **Shift** to add, **Cmd/Ctrl** to subtract, or **Shift+Cmd/Ctrl** to intersect when making another selection. Use Command on macOS and Ctrl on Windows/Linux.
4. Paint or fill on the active layer. Choose **Select → Select None** to remove the limit before editing elsewhere.

Choose **1:1** with Oval Select for a circle. Rectangle and Oval share an aspect ratio; Crop remembers a separate ratio. Choosing a fixed ratio also refits the current editable shape around its center.

## Antialias and feather {#soft-edges}

**Antialias edges** smooths curves and slanted edges over a small fringe. **Feather new selections** spreads coverage over a broader radius. These are different controls and can be combined.

For a selection that already exists, **Select → Feather…** changes its coverage. **Grow…** expands it and **Shrink…** contracts it while preserving soft coverage. Changing the new-selection feather preference alone does not change the current selection.

After combining shapes or editing their coverage, a selection may become a mask without the original rectangle/oval resize controls. You can still move it: choose Rectangle Select or Oval Select and drag a selected area. Holes and soft edges move with it. A selection mask and a layer mask serve different purposes: the former limits editing; the latter controls a layer's visibility.

## Paint a border around a selection {#stroke}

Use this for a rectangular frame, a circle, or a free-shaped border.

1. Make the selection. For a crisp geometric border, leave **Feather new selections** off; choose Antialias separately if you want smoother curved edges.
2. Choose **Layer → New Layer…**, name it **Border**, and use **Transparent** fill. Keep it visible and Alpha Lock off. This gives the border its own layer; creating the layer keeps the selection in place.
3. Set the foreground color, then choose **Edit → Stroke Selection…**.
4. Choose **Pencil** for a hard stroke or **Brush** for adjustable hardness. Set **Size (px)**, start with 100% Opacity and Buildup off, then click **Stroke**. Size controls the border's thickness in image pixels.
5. Choose **Select → Select None** to inspect the painted border. Use **Edit → Undo** if you want to try a different size.

**Expected result:** the border is painted onto the active raster layer along the selection edge. The brush extends to both sides of that edge, rather than being clipped to the selected interior. Leave room inside the canvas for the full stroke. Changing the selection later does not redraw the border.

**Edit → Stroke Selection…** follows a selection you made and paints on the active layer. **Filters → Stroke…** instead uses a layer's visible silhouette to generate an outside-only outline on a new layer beneath it. For example, use the filter to outline [text](../../guides/text-effects/) without first drawing a selection.

## Related guides {#related}

- [Selection tools](../../reference/selection/)
- [Remove a background](../../guides/remove-background/)
- [Layer masks](../../basics/masks/)
- [Copy selected pixels into another image](../../guides/combine-images/#clipboard)
