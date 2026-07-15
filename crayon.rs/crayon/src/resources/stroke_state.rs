use crate::document::{ArtboardId, LayerId};
use crate::resource::Resource;

/// Tracks brush stroke boundaries so the paint systems know when to reset the stroke
/// layer (start) and when to merge it into the canvas (end).
#[derive(Default)]
pub struct StrokeState {
    active: bool,
    needs_clear: bool,
    needs_merge: bool,
    /// The layer the stroke accumulates into and merges into on end. Set on
    /// `StrokeStart`; a selection change mid-stroke cannot retarget it.
    pub target: Option<(ArtboardId, LayerId)>,
}

impl StrokeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks the beginning of a stroke targeting `target`; the next
    /// accumulate pass clears the stroke layer.
    pub fn start(&mut self, target: (ArtboardId, LayerId)) {
        self.active = true;
        self.needs_clear = true;
        self.target = Some(target);
    }

    /// Marks the end of a stroke; the stroke layer is merged into the canvas next frame.
    pub fn end(&mut self) {
        if self.active {
            self.needs_merge = true;
        }
    }

    /// The stroke target while a stroke is being drawn — drives the live
    /// stroke quad in the scene pass.
    pub fn active_target(&self) -> Option<(ArtboardId, LayerId)> {
        if self.active { self.target } else { None }
    }

    /// Consumes the pending-clear flag.
    pub fn take_needs_clear(&mut self) -> bool {
        std::mem::take(&mut self.needs_clear)
    }

    /// Consumes the pending-merge flag, ending the active stroke.
    pub fn take_needs_merge(&mut self) -> bool {
        let merge = std::mem::take(&mut self.needs_merge);
        if merge {
            self.active = false;
        }
        merge
    }
}

impl Resource for StrokeState {}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: (ArtboardId, LayerId) = (ArtboardId(1), LayerId(2));

    #[test]
    fn stroke_lifecycle_carries_target() {
        let mut stroke = StrokeState::new();
        assert_eq!(stroke.active_target(), None);

        stroke.start(TARGET);
        assert_eq!(stroke.active_target(), Some(TARGET));
        assert!(stroke.take_needs_clear());
        assert!(!stroke.take_needs_clear(), "clear consumed once");

        stroke.end();
        assert!(stroke.take_needs_merge());
        assert_eq!(stroke.active_target(), None, "merge ends the stroke");
        assert_eq!(stroke.target, Some(TARGET), "target stays for the merge");
    }

    #[test]
    fn end_without_start_does_not_merge() {
        let mut stroke = StrokeState::new();
        stroke.end();
        assert!(!stroke.take_needs_merge());
    }
}
