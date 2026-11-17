//! Background compositing for previews whose blend modes depend on the backdrop.
//! The normal-only stack retains its existing immediate GPU sprite path.
use super::{Workspace, layers::MovePaint, preview_backdrop::PreviewContext};
use crate::document::{self, BlendMode, DocumentContent, LayerId};
use gpui::{Context, Window};
use image::{Rgba, RgbaImage};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    context: PreviewContext,
    sprite: gpui::ImageId,
    offset: (i32, i32),
    size: (u32, u32),
    linked_move: bool,
}

impl Key {
    fn same_session(self, other: Self) -> bool {
        self.context == other.context && self.linked_move == other.linked_move
    }
}

#[derive(Default)]
pub(super) struct CompositePreview {
    published: Option<(Key, MovePaint)>,
    busy: bool,
}

impl CompositePreview {
    fn complete(&mut self, key: Key, current: Option<Key>, paint: MovePaint) -> Option<MovePaint> {
        self.busy = false;
        if current.is_some_and(|current| key.same_session(current)) {
            self.published.replace((key, paint)).map(|(_, old)| old)
        } else {
            Some(paint)
        }
    }
}

impl Workspace {
    fn linked_move_preview(&self, raw: &MovePaint) -> bool {
        self.move_overlay
            .as_ref()
            .is_some_and(|paint| Arc::ptr_eq(&paint.sprite, &raw.sprite))
            && self
                .active_tab()
                .is_some_and(|tab| tab.document.content.movement_layers(tab.active_layer).len() > 1)
    }

    fn composite_key(&self, raw: &MovePaint) -> Option<Key> {
        let tab = self.active_tab()?;
        if !tab.document.content.needs_composite_preview() && !self.linked_move_preview(raw) {
            return None;
        }
        let matches = |overlay: &Option<MovePaint>| {
            overlay
                .as_ref()
                .is_some_and(|paint| Arc::ptr_eq(&paint.sprite, &raw.sprite))
        };
        // Text and Move already have explicit commit handoffs. Keep the visible
        // composite's identity through a tool change while that commit settles.
        if ((matches(&self.text_overlay) && self.text_settled == Some(tab.id))
            || (matches(&self.move_overlay) && self.move_settled.is_some()))
            && let Some((key, _)) = self.composite_preview.published.as_ref()
            && key.context.tab == tab.id
            && key.context.generation == tab.history.generation()
            && key.context.layer == tab.active_layer
            && key.sprite == raw.sprite.id
        {
            return Some(*key);
        }
        let (session, target, above) = if matches(&self.text_overlay) {
            let (target, above) = self
                .text_preview_target()
                .unwrap_or((Some(tab.active_layer), tab.active_layer));
            (self.text_backdrop.epoch, target, above)
        } else {
            let session = if matches(&self.move_overlay) {
                self.move_token
            } else if matches(&self.warp_overlay) {
                self.warp_backdrop.epoch
            } else if matches(&self.scale_overlay) {
                self.scale_backdrop.epoch
            } else if matches(&self.gradient_overlay) {
                self.gradient.as_ref()?.epoch
            } else {
                return None;
            };
            (session, Some(tab.active_layer), tab.active_layer)
        };
        Some(Key {
            context: self.preview_context(session, target, above)?,
            sprite: raw.sprite.id,
            offset: raw.offset,
            size: raw.size,
            linked_move: self.linked_move_preview(raw),
        })
    }

    pub(super) fn move_paint(&self) -> Option<MovePaint> {
        if let Some(paint) = self
            .active_tab()
            .and_then(|tab| tab.pending_canvas())
            .and_then(|pending| pending.paint.clone())
        {
            return Some(paint);
        }
        let raw = self.raw_move_paint()?;
        if self
            .active_tab()
            .is_some_and(|tab| tab.document.content.needs_composite_preview())
            || self.linked_move_preview(&raw)
        {
            self.composite_preview
                .published
                .as_ref()
                .filter(|(key, _)| {
                    self.composite_key(&raw)
                        .is_some_and(|current| key.same_session(current))
                })
                .map(|(_, paint)| paint.clone())
        } else {
            Some(raw)
        }
    }

    // Transfer the visible representation before canceling its tool session.
    // Keeping only an Arc clone would let cancellation evict the still-visible
    // GPU images and force another upload during Apply.
    pub(super) fn take_move_paint(&mut self) -> Option<MovePaint> {
        let visible = self.move_paint()?;
        if self
            .composite_preview
            .published
            .as_ref()
            .is_some_and(|(_, paint)| paint.sprite.id == visible.sprite.id)
        {
            return self
                .composite_preview
                .published
                .take()
                .map(|(_, paint)| paint);
        }
        for slot in [
            &mut self.move_overlay,
            &mut self.warp_overlay,
            &mut self.gradient_overlay,
            &mut self.text_overlay,
            &mut self.scale_overlay,
        ] {
            if slot
                .as_ref()
                .is_some_and(|paint| paint.sprite.id == visible.sprite.id)
            {
                return slot.take();
            }
        }
        None
    }

