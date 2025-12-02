## Crayon.rs

Implementation of Crayon.

### Current Architecture

```
┌────────────────────────────────────────────────────┐
│                 app.rs (`App`)                     │
│────────────────────────────────────────────────────│
│ Holds the `State` and tool controllers             │
│                                                    │
│ Responsibilities                                   │
│ - process window events and user input             │
│ - coordinate controllers & forward events to State │
└────────────────────────┬───────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────┐
│               state.rs (`State`)                  │
│───────────────────────────────────────────────────│
│ Holds the `RendererState` & `Camera2D`            │
│                                                   │
│ Responsibilities                                  │
│ - manage camera state and transformations         │
│ - forward rendering commands to `RendererState`   │
└────────────────────────┬──────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────┐
│        renderer/state.rs (`RendererState`)        │
│───────────────────────────────────────────────────│
│ Holds GPU resources (device, queue, surface, etc) │
│                                                   │
│ Responsibilities                                  │
│ - manage GPU resources and ping-pong textures     │
│ - execute render passes                           │
└────────────────────────┬──────────────────────────┘
                         │
             ┌───────────┴─────────────┐
             ▼                         ▼
    ┌──────────────────┐      ┌──────────────────┐
    │ Canvas Renderer  │      │   UI Renderer    │
    │                  │      │                  │
    │──────────────────│      │──────────────────│
    │    Renders       │      │ Renders egui     │
    │    drawing       │      │ interface        │
    │    operations    │      │                  │
    └──────────────────┘      └──────────────────┘
```
