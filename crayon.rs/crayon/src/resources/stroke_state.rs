use crate::resource::Resource;

/// Tracks brush stroke boundaries so the paint systems know when to reset the stroke
/// layer (start) and when to merge it into the canvas (end).
#[derive(Default)]
pub struct StrokeState {
    active: bool,
    needs_clear: bool,
    needs_merge: bool,
}

impl StrokeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks the beginning of a stroke; the next accumulate pass clears the stroke layer.
    pub fn start(&mut self) {
        self.active = true;
        self.needs_clear = true;
    }

    /// Marks the end of a stroke; the stroke layer is merged into the canvas next frame.
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
