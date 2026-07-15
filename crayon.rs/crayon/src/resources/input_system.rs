//! The normalize-then-bubble dispatcher (multi-artboard.md §3.2). Raw winit
//! events become `InputAction`s exactly once, then bubble innermost →
//! outermost through the selection stack until a handler claims them.

use cgmath::{EuclideanSpace, Point2};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{ModifiersState, PhysicalKey},
};

use crate::{
    input::{
        artboard_handler::ArtboardContextHandler,
        dispatch::{ContextHandler, DispatchEnv, Handled, InputAction},
        global_handler::GlobalContextHandler,
        layer_handler::LayerContextHandler,
        selection::SelectionCtx,
    },
    resource::Resource,
};

pub struct InputSystem {
    /// Tracked once via `WindowEvent::ModifiersChanged` — replaces the
    /// retired controllers' per-key Super tracking (and is immune to the
    /// stuck-modifier-on-focus-loss bug that approach had).
    modifiers: ModifiersState,
    /// Last cursor position; `MouseInput`/`MouseWheel` carry no position.
    cursor: Point2<f32>,
    layer_handler: LayerContextHandler,
    artboard_handler: ArtboardContextHandler,
    global_handler: GlobalContextHandler,
}

impl InputSystem {
    pub fn new() -> Self {
        Self {
            modifiers: ModifiersState::default(),
            cursor: Point2::origin(),
            layer_handler: LayerContextHandler::new(),
            artboard_handler: ArtboardContextHandler::new(),
            global_handler: GlobalContextHandler::new(),
        }
    }

    pub fn process_event(&mut self, event: &WindowEvent, mut env: DispatchEnv) {
        if let WindowEvent::ModifiersChanged(modifiers) = event {
            self.modifiers = modifiers.state();
            return;
        }

        let Some(action) = self.normalize(event) else {
            return;
        };

        env.modifiers = self.modifiers;
        self.dispatch(&action, &env);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn normalize(&mut self, event: &WindowEvent) -> Option<InputAction> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Point2::new(position.x as f32, position.y as f32);
                Some(InputAction::PointerMove {
                    screen: self.cursor,
                })
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => Some(if *state == ElementState::Pressed {
                InputAction::PointerDown {
                    screen: self.cursor,
                }
            } else {
                InputAction::PointerUp {
                    screen: self.cursor,
                }
            }),
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => *y as f32,
                };
                Some(InputAction::Scroll {
                    delta,
                    screen: self.cursor,
                })
            }
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => match event.physical_key {
                PhysicalKey::Code(code) => Some(InputAction::Key {
                    code,
                    pressed: event.state.is_pressed(),
                }),
                PhysicalKey::Unidentified(_) => None,
            },
            _ => None,
        }
    }

    fn dispatch(&mut self, action: &InputAction, env: &DispatchEnv) {
        for ctx in env.selection.contexts_inner_to_outer() {
            let handler: &mut dyn ContextHandler = match ctx {
                SelectionCtx::Layer(..) => &mut self.layer_handler,
                SelectionCtx::Artboard(..) => &mut self.artboard_handler,
                SelectionCtx::Global => &mut self.global_handler,
            };
            if let Handled::Yes = handler.handle(ctx, action, env) {
                break;
            }
        }
    }
}

impl Resource for InputSystem {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use cgmath::Vector2;
    use winit::keyboard::KeyCode;

    use crate::{
        document::{Artboard, ArtboardId, DOCUMENT_VERSION, Document, Layer, LayerId},
        event_sender::EventSender,
        events::ControllerEvent,
        input::selection::SelectionStack,
        renderer::camera::Camera2D,
        testing::events::drain,
    };

    const ARTBOARD: ArtboardId = ArtboardId(1);
    const LAYER: LayerId = LayerId(2);

    /// One 100x100 artboard at world (0,0) with a single layer.
    fn doc() -> Document {
        Document {
            version: DOCUMENT_VERSION,
            next_id: 3,
            artboards: vec![Artboard {
                id: ARTBOARD,
                name: "a".to_string(),
                position: [0.0, 0.0],
                size: [100.0, 100.0],
                layers: vec![Layer {
                    id: LAYER,
                    name: "l".to_string(),
                    offset: [0.0, 0.0],
                    visible: true,
                    content: None,
                    thumbhash: None,
                }],
            }],
        }
    }

