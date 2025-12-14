---
title: Brush Size and Preview
author: Tauseef
status: WIP
tags: Project
---

## Problem

Crayon currently has no way to update brush size, which is quite sad actually.

## Why & why now

Brush size is an essential tool for drawing/painting, imagine having to use a large straight edge to paint the starry night, wouldn't be so starry.

## Proposed Solutions

### Common Changes

#### Passing brush size to BrushController

As brush size also has an impact on the curve interpolation, I've updated the `InputSystem` to thread through the brush size to the BrushController. I don't necessarily like this approach, but I'll come back to this later.

#### Brush Preview

To demostrate what the updated brush looks like I want to add a brush preview.
This would use the debug context from `egui` to draw the preview on top of everything else.
It would be similar to the `fps` system, and will display the preview circle when the user interacts with the tools and hide after some delay.
Creating this as a separate widget also allows us to show this while the user is creating brush strokes. I've enjoyed that UX on `AWE`.

Most important part is the preview state, which handles the show/hide state, and also the debounced hiding.

```rust
pub struct BrushPreviewState {
    pub visible: bool,
    last_interaction: Option<Instant>,
    timeout_duration: Duration,
}

impl BrushPreviewState {
    /// Should run on user interaction
    pub fn mark_interaction(&mut self) {...}

    /// Should run every frame to toggle preview
    /// The `debounce` is accomplished by measuring the time delta between current time and last_interaction
    pub fn update(&mut self) {}

    pub fn is_visible(&self) -> bool {...}
}
```

Other widgets that require the preview would just update the preview state resource by marking the interaction.

```rust
impl System for BrushPreviewUpdateSystem {
    // This would run every frame and call update on the `BrushPreviewState`.
    fn run(&self, app: &App) {
        if let Some(mut preview_state) = app.write::<BrushPreviewState>() {
            preview_state.update();
        }
    }
}
```

`BrushPreviewWidget` takes care of actually rendering the preview. It ensures that the camera zoom is applied to the preview so it correctly reflects the brush stroke thickness.

### Option 1 - Separate `BrushSizeWidget`

This requires the creation of a separate widget, that displays the slider, and updating the `ControllerEvent::BrushColor` to instead take a `BrushProperties` struct.

```rust
struct BrushProperties {
  color: BrushColor,
  size: f32
}
```

From the widgets `BrushSizeWidget` and `BrushColorWidget`, the event fired would contain the updated values.

As I'm creating a separate `BrushPreviewWidget`, it's better to just create them separately.

### Option 2 - Add to `BrushColorWidget` and rename to `BrushUpdateWidget`

This is straightforward, would require adding the code to draw a slider in the the existing widget. However, in the future this would be difficult to extend.
