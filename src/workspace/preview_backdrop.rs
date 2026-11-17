//! Session-scoped ownership of unchanged stack partitions during preview startup.
use super::{Workspace, layers::MovePaint};
use crate::{
    commands::Tool,
    document::{self, DocumentContent, LayerId},
};
use gpui::{Context, RenderImage, Window};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PreviewContext {
    pub tab: usize,
    pub generation: u64,
    pub layer: LayerId,
    pub tool: Tool,
    pub session: u64,
    pub target: Option<LayerId>,
    pub above: LayerId,
}

impl Workspace {
    pub(super) fn preview_context(
        &self,
        session: u64,
        target: Option<LayerId>,
        above: LayerId,
    ) -> Option<PreviewContext> {
        let tab = self.active_tab()?;
        Some(PreviewContext {
            tab: tab.id,
            generation: tab.history.generation(),
            layer: tab.active_layer,
            tool: self.preferences.tool,
            session,
            target,
            above,
        })
    }
}

pub(super) struct Backdrop {
    pub below: Option<Arc<RenderImage>>,
    pub above: Option<Arc<RenderImage>>,
}

impl Backdrop {
    pub fn prepare(content: &DocumentContent, target: Option<LayerId>, above: LayerId) -> Self {
        let (below, above) = match target {
            Some(layer) => content.preview_around(layer),
            None => content.preview_at_insertion(above),
        };
        Self {
            below: below.as_ref().map(document::preview_render),
            above: above.as_ref().map(document::preview_render),
        }
    }

    pub fn take(paint: &mut MovePaint) -> Self {
        Self {
            below: paint.below.take(),
            above: paint.above.take(),
        }
    }

    pub fn into_images(self) -> impl Iterator<Item = Arc<RenderImage>> {
        self.below.into_iter().chain(self.above)
    }
}

#[derive(Default)]
pub(super) struct BackdropCache {
    pub epoch: u64,
    waiting: Option<(PreviewContext, Backdrop)>,
}

impl BackdropCache {
    pub fn reset(&mut self, window: &mut Window, cx: &mut Context<Workspace>) {
        self.epoch = self.epoch.wrapping_add(1);
        self.clear(window, cx);
    }

    pub fn clear(&mut self, window: &mut Window, cx: &mut Context<Workspace>) {
        if let Some((_, backdrop)) = self.waiting.take() {
            super::drop_images(backdrop.into_images(), window, cx);
        }
    }