    /// Camera over a 200x200 viewport centered on the artboard center, scale
    /// 1: screen px == world px + (50, 50).
    fn camera() -> Camera2D {
        let mut camera = Camera2D::with_viewport(200.0, 200.0);
        camera.center_on(Point2::new(50.0, 50.0));
        camera
    }

    fn env<'a>(
        doc: &'a Document,
        selection: &'a SelectionStack,
        sender: &'a EventSender,
        super_pressed: bool,
        stroke_active: bool,
    ) -> DispatchEnv<'a> {
        DispatchEnv {
            modifiers: if super_pressed {
                ModifiersState::SUPER
            } else {
                ModifiersState::default()
            },
            camera: camera(),
            doc,
            selection,
            brush_size: 20.0,
            stroke_active,
            sender,
        }
    }

    fn layer_selected(doc: &Document) -> SelectionStack {
        let mut selection = SelectionStack::new();
        selection.select_artboard(doc, ARTBOARD);
        selection
    }

    #[test]
    fn cmd_drag_with_layer_selected_moves_the_layer() {
        let doc = doc();
        let selection = layer_selected(&doc);
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();

        let down = InputAction::PointerDown {
            screen: Point2::new(100.0, 100.0),
        };
        let drag = InputAction::PointerMove {
            screen: Point2::new(110.0, 96.0),
        };
        input.dispatch(&down, &env(&doc, &selection, &sender, true, false));
        input.dispatch(&drag, &env(&doc, &selection, &sender, true, false));

        let events = drain(&receiver);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ControllerEvent::MoveLayer {
                layer: LAYER,
                world_delta: Vector2 { x, y },
            } if (x - 10.0).abs() < 1e-6 && (y + 4.0).abs() < 1e-6
        ));
    }

    #[test]
    fn cmd_drag_with_no_selection_pans_the_camera() {
        let doc = doc();
        let selection = SelectionStack::new();
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();

        let down = InputAction::PointerDown {
            screen: Point2::new(100.0, 100.0),
        };
        let drag = InputAction::PointerMove {
            screen: Point2::new(90.0, 105.0),
        };
        input.dispatch(&down, &env(&doc, &selection, &sender, true, false));
        input.dispatch(&drag, &env(&doc, &selection, &sender, true, false));

        let events = drain(&receiver);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ControllerEvent::CameraMove {
                world_delta: Vector2 { x, y },
            } if (x + 10.0).abs() < 1e-6 && (y - 5.0).abs() < 1e-6
        ));
    }

    #[test]
    fn cmd_scroll_zooms_from_any_selection_depth() {
        let doc = doc();
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();
        let scroll = InputAction::Scroll {
            delta: 1.0,
            screen: Point2::new(100.0, 100.0),
        };

        // bubbles through Layer and Artboard to Global
        for selection in [layer_selected(&doc), SelectionStack::new()] {
            input.dispatch(&scroll, &env(&doc, &selection, &sender, true, false));
            let events = drain(&receiver);
            assert_eq!(events.len(), 1);
            assert!(matches!(events[0], ControllerEvent::CameraZoom { .. }));
        }

        // without cmd, nobody claims it
        let selection = SelectionStack::new();
        input.dispatch(&scroll, &env(&doc, &selection, &sender, false, false));
        assert!(drain(&receiver).is_empty());
    }

    #[test]
    fn click_at_global_selects_hit_artboard_and_clears_on_miss() {
        let doc = doc();
        let selection = SelectionStack::new();
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();

        // screen (100,100) → world (50,50): inside the artboard
        let hit = InputAction::PointerDown {
            screen: Point2::new(100.0, 100.0),
        };
        input.dispatch(&hit, &env(&doc, &selection, &sender, false, false));
        // release ends the interaction, then click empty space
        let up = InputAction::PointerUp {
            screen: Point2::new(100.0, 100.0),
        };
        input.dispatch(&up, &env(&doc, &selection, &sender, false, false));
        let miss = InputAction::PointerDown {
            screen: Point2::new(190.0, 190.0),
        };
        input.dispatch(&miss, &env(&doc, &selection, &sender, false, false));

        let events = drain(&receiver);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], ControllerEvent::SelectArtboard(ARTBOARD)));
        assert!(matches!(events[1], ControllerEvent::ClearSelection));
    }

    #[test]
    fn stroke_lifecycle_in_layer_context() {
        let doc = doc();
        let selection = layer_selected(&doc);
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();

        let down = InputAction::PointerDown {
            screen: Point2::new(100.0, 100.0),
        };
        input.dispatch(&down, &env(&doc, &selection, &sender, false, false));
        let events = drain(&receiver);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ControllerEvent::StrokeStart));

        // Once the stroke is active, moves feed the point processor (it
        // buffers a 4-point Catmull-Rom window, so early moves may emit
        // nothing) and the release flushes the tail before committing.
        for step in 1..=6 {
            #[allow(clippy::cast_precision_loss)]
            let stroke_move = InputAction::PointerMove {
                screen: Point2::new(100.0 + 4.0 * step as f32, 100.0),
            };
            input.dispatch(&stroke_move, &env(&doc, &selection, &sender, false, true));
        }
        let up = InputAction::PointerUp {
            screen: Point2::new(124.0, 100.0),
        };
        input.dispatch(&up, &env(&doc, &selection, &sender, false, true));

        let events = drain(&receiver);
        let (last, dabs) = events.split_last().expect("events were sent");
        assert!(matches!(last, ControllerEvent::StrokeEnd));
        assert!(!dabs.is_empty());
        assert!(
            dabs.iter()
                .all(|event| matches!(event, ControllerEvent::BrushPoint { .. }))
        );
    }

    #[test]
    fn click_outside_own_artboard_bubbles_to_global_hit_test() {
        let mut doc = doc();
        // second artboard to the right of the first
        doc.artboards.push(Artboard {
            id: ArtboardId(10),
            name: "b".to_string(),
            position: [110.0, 0.0],
            size: [50.0, 50.0],
            layers: vec![],
        });
        let selection = layer_selected(&doc);
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();

        // screen (180, 60) → world (130, 10): inside the second artboard
        let down = InputAction::PointerDown {
            screen: Point2::new(180.0, 60.0),
        };
        input.dispatch(&down, &env(&doc, &selection, &sender, false, false));

        let events = drain(&receiver);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ControllerEvent::SelectArtboard(ArtboardId(10))
        ));
    }

    #[test]
    fn artboard_context_click_selects_top_layer_and_starts_stroke() {
        let doc = doc();
        let mut selection = layer_selected(&doc);
        selection.pop(); // [Global, Artboard]
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();

        let down = InputAction::PointerDown {
            screen: Point2::new(100.0, 100.0),
        };
        input.dispatch(&down, &env(&doc, &selection, &sender, false, false));

        let events = drain(&receiver);
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            ControllerEvent::SelectLayer(ARTBOARD, LAYER)
        ));
        assert!(matches!(events[1], ControllerEvent::StrokeStart));
    }

    #[test]
    fn cmd_drag_with_artboard_selected_moves_the_artboard() {
        let doc = doc();
        let mut selection = layer_selected(&doc);
        selection.pop(); // [Global, Artboard]
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();

        let down = InputAction::PointerDown {
            screen: Point2::new(100.0, 100.0),
        };
        let drag = InputAction::PointerMove {
            screen: Point2::new(107.0, 103.0),
        };
        input.dispatch(&down, &env(&doc, &selection, &sender, true, false));
        input.dispatch(&drag, &env(&doc, &selection, &sender, true, false));

        let events = drain(&receiver);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ControllerEvent::MoveArtboard {
                artboard: ARTBOARD,
                world_delta: Vector2 { x, y },
            } if (x - 7.0).abs() < 1e-6 && (y - 3.0).abs() < 1e-6
        ));
    }

    #[test]
    fn cmd_r_clears_the_selected_layer_only() {
        let doc = doc();
        let (sender, receiver) = EventSender::capturing();
        let mut input = InputSystem::new();
        let key_r = InputAction::Key {
            code: KeyCode::KeyR,
            pressed: true,
        };

        let selection = layer_selected(&doc);
        input.dispatch(&key_r, &env(&doc, &selection, &sender, true, false));
        let events = drain(&receiver);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ControllerEvent::ClearLayer { layer: LAYER }
        ));

        // no layer selected → global clear-canvas is retired, nothing fires
        let selection = SelectionStack::new();
        input.dispatch(&key_r, &env(&doc, &selection, &sender, true, false));
        assert!(drain(&receiver).is_empty());
    }
}