    pub(super) fn refresh_composite_preview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = self.raw_move_paint().and_then(|raw| {
            let key = self.composite_key(&raw)?;
            Some((key, raw, self.active_tab()?.document.content.clone()))
        });
        let Some((key, raw, content)) = input else {
            if let Some((_, paint)) = self.composite_preview.published.take() {
                super::drop_images(paint.into_images(), window, cx);
            }
            return;
        };
        // Session boundaries include tab/layer/tool/content and cancel/restart epochs.
        if self
            .composite_preview
            .published
            .as_ref()
            .is_some_and(|(old, _)| !old.same_session(key))
        {
            let (_, paint) = self.composite_preview.published.take().unwrap();
            super::drop_images(paint.into_images(), window, cx);
        }
        if self.composite_preview.busy
            || self
                .composite_preview
                .published
                .as_ref()
                .is_some_and(|(old, _)| *old == key)
        {
            return;
        }
        self.composite_preview.busy = true;
        let mask_baked = self
            .warp_overlay
            .as_ref()
            .is_some_and(|warp| Arc::ptr_eq(&warp.sprite, &raw.sprite))
            || self
                .scale_overlay
                .as_ref()
                .is_some_and(|scale| Arc::ptr_eq(&scale.sprite, &raw.sprite));
        let job = cx.background_executor().spawn(async move {
            let image = composite(
                &content,
                &raw,
                key.context.target,
                Some(key.context.above),
                1024,
                mask_baked,
                key.linked_move,
            );
            MovePaint {
                below: None,
                above: None,
                sprite: document::preview_render(&image),
                offset: (0, 0),
                size: content.size(),
            }
        });
        cx.spawn_in(window, async move |workspace, cx| {
            let paint = job.await;
            let _ = workspace.update_in(cx, |workspace, window, cx| {
                // Re-read live state: cancellation may happen before the next render.
                // One worker at a time preserves completion order. Accept an interim
                // frame from this session so continuous motion cannot starve display;
                // the next render schedules the newest raw sprite/geometry.
                let current = workspace
                    .raw_move_paint()
                    .and_then(|raw| workspace.composite_key(&raw));
                if let Some(discarded) = workspace.composite_preview.complete(key, current, paint) {
                    super::drop_images(discarded.into_images(), window, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }
}

fn composite(
    content: &DocumentContent,
    paint: &MovePaint,
    mut target: Option<LayerId>,
    mut above: Option<LayerId>,
    edge: u32,
    mask_baked: bool,
    linked_move: bool,
) -> RgbaImage {
    // Translate the immutable snapshot, preserving stack order, masks, opacity,
    // and backdrop-dependent blending even across intervening unlinked layers.
    let mut moved;
    let content = if linked_move {
        moved = content.clone();
        if let Some(id) = target {
            moved.move_linked_layers(id, paint.offset);
        }
        target = None;
        above = None;
        &moved
    } else {
        content
    };
    let (width, height) = content.size();
    let scale = |value: u32| {
        ((u64::from(value) * u64::from(edge.min(width.max(height))) / u64::from(width.max(height)))
            as u32)
            .max(1)
    };
    let mut output = RgbaImage::new(scale(width), scale(height));
    let size = paint.sprite.size(0);
    let sprite_size = (u32::from(size.width), u32::from(size.height));
    let bytes = paint.sprite.as_bytes(0).unwrap_or_default();
    let sprite_at = |cx: i64, cy: i64| -> Option<Rgba<u8>> {
        let (x, y) = (
            cx - i64::from(paint.offset.0),
            cy - i64::from(paint.offset.1),
        );
        if x < 0 || y < 0 || x >= i64::from(paint.size.0) || y >= i64::from(paint.size.1) {
            return None;
        }
        let sx =
            ((x as u64 * 2 + 1) * u64::from(sprite_size.0) / (2 * u64::from(paint.size.0))) as u32;
        let sy =
            ((y as u64 * 2 + 1) * u64::from(sprite_size.1) / (2 * u64::from(paint.size.1))) as u32;
        let index = ((u64::from(sy) * u64::from(sprite_size.0) + u64::from(sx)) * 4) as usize;
        bytes
            .get(index..index + 4)
            .map(|pixel| Rgba([pixel[2], pixel[1], pixel[0], pixel[3]]))
    };
    let (ow, oh) = output.dimensions();
    for (x, y, out) in output.enumerate_pixels_mut() {
        let cx = ((u64::from(x) * 2 + 1) * u64::from(width) / (u64::from(ow) * 2))
            .min(u64::from(width - 1)) as i64;
        let cy = ((u64::from(y) * 2 + 1) * u64::from(height) / (u64::from(oh) * 2))
            .min(u64::from(height - 1)) as i64;
        for layer in content.layers() {
            if layer.visible && layer.opacity != 0 {
                if target == Some(layer.id) {
                    if let Some(mut pixel) = sprite_at(cx, cy) {
                        if !mask_baked {
                            let sx = (((cx - i64::from(paint.offset.0)) as u64 * 2 + 1)
                                * u64::from(layer.pixels.width())
                                / (2 * u64::from(paint.size.0)))
                                as u32;
                            let sy = (((cy - i64::from(paint.offset.1)) as u64 * 2 + 1)
                                * u64::from(layer.pixels.height())
                                / (2 * u64::from(paint.size.1)))
                                as u32;
                            pixel = layer.masked(pixel, sx, sy);
                        }
                        *out = document::blend(*out, pixel, 255, layer.blend_mode);
                    }
                } else {
                    let (sx, sy) = (
                        cx - i64::from(layer.offset.0),
                        cy - i64::from(layer.offset.1),
                    );
                    if sx >= 0
                        && sy >= 0
                        && sx < i64::from(layer.pixels.width())
                        && sy < i64::from(layer.pixels.height())
                    {
                        *out = document::blend(
                            *out,
                            layer.masked(
                                *layer.pixels.get_pixel(sx as u32, sy as u32),
                                sx as u32,
                                sy as u32,
                            ),
                            layer.opacity,
                            layer.blend_mode,
                        );
                    }
                }
            }
            if target.is_none()
                && Some(layer.id) == above
                && let Some(pixel) = sprite_at(cx, cy)
            {
                *out = document::blend(*out, pixel, 255, BlendMode::Normal);
            }
        }
    }
    output
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    use crate::commands::Tool;

    fn paint(value: u8) -> MovePaint {
        MovePaint {
            below: None,
            above: None,
            sprite: document::preview_render(&RgbaImage::from_pixel(
                8,
                8,
                Rgba([value, 20, 40, 255]),
            )),
            offset: (0, 0),
            size: (8, 8),
        }
    }

    #[test]
    fn continuous_input_publishes_completed_frames_then_converges_to_latest() {
        let mut state = CompositePreview::default();
        let mut key = Key {
            context: PreviewContext {
                tab: 1,
                generation: 2,
                layer: LayerId(1),
                tool: Tool::Move,
                session: 3,
                target: Some(LayerId(1)),
                above: LayerId(1),
            },
            sprite: paint(0).sprite.id,
            offset: (0, 0),
            size: (8, 8),
            linked_move: true,
        };
        for index in 1..5 {
            let next = Key {
                sprite: paint(index).sprite.id,
                offset: (index as i32, 0),
                ..key
            };
            let finished = paint(index);
            let id = finished.sprite.id;
            state.busy = true;
            // Every job finishes after another pointer/sprite update.
            state.complete(key, Some(next), finished);
            assert!(!state.busy);
            assert_eq!(state.published.as_ref().unwrap().1.sprite.id, id);
            key = next;
        }
        let final_frame = paint(255);
        let final_bytes = final_frame.sprite.as_bytes(0).unwrap().to_vec();
        state.complete(key, Some(key), final_frame);
        let (published, published_paint) = state.published.as_ref().unwrap();
        assert!(*published == key);
        assert_eq!(published_paint.sprite.as_bytes(0).unwrap(), final_bytes);

        // Reject old work even when tab/tool are unchanged after cancel/restart.
        for context in [
            PreviewContext {
                session: 4,
                ..key.context
            },
            PreviewContext {
                tab: 2,
                ..key.context
            },
            PreviewContext {
                generation: 3,
                ..key.context
            },
            PreviewContext {
                layer: LayerId(2),
                ..key.context
            },
            PreviewContext {
                tool: Tool::Rotate,
                ..key.context
            },
            PreviewContext {
                target: None,
                ..key.context
            },
            PreviewContext {
                above: LayerId(2),
                ..key.context
            },
        ] {
            let rejected = state.complete(key, Some(Key { context, ..key }), paint(0));
            assert!(rejected.is_some());
            assert_eq!(
                state
                    .published
                    .as_ref()
                    .unwrap()
                    .1
                    .sprite
                    .as_bytes(0)
                    .unwrap(),
                final_bytes
            );
        }
        assert!(state.complete(key, None, paint(0)).is_some());
    }
}
