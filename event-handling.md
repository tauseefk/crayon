```
                          RAW INPUT
 ┌───────────────────────────────────────────────────────────────┐
 │ winit WindowEvent  (app.rs::window_event)                     │
 │   1. egui gets first refusal ── consumed? → stop              │
 │   2. Esc special-case ── pops SelectionStack / exits app      │
 │      (needs event-loop + &mut selection, so it's outside      │
 │       the dispatcher)                                         │
 │   3. everything else ↓  builds a read-only DispatchEnv        │
 └───────────────────────────────────────────────────────────────┘
                             │  DispatchEnv {
                             │    modifiers, camera, doc,
                             │    &selection, brush_size,
                             │    stroke_active, &sender }
                             ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ InputSystem  (resources/input_system.rs)                     │
 │                                                              │
 │  ModifiersChanged ─► stored on self.modifiers (tracked once, │
 │                      stamped into env, immune to focus-loss) │
 │                                                              │
 │  normalize(WindowEvent) ─► Option<InputAction>               │
 │     CursorMoved  → PointerMove{screen}                       │
 │     MouseInput L → PointerDown / PointerUp                   │
 │     MouseWheel   → Scroll{delta,screen}                      │
 │     Keyboard     → Key{code,pressed}                         │
 │                                                              │
 │  dispatch(action, env):  ═══ THE BUBBLE LOOP ═══             │
 │     for ctx in selection.contexts_inner_to_outer():          │
 │         handler = match ctx { Layer→…, Artboard→…, Global }  │
 │         if handler.handle(ctx, action, env) == Yes: break    │
 └──────────────────────────────────────────────────────────────┘
                             │
       selection stack drives the order (bubble phase):
       [Global, Artboard(a), Layer(a,l)]  ──rev──►  Layer, Artboard, Global
                             │
       ┌─────────────────────┼─────────────────────┐
       ▼ (innermost)         ▼                     ▼ (outermost)
┌──────────────┐     ┌──────────────┐       ┌──────────────┐
│ LayerHandler │     │ArtboardHandlr│       │GlobalHandler │
│ draw stroke  │ No→ │ cmd-drag=move│  No→  │ pan / zoom   │
│ cmd-drag=move│     │ click=select │       │ hit-select   │
│ cmd-R clear  │     │ top layer +  │       │ artboard /   │
│              │     │ StrokeStart  │       │ clear;       │
│              │     │              │       │ stroke-end   │
│              │     │              │       │ safety-net   │
└┬─────────────┘     └───────┬──────┘       └─────────────┬┘
 │ each ctx-typed handler returns Yes/No; the first Yes   │
 │ stops the bubble. Handlers send ControllerEvents only. │
 └───────────────────────────┬────────────────────────────┘
                             ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ EventSender  (event_sender.rs)                               │
 │   desktop: mpsc channel ─► relay thread ─► EventLoopProxy    │
 │   wasm:    EventLoopProxy.send_event  directly               │
 │   ControllerEvent ─(From)─► CustomEvent                      │
 └──────────────────────────────────────────────────────────────┘
                                    ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ App::user_event  (app.rs)  ── THE ONLY WRITER                │
 │   Select*/Clear   → DocumentState.selection.{select_*,pop..} │◄─┐
 │   Move*           → doc.document offsets (pure CPU)          │  │
 │   ClearLayer      → doc.gpu_dirty.push(GpuOp) (PaintSystem)  │  │
 │   Camera*         → State.camera                             │  │
 │   Stroke*/Brush   → StrokeState / BrushPointQueue            │  │
 └──────────────────────────────────────────────────────────────┘  │
            │ mutates the SelectionStack the next event reads ─────┘
            ▼  (feedback loop: selection change reshapes the bubble path)
```
