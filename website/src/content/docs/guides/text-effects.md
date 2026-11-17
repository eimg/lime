---
title: "Add text, shadows, and outlines"
description: "Keep text editable while adding a separate shadow or outside stroke."
translationKey: guides/text-effects
sourceRevision: "text-styles-2"
translatedRevision: "text-styles-2"
---

## Add editable text {#steps}

1. Choose **Text** (`T`) and click an empty area of the canvas. New text appears above all layers, regardless of which layer is selected. Clicking existing text edits it in its current layer position.
2. Type or paste text. Return inserts a new line. For input methods requiring native composition, compose elsewhere and paste into Lime.
3. In **Tool Options**, choose an installed font, size, color, Bold/Italic, spacing, and alignment. Font, size, color, Bold, and Italic affect highlighted text. With nothing highlighted, they change all text in the active layer immediately, including while you are typing. Double-click a word or triple-click a visible line to highlight it; wrapped lines are selected separately. Spacing and alignment always affect the whole layer.
4. Turn on **Fixed box** to enable Width/Height and wrap text inside a box. An overflow notice means some stored text falls outside the visible box; enlarge it to reveal more, or turn Fixed box off to remove the limits.
5. Click outside or press Cmd/Ctrl+Enter to finish. Escape also commits a text draft; it does not discard typing. Choose **Edit → Undo** to reverse edits.

Text size accepts **1–4,096 image pixels**, so you can use sizes such as 300 or 1,024 on high-resolution images. Type the size directly in Tool Options. Very large text still needs to fit within Lime’s layer dimensions and memory limits.

Copy and paste between text layers in Lime keeps the selected styles. Plain text pasted from another app inherits the insertion style. Save as `.lime` to keep every style editable. Emptying the Size field or pressing Escape while editing that field restores the original sizes.

## Add a shadow or outline {#effects}

Select the visible text layer, then choose **Filters → Drop Shadow…** or **Filters → Stroke…**. Adjust the preview and compare **Before** with **After**. Choose a color directly or use **Use Foreground**. **Reset** restores the built-in parameters and black color. **Apply** commits; **Cancel** leaves the document unchanged.

Stroke creates an outside-only outline, including around holes in the letters. Its Width is measured in image pixels, from 0 to 128. Zero width or opacity adds no layer. Drop Shadow and Stroke both add a raster layer directly beneath the source while keeping the text selected and editable.

## Expected result {#result}

The project contains editable text and a separate effect layer. Moving the source also moves its attached effect. Later text edits do not redraw that layer. If the silhouette changes, hide or delete the old effect layer before creating its replacement. Detaching only removes its movement relationship; its old pixels remain visible.

## Troubleshooting {#troubleshooting}

If a named font is missing, Lime shows a substitution notice. Choose an available family, or save the project, install the missing font, restart Lime, and reopen the project before editing again. Lime caches its font list for the current app session. Keep a `.lime` project before using **Layer → Rasterize Text Layer**; rasterizing removes the editable source to enable pixel tools.

For lettering on a curve, first remove hard line breaks from the text; Text Along Path accepts only one line. Select a visible path in **Paths** and use **Layer → Text Along Path…**. It creates a raster result and hides the original editable text. It is not a live text/path link.

## Related guides {#related}

- [Draw and use paths](../../guides/paths/)
- [Color adjustments and effects](../../reference/effects/)
- [Save and export](../../reference/files/)
