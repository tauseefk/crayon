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
