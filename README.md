## Crayon

Crayon is a cross-platform drawing app built with `wgpu`.

### Directory Structure

```
crayon 
│
├─ www                           // web frontend
│
└─ crayon.rs                     // Rust workspace
   ├─ build.sh                   // compile to WASM
   │
   ├─ crayon                     // main app crate
   │  └─ src 
   │     ├─ lib.rs               // WASM entrypoint
   │     ├─ main.rs              // native entrypoint 
   │     ├─ app.rs               // window initialization & event handling
   │     ├─ renderer 
   │     │  ├─ shaders
   │     │  ├─ pipeline.rs       // Render pipeline setup
   │     │  └─ state.rs          // GPU state management
   │     ├─ brush_controller.rs
   │     ├─ camera_controller.rs
   │     └─ utils                // math helpers
   │
   └─ batteries                  // utilities crate
      └─ src 
         ├─ batteries.rs         // primitives & curve interpolation
         └─ point_processor.rs   // point data processing
```

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
└──────────────────────┬─────────────────────────────┘
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
└──────────────────────┬────────────────────────────┘
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
└───────────────────────────────────────────────────┘
```
