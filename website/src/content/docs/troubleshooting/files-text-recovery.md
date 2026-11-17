---
title: "Text, export, and recovery problems"
description: "Handle missing fonts, flattened exports, PDF limits, and interrupted sessions."
translationKey: troubleshooting/files-text-recovery
sourceRevision: "text-styles-1"
translatedRevision: "text-styles-1"
---

## Text looks different or is clipped {#text}

A missing named font causes substitution. Check Lime's notice and choose an installed family. To use the original font, save the project, install the font, restart Lime, and reopen the project before editing again; the app caches its font list for the session. The label above the Text controls identifies their target: **All text**, **Selected text**, or **New typing**. Highlight a phrase to change only its font, size, color, Bold, or Italic. **Mixed** means the selection contains different values; changing one property preserves the others.

Turn on **Fixed box** to enable Width/Height and wrap text within those dimensions. A fixed text box clips content outside it. If overflow is reported, increase Width/Height or turn Fixed box off to remove the limits. The source text remains stored. Native input-method composition is unsupported: compose in another app and paste into Lime.

Escape commits a text draft. Use Undo to reverse typing; do not use Escape expecting to discard it. Pixel tools require rasterization, which removes the editable source. Save a `.lime` copy first.

## Export loses layers or transparency {#export}

**File → Export…** always writes a flattened copy; reopen the `.lime` project to continue editing layers, paths, and text. Export does not mark the project saved. Save the project separately.

Use PNG or WebP for transparency. JPEG has no alpha, and Lime's PDF export flattens the canvas over white. The checkerboard indicates transparent pixels in the editor; it is not a pattern painted into the export.

Unapplied Crop/Gradient/transform drafts must be applied or canceled before saving/exporting. Numeric and text edits finish first. If a text commit reports an error, its draft stays available and Save/Export does not continue; fix the reported problem and retry.

## A PDF cannot import or differs from the source {#pdf}

Encrypted PDFs are unsupported. Import rasterizes the selected pages at the chosen DPI, so imported text is not editable as text. A first-page canvas may clip larger pages; choose **Fit All Pages** for a shared canvas or separate tabs to retain each page's dimensions. Open as Layers does not resize an existing canvas.

For a size/memory rejection, reduce DPI or select fewer pages. PDF export creates one flattened page, not a multipage or searchable-text PDF. DPI changes physical page size without resampling the canvas.

## Recover interrupted work {#recovery}

After an interrupted session, Lime can reopen separate recovery projects and show a recovery notice. Inspect each recovered tab, then use **Save As…** to choose a destination. Recovery does not overwrite original files or change their saved revisions.

Recovery is periodic, so the final seconds may be missing. Text drafts are included where possible; other unapplied previews are not. If Lime reports a recovery failure, keep the recovered tab open while saving a new copy. Recovery does not replace regular project saves.

If automatic recovery cannot open a copy, Lime preserves it and reports its path. Use **File → Open…** to try that copy manually. Lime retries preserved copies at later startups, but reports the same failure for an unchanged copy only once across launches. A changed copy or a different error can show a new warning.

## Related guides {#related}

- [Save and export](../../reference/files/)
- [Import and export PDFs](../../image/pdf/)
- [Add text and effects](../../guides/text-effects/)
