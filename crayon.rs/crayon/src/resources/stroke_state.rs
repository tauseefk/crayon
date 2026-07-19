use crate::{
    document::{ArtboardId, LayerId},
    resource::Resource,
};

pub type StrokeTarget = (ArtboardId, LayerId);

#[derive(Default)]
pub struct StrokeState {
    active: bool,
    needs_clear: bool,
    needs_merge: bool,
    pub target: Option<StrokeTarget>,
}

impl StrokeState {
    pub fn new() -> Self {
        Self::default()
    }

    // The next accumulate pass clears the stroke layer.
    pub fn start(&mut self, target: StrokeTarget) {
        self.active = true;
        self.needs_clear = true;
        self.target = Some(target);
    }

    pub fn active_target(&self) -> Option<StrokeTarget> {
        if self.active { self.target } else { None }
    }

    // The stroke layer is merged into the canvas next frame.
    pub fn end(&mut self) {
        if self.active {
            self.needs_merge = true;
        }
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
