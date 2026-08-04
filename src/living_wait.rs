//! One-frame arbitration for Poolrooms' animated waiting raft.

use dwemer_poolrooms::{chrome, water::Surface};
use egui::{Context, Rect, Stroke, StrokeKind, Vec2};

const BOUNCER_WIDTH: f32 = 250.0;
const BOUNCER_HEIGHT: f32 = 150.0;

/// The single living waiting region admitted by one Poolrooms surface.
///
/// Product widgets claim their visible rectangle while drawing. Composition
/// consumes the largest claim, which lets a prominent waiting card supersede a
/// concurrent status label without coupling either caller to draw order.
#[derive(Debug, Default)]
pub struct LivingWait {
    claim: Option<Rect>,
}

impl LivingWait {
    /// Paint and claim the standard central loading bouncer.
    ///
    /// The card is ordinary Poolrooms chrome; [`Self::compose`] couples its
    /// rectangle to the animated waiting raft later in the same frame.
    pub fn bouncer(&mut self, ui: &mut egui::Ui, arena: Rect) -> Rect {
        let rect = bouncer_rect(arena);
        self.claim(rect);
        let painter = ui.painter();
        let _fill = painter.rect_filled(rect, 2.0, chrome::SURFACE);
        let _stroke = painter.rect_stroke(
            rect,
            2.0,
            Stroke::new(1.0_f32, chrome::EDGE_STRONG),
            StrokeKind::Inside,
        );
        let font = egui::FontId::new(37.0, egui::FontFamily::Proportional);
        let galley = painter.layout_no_wrap("LOADING".to_owned(), font.clone(), chrome::HOT);
        painter.galley(rect.center() - galley.size() * 0.5, galley, chrome::HOT);
        rect
    }

    /// Mark a visible widget or panel as waiting during the current frame.
    pub fn claim(&mut self, rect: Rect) {
        if !rect.is_positive() {
            return;
        }
        if self
            .claim
            .is_none_or(|incumbent| area(rect) > area(incumbent))
        {
            self.claim = Some(rect);
        }
    }

    /// Reconcile this frame's claims with the physical loading raft.
    ///
    /// Consuming the claim makes disappearance automatic: a later frame with
    /// no waiting widget always settles the raft.
    pub fn compose(&mut self, ctx: &Context, water: &mut Surface) {
        match self.claim.take() {
            Some(rect) => water.show_loading(ctx, rect),
            None => water.hide_loading(),
        }
    }
}

fn bouncer_rect(arena: Rect) -> Rect {
    let size = Vec2::new(
        BOUNCER_WIDTH.min((arena.width() - 24.0).max(120.0)),
        BOUNCER_HEIGHT.min((arena.height() - 24.0).max(96.0)),
    );
    Rect::from_center_size(arena.center(), size)
}

fn area(rect: Rect) -> f32 {
    rect.width() * rect.height()
}

#[cfg(test)]
mod tests {
    use egui::{Rect, pos2};

    use super::*;

    #[test]
    fn largest_waiting_surface_wins_without_draw_order_authority() {
        let small = Rect::from_min_max(pos2(2.0, 3.0), pos2(12.0, 8.0));
        let large = Rect::from_min_max(pos2(20.0, 30.0), pos2(60.0, 70.0));

        let mut forward = LivingWait::default();
        forward.claim(small);
        forward.claim(large);

        let mut backward = LivingWait::default();
        backward.claim(large);
        backward.claim(small);

        assert_eq!(forward.claim, Some(large));
        assert_eq!(backward.claim, Some(large));
    }

    #[test]
    fn dead_rectangles_cannot_arm_animation() {
        let mut wait = LivingWait::default();
        wait.claim(Rect::ZERO);
        wait.claim(Rect::NOTHING);
        assert_eq!(wait.claim, None);
    }

    #[test]
    fn frame_claim_is_consumed_exactly_once() {
        let rect = Rect::from_min_max(pos2(2.0, 3.0), pos2(12.0, 8.0));
        let mut wait = LivingWait::default();
        wait.claim(rect);
        assert_eq!(wait.claim.take(), Some(rect));
        assert_eq!(wait.claim.take(), None);
    }

    #[test]
    fn bouncer_keeps_its_canonical_size_until_the_arena_contracts() {
        let large = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(900.0, 700.0));
        assert_eq!(bouncer_rect(large).size(), egui::vec2(250.0, 150.0));

        let narrow = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(180.0, 110.0));
        assert_eq!(bouncer_rect(narrow).size(), egui::vec2(156.0, 96.0));
    }
}