    pub fn take(
        &mut self,
        context: PreviewContext,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Option<Backdrop> {
        match self.take_matching(context)? {
            Ok(backdrop) => Some(backdrop),
            Err(backdrop) => {
                super::drop_images(backdrop.into_images(), window, cx);
                None
            }
        }
    }

    fn take_matching(&mut self, context: PreviewContext) -> Option<Result<Backdrop, Backdrop>> {
        let (stored, backdrop) = self.waiting.take()?;
        Some(if stored == context {
            Ok(backdrop)
        } else {
            Err(backdrop)
        })
    }

    // Return rejected/replaced ownership to the caller for GPU-cache cleanup.
    // An empty pair is still a prepared backdrop and must survive supersession.
    pub fn retain(
        &mut self,
        request: PreviewContext,
        current: Option<PreviewContext>,
        has_overlay: bool,
        backdrop: Backdrop,
    ) -> Option<Backdrop> {
        if current == Some(request) && !has_overlay {
            self.waiting
                .replace((request, backdrop))
                .map(|(_, old)| old)
        } else {
            Some(backdrop)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    pub(super) fn context(tool: Tool) -> PreviewContext {
        PreviewContext {
            tab: 1,
            generation: 4,
            layer: LayerId(2),
            tool,
            session: 7,
            target: Some(LayerId(2)),
            above: LayerId(2),
        }
    }

    fn image() -> Arc<RenderImage> {
        document::preview_render(&RgbaImage::from_pixel(8, 8, Rgba([17, 90, 200, 128])))
    }

    #[test]
    fn superseded_text_and_transform_jobs_transfer_backgrounds_without_rebuilding() {
        for tool in [
            Tool::Text,
            Tool::ScaleLayer,
            Tool::Rotate,
            Tool::Perspective,
        ] {
            let context = context(tool);
            let mut cache = BackdropCache::default();
            let mut paint = MovePaint {
                below: Some(image()),
                above: Some(image()),
                sprite: image(),
                offset: (0, 0),
                size: (8, 8),
            };
            let ids = (
                paint.below.as_ref().unwrap().id,
                paint.above.as_ref().unwrap().id,
            );
            let obsolete_sprite = paint.sprite.id;
            for _ in 0..3 {
                let backdrop = Backdrop::take(&mut paint);
                assert!(paint.below.is_none() && paint.above.is_none());
                assert!(
                    cache
                        .retain(context, Some(context), false, backdrop)
                        .is_none()
                );
                let reused = cache.take_matching(context).unwrap().ok().unwrap();
                paint.below = reused.below;
                paint.above = reused.above;
                paint.sprite = image();
                assert_eq!(
                    (
                        paint.below.as_ref().unwrap().id,
                        paint.above.as_ref().unwrap().id
                    ),
                    ids
                );
                assert_ne!(paint.sprite.id, obsolete_sprite);
                assert!(cache.waiting.is_none());
            }
            // Empty partitions also count as prepared (single-layer scenes).
            assert!(
                cache
                    .retain(
                        context,
                        Some(context),
                        false,
                        Backdrop {
                            below: None,
                            above: None
                        }
                    )
                    .is_none()
            );
            assert!(cache.take_matching(context).unwrap().is_ok());
        }
    }

    #[test]
    fn background_reuse_rejects_stale_contexts_at_completion_and_consumption() {
        let original = context(Tool::Text);
        for current in [
            None,
            Some(PreviewContext { tab: 2, ..original }),
            Some(PreviewContext {
                generation: 5,
                ..original
            }),
            Some(PreviewContext {
                layer: LayerId(3),
                ..original
            }),
            Some(PreviewContext {
                tool: Tool::Rotate,
                ..original
            }),
            Some(PreviewContext {
                session: 8,
                ..original
            }),
            Some(PreviewContext {
                target: None,
                ..original
            }),
            Some(PreviewContext {
                above: LayerId(3),
                ..original
            }),
        ] {
            let mut cache = BackdropCache::default();
            let backdrop = Backdrop {
                below: Some(image()),
                above: None,
            };
            assert!(cache.retain(original, current, false, backdrop).is_some());
            assert!(cache.waiting.is_none());
            if let Some(current) = current {
                cache.retain(
                    original,
                    Some(original),
                    false,
                    Backdrop {
                        below: Some(image()),
                        above: None,
                    },
                );
                assert!(cache.take_matching(current).unwrap().is_err());
                assert!(cache.waiting.is_none());
            }
        }
        let mut cache = BackdropCache::default();
        assert!(
            cache
                .retain(
                    original,
                    Some(original),
                    true,
                    Backdrop {
                        below: Some(image()),
                        above: None
                    }
                )
                .is_some()
        );
        assert!(cache.waiting.is_none());
    }

    #[test]
    fn prepared_backgrounds_match_existing_replacement_and_insertion_partitions() {
        let mut content = DocumentContent::single_layer(
            "Below".into(),
            RgbaImage::from_fn(16, 16, |x, y| Rgba([x as u8 * 10, y as u8 * 10, 90, 190])),
        );
        let layer = content
            .add_layer(
                content.top_layer_id(),
                "Target".into(),
                Rgba([20, 40, 60, 90]),
            )
            .unwrap();
        content
            .add_layer(layer, "Above".into(), Rgba([90, 50, 20, 120]))
            .unwrap();
        for target in [Some(layer), None] {
            let prepared = Backdrop::prepare(&content, target, layer);
            let (below, above) = match target {
                Some(layer) => content.preview_around(layer),
                None => content.preview_at_insertion(layer),
            };
            for (actual, expected) in [(prepared.below, below), (prepared.above, above)] {
                let actual = actual.unwrap();
                let expected = document::preview_render(&expected.unwrap());
                assert_eq!(actual.size(0), expected.size(0));
                assert_eq!(actual.as_bytes(0), expected.as_bytes(0));
            }
        }
    }
}
