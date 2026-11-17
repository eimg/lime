---
title: "Previews, Apply, Cancel, and undo"
description: "Know when an edit is a draft, how to finish numeric fields, and how text differs from transforms."
translationKey: "basics/previews"
sourceRevision: "1.2.3-tool-feedback-1"
translatedRevision: "1.2.3-tool-feedback-1"
---

## Finish a tool preview {#tool-previews}

Crop, Gradient, Scale Layer, Rotate, Flip, and Perspective can show a draft on the canvas before changing the document.

1. Adjust the preview as many times as needed.
2. Choose **Apply** to commit it, or **Cancel** to discard it. **Return** and **Escape** perform these actions when the canvas has focus. Crop also has **Image → Apply Crop**.
3. Use **Edit → Undo** after applying if you want to reverse the committed edit.

Scale and the warp tools keep the original pixels while you revise their preview. Repeated adjustments before Apply do not repeatedly resample the committed layer. Switching tools, layers, or tabs cancels those transform previews. Apply/Cancel dismiss the transform controls; select the tool again to begin another session.

The status bar identifies a pending preview and reminds you that Enter on the canvas applies it while Escape or changing tools discards it. During a drag, numeric edit, or invalid transform, it shows the step needed before applying instead. An untouched Scale, Rotate, or Perspective session is not marked as a pending edit. Hover over the status for its full text.

After Apply, the visible canvas draft stays in place while the finished edit is prepared, and the status shows that the edit is being applied.

## Numeric fields have their own edit {#fields}

Return inside a numeric field usually finishes that field. It does not apply a Scale/Rotate/Perspective session. Empty input or Escape restores the field's starting value while retaining earlier preview adjustments. Crop's coordinates update its draft immediately.

Finish the field, then use the preview's **Apply**. Save and other commands resolve supported inline edits first, but an unfinished canvas preview still needs Apply or Cancel.

Rectangle Select’s radius **− / +** buttons keep rapid clicks in order. The number shows the latest requested radius while the selection updates.

## Text and strokes {#text-strokes}

Text uses a different contract: **Escape**, **Cmd/Ctrl+Enter**, clicking outside, or changing tools/layers/tabs commits the text draft. Escape does not discard typed text. Undo steps through typing before document history. If a text commit fails, the draft remains available to correct.

Painting commits when you release the pointer. Escape cancels an unfinished stroke. Holding the pointer down for several overlapping dabs still produces one stroke history step.

## Effect dialogs and saved state {#dialogs-history}

In effect dialogs, **Before/After** compares appearance without changing parameters. Apply uses the chosen parameters even while Before is displayed. **Cancel** closes without changing pixels. **Reset** returns built-in defaults and shows After.

After Apply, effect and Text Along Path dialogs show **Applying…** until the finished edit is ready. Their controls are temporarily unavailable.

Undoing back to the saved document state clears its unsaved marker. Exporting an image does not save the editable project. Selection-only edits do not require project Save, while layer changes, masks, paths, and guides do.

## Related guides {#related}

- [Color adjustments and filters](../../reference/effects/)
- [Layer transforms](../../reference/transforms/)
- [Save projects and export images](../../reference/files/)
