---
title: "Draw and use paths"
description: "Edit Bézier anchors, convert paths to selections, and stroke a path."
translationKey: guides/paths
sourceRevision: "path-gestures-4"
translatedRevision: "path-gestures-4"
---

## Draw a path {#steps}

1. Choose **Path** (`P`). Click to place corner anchors, or drag as you place an anchor to pull curve handles.
2. **Enter** or choosing another tool finishes editing without closing the path. The next click on empty canvas starts a new path. To edit an existing path, click one of its nodes. To continue an open path, select either endpoint, then click to add nodes at that end. Select an endpoint or press Enter before adding nodes after editing an interior node.
3. Drag a node square to move it immediately. Only selected nodes show handles; drag those handles to bend the curve. Use **Cmd** on macOS and **Ctrl** on Windows/Linux for the following edits. **Cmd/Ctrl-drag** a node to pull symmetric handles without moving the node. **Cmd/Ctrl-click the first node** to deliberately close an outline with at least three nodes. Escape ends editing without closing and discards an unfinished drag. **Cmd/Ctrl-click** (or **Alt/Option-click**) a visible curve to insert a node without changing its shape; continue dragging to reposition it if needed.
4. Shift-click anchors to select a group. Drag one selected anchor to move the group, or use **Corner**, **Smooth**, and Delete/Backspace on the group. **All Anchors** and **Deselect** are in Tool Options.
5. Use **Paths**, beside Layers, to select a path and toggle its visibility. Hidden paths stay unchanged when drawing: even if a hidden path is selected, clicking empty canvas starts a new visible path.

## Make a selection or paint the curve {#use}

**Select → Visible Paths to Selection** combines visible closed paths into a selection. Open paths cannot enclose a selection. **Select → Selection to Path** creates closed paths from the current selection.

To paint a path, select a visible path and a visible, paintable raster layer. **Edit → Stroke Path…** opens Pencil/Brush settings such as size, opacity, hardness, flow, and buildup. Choose **Stroke** to paint with the foreground color. The **Paths** panel's **Stroke** button applies the shared settings immediately without opening that dialog.

## Expected result {#result}

The project retains the editable path independently of the pixels it strokes. Editing a path afterward does not redraw an earlier stroke. A path remains a guide until it is converted or stroked; simply displaying it does not add it to exported artwork.

## Handle behavior {#handles}

**Independent** moves one handle alone. **Aligned** keeps the opposite handle on the same line while preserving its length. **Symmetric** mirrors its direction and length. These settings control future handle drags. **Corner** collapses selected handles; **Smooth** aligns or creates them.

For text along a path, remove hard line breaks first; multiline text is rejected even if it fits a fixed box. Select visible editable text and a visible path, then use **Layer → Text Along Path…**. The preview fits one line with alignment, offsets, and Reverse Direction. Apply makes a new raster layer and hides the original text; the source and path remain available in the project.

## Related guides {#related}

- [Selections and soft edges](../../basics/selections/)
- [Add text and effects](../../guides/text-effects/)
- [Painting tool reference](../../reference/painting/)
