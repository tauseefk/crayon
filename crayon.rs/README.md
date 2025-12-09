## Crayon.rs

Implementation of Crayon.

### Current Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   app.rs (`App`)                        │
│─────────────────────────────────────────────────────────│
│ Resource container and system scheduler                 │
│                                                         │
│ Responsibilities                                        │
│ - hold all resources and systems                        │
│ - process window events and user input                  │
│ - run systems (PreUpdate → Update → PostUpdate)         │
│                                                         │
└────────────────┬────────────────────────────────────────┘
                 │
     ┌───────────┴─────────────┐
     │                         │
     ▼                         ▼
┌───────────────────┐   ┌──────────────────────────────────┐
│    Resources      │   │           Systems                │
│───────────────────│   │──────────────────────────────────│
│ Data storage:     │   │ PreUpdate:                       │
│                   │   │ - FrameAcquireSystem             │
│ - WindowResource  │   │                                  │
│ - RenderContext   │   │ Update:                          │
│ - CanvasContext   │   │ - FrameTimeUpdateSystem          │
│ - EguiContext     │   │ - PaintSystem                    │
│ - State           │   │ - CanvasRenderSystem             │
│ - FrameContext    │──▶│ - ToolsSystem                    │
│ - FrameTime       │   │                                  │
│ - InputSystem     │◀──┤ PostUpdate:                      │
│ - BrushPointQueue │   │ - FramePresentSystem             │
│                   │   │                                  │
└───────────────────┘   └──────────────────────────────────┘
```
