//! Safety checks spanning project persistence, recovery, and prepared canvas data.
//! Keep feature-specific regressions beside the code they exercise.
use crate::{
    adjustments::Adjustment,
    document::layer_effect::EffectSpec,
    document::{self, BlendMode, DocumentContent, MaskEdit},
    preview::Dirty,
    selection::Selection,
    transform,
};
use image::{Rgba, RgbaImage};
use std::{io::Cursor, sync::atomic::AtomicBool};

fn scene() -> DocumentContent {
    DocumentContent::single_layer(
        "Art".into(),
        RgbaImage::from_fn(6, 4, |x, y| Rgba([x as u8 * 35, y as u8 * 60, 90, 200])),
    )
}
fn soft_selection() -> Selection {
    Selection::from_coverage(6, 4, |x, y| ((x + y) * 28).min(255) as u8)
}
fn roundtrip(content: &DocumentContent) -> DocumentContent {
    let mut bytes = Vec::new();
    crate::project::write(&mut bytes, content).unwrap();
    crate::project::read(Cursor::new(bytes)).unwrap()
}
#[test]
fn masks_keep_source_pixels_round_trip_and_apply_the_displayed_alpha() {
    let mut content = scene();
    let id = content.top_layer_id();
    let source = content.layer(id).unwrap().pixels.clone();
    content.mask_from_selection(id, &soft_selection());
    let shown = content.composite();
    assert_eq!(content.layer(id).unwrap().pixels, source);
    assert_eq!(shown.get_pixel(2, 1)[3], ((200u16 * 84 + 127) / 255) as u8);
    let mut loaded = roundtrip(&content);
    assert_eq!(loaded.composite(), shown);
    loaded.edit_mask(id, MaskEdit::Toggle);
    assert_eq!(loaded.composite(), source);
    loaded.edit_mask(id, MaskEdit::Toggle);
    loaded.edit_mask(id, MaskEdit::Invert);
    assert_ne!(loaded.composite(), shown);
    loaded.edit_mask(id, MaskEdit::Invert);
    loaded.edit_mask(id, MaskEdit::Apply);
    assert!(loaded.layer(id).unwrap().mask.is_none());
    for (actual, expected) in loaded.composite().pixels().zip(shown.pixels()) {
        assert_eq!(actual[3], expected[3]);
        if actual[3] != 0 {
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn retained_filter_mask_and_blend_persist_together_and_bake_on_pixel_edits() {
    let mut content = scene();
    let id = content.top_layer_id();
    content.mask_from_selection(id, &soft_selection());
    content.change_blend_mode(id, BlendMode::Multiply);
    let original = content.layer(id).unwrap().pixels.clone();
    content.apply_retained_effect(
        id,
        EffectSpec::Blur { radius: 2 },
        &Selection::Unrestricted,
        false,
    );
    let mut loaded = roundtrip(&content);
    assert_eq!(loaded.composite(), content.composite());
    assert_eq!(
        loaded.layer(id).unwrap().effect.as_ref().unwrap().source,
        original
    );
    let shown = loaded.layer(id).unwrap().pixels.clone();
    assert_eq!(loaded.remove_retained_effect(id, true), Dirty::Unpainted);
    assert_eq!(loaded.layer(id).unwrap().pixels, shown);
    assert!(loaded.layer(id).unwrap().effect.is_none());
    content.remove_retained_effect(id, false);
    assert_eq!(content.layer(id).unwrap().pixels, original);
    content.apply_retained_effect(
        id,
        EffectSpec::Adjustment(Adjustment::Invert),
        &Selection::Unrestricted,
        false,
    );
    assert!(content.turn_layer(id, transform::Turn::Right));
    assert!(content.layer(id).unwrap().effect.is_none());
    assert_eq!(
        content.layer(id).unwrap().pixels.dimensions(),
        content
            .layer(id)
            .unwrap()
            .mask
            .as_ref()
            .unwrap()
            .pixels
            .dimensions()
    );
}

#[test]
fn recovery_skips_live_sessions_recovers_crashes_and_never_overwrites_source() {
    let root = tempfile::tempdir().unwrap();
    let (store, docs, errors) = crate::recovery::Store::open(root.path()).unwrap();
    assert!(docs.is_empty() && errors.is_empty());
    let content = scene();
    let alive = AtomicBool::new(true);
    store
        .snapshot(7, "Test image", &content, None, &alive)
        .unwrap();
    let (other, docs, _) = crate::recovery::Store::open(root.path()).unwrap();
    assert!(docs.is_empty());
    other.clean_exit();
    drop(other);
    drop(store);
    let (reopened, docs, errors) = crate::recovery::Store::open(root.path()).unwrap();
    assert!(errors.is_empty());
    assert_eq!(docs.len(), 1);
    assert!(docs[0].document.path.is_none());
    assert_eq!(docs[0].document.name, "Recovered Test image");
    assert_eq!(docs[0].document.content.composite(), content.composite());
    reopened.clean_exit();
    let (empty, docs, _) = crate::recovery::Store::open(root.path()).unwrap();
    assert!(docs.is_empty());
    empty.clean_exit();
}

#[test]
fn checked_document_preparation_cancels_inside_work_and_keeps_tiles_consistent() {
    let mut content = scene();
    let id = content.top_layer_id();
    content.add_layer(id, "Top".into(), Rgba([10, 20, 30, 120]));
    let expected = content.composite();
    let prepared =
        document::Document::from_content_checked("Prepared".into(), content.clone(), &|| Ok(()))
            .unwrap();
    assert_eq!(prepared.canvas.pixels, expected);
    for cache in prepared.canvas.level_caches() {
        for span in cache.spans() {
            assert_eq!(
                cache.slot(span.index).unwrap().as_ref(),
                &image::imageops::crop_imm(
                    &*expected,
                    span.stored.x as u32,
                    span.stored.y as u32,
                    span.stored.width,
                    span.stored.height
                )
                .to_image()
            );
        }
    }
    let checks = std::cell::Cell::new(0);
    let result =
        document::Document::from_content_checked("Canceled".into(), content.clone(), &|| {
            let count = checks.get() + 1;
            checks.set(count);
            if count >= 4 {
                Err(crate::pdf::PdfCanceled.into())
            } else {
                Ok(())
            }
        });
    assert!(result.err().unwrap().is::<crate::pdf::PdfCanceled>());
    assert_eq!(content.composite(), expected);
}

#[test]
fn corrupt_recovery_files_survive_clean_exit_and_canceled_snapshots_do_not_reappear() {
    let root = tempfile::tempdir().unwrap();
    let (store, _, _) = crate::recovery::Store::open(root.path()).unwrap();
    let session = std::fs::read_dir(root.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let damaged = session.join("tab-99.lime");
    std::fs::write(&damaged, b"IMGEDIT\0bad header").unwrap();
    let dead = AtomicBool::new(false);
    store
        .snapshot(0, "Canceled", &scene(), None, &dead)
        .unwrap();
    assert!(!session.join("tab-0.lime").exists());
    drop(store);
    let (reopened, documents, errors) = crate::recovery::Store::open(root.path()).unwrap();
    assert!(documents.is_empty());
    assert!(!errors.is_empty());
    reopened.clean_exit();
    assert!(damaged.exists());
}

#[test]
fn recovered_failed_text_source_and_name_survive_repeated_crashes() {
    let root = tempfile::tempdir().unwrap();
    let content = scene();
    let text = serde_json::json!({"source":"Preserve this draft even when its renderer fails"});
    let (first, _, _) = crate::recovery::Store::open(root.path()).unwrap();
    first
        .snapshot(5, "Original", &content, Some(&text), &AtomicBool::new(true))
        .unwrap();
    drop(first);
    let (second, documents, errors) = crate::recovery::Store::open(root.path()).unwrap();
    assert!(errors.is_empty());
    assert_eq!(documents[0].text.as_ref(), Some(&text));
    drop(second);
    let (third, documents, errors) = crate::recovery::Store::open(root.path()).unwrap();
    assert!(errors.is_empty());
    assert_eq!(documents[0].text.as_ref(), Some(&text));
    assert_eq!(documents[0].document.name, "Recovered Original");
    third.clean_exit();
}

#[test]
fn a_failed_recovery_replacement_keeps_its_previous_embedded_draft() {
    let root = tempfile::tempdir().unwrap();
    let (store, _, _) = crate::recovery::Store::open(root.path()).unwrap();
    let session = std::fs::read_dir(root.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = serde_json::json!({"source":"Last complete recovery draft"});
    store
        .snapshot(2, "Atomic", &scene(), Some(&text), &AtomicBool::new(true))
        .unwrap();
    let original = std::fs::read(session.join("tab-2.lime")).unwrap();
    let quota = std::fs::File::create(session.join("quota-test")).unwrap();
    quota.set_len(1024 * 1024 * 1024).unwrap();
    assert!(
        store
            .snapshot(2, "Replacement", &scene(), None, &AtomicBool::new(true))
            .is_err()
    );
    assert_eq!(std::fs::read(session.join("tab-2.lime")).unwrap(), original);
    drop(quota);
    std::fs::remove_file(session.join("quota-test")).unwrap();
    drop(store);
    let (recovered, documents, errors) = crate::recovery::Store::open(root.path()).unwrap();
    assert!(errors.is_empty());
    assert_eq!(documents[0].text.as_ref(), Some(&text));
    assert_eq!(documents[0].document.name, "Recovered Atomic");
    recovered.clean_exit();
}

#[test]
fn full_canvas_changes_publish_existing_reduced_levels_with_the_new_pixels() {
    let mut content = DocumentContent::single_layer(
        "Fine detail".into(),
        RgbaImage::from_fn(519, 267, |x, y| {
            Rgba([if (x + y) % 2 == 0 { 255 } else { 0 }, 80, 30, 200])
        }),
    );
    let bottom = content.top_layer_id();
    content.add_layer(bottom, "Top".into(), Rgba([20, 100, 220, 90]));
    let mut canvas = document::RenderedCanvas::new(&content);
    for shift in [1, 2] {
        canvas.install_cache(document::build_level(&content, shift));
    }
    assert!(content.move_layer(bottom, 1));
    let patch = document::RenderedCanvas::prepare_patch(
        &canvas.pixels,
        true,
        canvas.level_caches(),
        &content,
        Dirty::Canvas,
    )
    .unwrap();
    canvas.apply(patch);
    assert_eq!(canvas.pixels, content.composite());
    for shift in [0, 1, 2] {
        assert!(
            canvas.has_level(shift),
            "first published frame must keep level {shift}"
        );
        let expected = document::build_level(&content, shift);
        let levels = canvas.level_caches();
        let actual = levels.iter().find(|level| level.shift() == shift).unwrap();
        for span in expected.spans() {
            assert_eq!(actual.slot(span.index), expected.slot(span.index));
        }
    }
}
