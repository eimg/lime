---
title: "Layers and transparency"
description: "Create and organize layers, control their appearance, and use locks and movement links."
translationKey: "basics/layers"
sourceRevision: "1.2.3-guide-2"
translatedRevision: "1.2.3-guide-2"
---

## Choose the layer you edit {#active-layer}

A document is a stack of layers. Transparent parts of an upper layer reveal the layers beneath it. The checkerboard shows areas where the combined image is transparent.

Click a row in **Layers** to make it active. Painting, fills, and most effects affect that layer, not every visible layer. Use **File → Open as Layers…** to add another image to the same document.

## Create and organize the stack {#organize}

- **Create:** choose **Layer → New Layer…**, give it a useful name, choose a Fill, and click **Create**. **Transparent** gives you an empty canvas-sized layer for painting; **Foreground** or **Background** fills it with that swatch. The new layer goes above the active one.
- **Rename:** select the row and choose **Layer → Rename Layer…**, or right-click it and choose **Rename Layer…**. Names such as Background, Photo, and Caption make a stack easier to navigate.
- **Duplicate:** choose **Layer → Duplicate Layer** to make a copy before experimenting. Editable text stays editable in the duplicate. The copy starts without movement links or source attachments.
- **Reorder:** drag a row up/down, or use **Layer → Raise Layer / Lower Layer**. Keep an opaque background below artwork so it does not cover it.
- **Delete:** select the intended row and choose **Layer → Delete Layer**, or use the layer context menu's **Delete Layer** item. **Edit → Undo** restores an accidental deletion. A document must keep at least one layer. Delete/Backspace on the canvas clears pixels instead of deleting the layer.

## Control the appearance {#appearance}

1. Toggle a row's visibility to compare the image with and without it. Apply or cancel any Crop, transform, or Gradient preview first.
2. Choose **Layer → Layer Opacity…** to make the whole layer more transparent.
3. Use **Layer → Raise Layer** or **Lower Layer** to change stacking. Higher layers cover lower layers where their pixels are opaque.
4. Choose a blend mode in the Layers panel when you want overlapping colors to interact. Start with **Normal** for ordinary layering. Try **Multiply** for dark linework on white over a colored layer: white stops covering the color beneath while the dark lines remain. Try **Screen** for a light texture on black over a photo: black stops covering the photo while light details brighten it. Reduce layer opacity if the result is too strong; switch back to Normal for ordinary stacking.

**Expected result:** the combined canvas changes while each layer remains separate. Save as `.lime` to retain the stack. Export creates a flattened image.

## Merge only when you want one layer {#merge}

Select the upper of two adjacent raster layers and choose **Layer → Merge Down**. They become one raster layer with their opacity, masks, and effects baked into the pixels. You can no longer move or revise the two parts separately without Undo or an earlier project copy. The result also loses those layers' movement links and attachments.

Both layers must be visible, rasterized, and free of alpha/position locks. Blend modes or image-size limits can also make Merge Down unavailable; **Normal** is the simplest starting point. Keep separate layers when you still need to edit them independently. Export already combines the visible artwork into a separate image, so merging is not required for sharing.

## Locks and moving together {#locks-links}

- **Alpha Lock** preserves the existing transparency of a raster layer. You can recolor painted pixels; erasing cannot remove them. It is unavailable for editable text.
- **Position Lock**, shown as a padlock in each row, blocks manual movement and layer transforms. Document-wide image operations still affect locked layers.
- **Movement Link**, the chain toggle, puts enabled layers in one movement set. Moving any member also moves the other enabled members, including hidden layers. A position lock in the set blocks the whole move.

New shadow and outline layers follow their source through a directional attachment. Moving the source moves its followers; moving a shadow directly changes its offset. Choose **Layer → Detach from Source** to make it independent. This attachment does not regenerate the shadow after source edits.

## Related guides {#related}

- [Combine and position images](../../guides/combine-images/)
- [Layer masks](../../basics/masks/)
- [A layer cannot move or transform](../../troubleshooting/moving/)
