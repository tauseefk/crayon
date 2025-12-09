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
   │     ├─ brush_controller.rs  // pointer events -> brush strokes
   │     ├─ camera_controller.rs // pointer events -> zoom/pan
   │     │
   │     ├─ resources            // all resources (except rendering)
   │     ├─ systems              // all systems (including rendering)
   │     │
   │     ├─ renderer             // contexts related to rendering
   │     │  ├─ shaders
   │     │  ├─ pipeline.rs       // Render pipeline setup
   │     │  └─ ui                // UI widgets
   │     │
   │     └─ utils                // math helpers
   │
   └─ batteries                  // utilities crate
      └─ src 
         ├─ batteries.rs         // primitives & curve interpolation
         └─ point_processor.rs   // point data processing
```
