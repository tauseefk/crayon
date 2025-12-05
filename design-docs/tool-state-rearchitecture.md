---
title: RFD Template
author: Tauseef
status: WIP
tags: Project, Example Type
dependent:: [[]]
---

## Problem

Crayon currently have a God-object complex. I need a better way to organize the state so it can be accessed by the different systems without app constructors having to thread it through.

## Why & why now

Adding the UI layer with a color picker has bloated the architecture.
So this needs to be prioritized to make adding new tools much more straightforward.

## Proposed Solutions

### static RwLock singleton

A static singleton `RwLock` tool state struct that can be accessed across the app might be the simplest solution.

### Resource sharing a-la-Actix

This is a bit more heavy handed, as it changes how the entire app functions instead of incremental change.

We're going with the heavy handed approach.

I've already tried the Resources and Systems approach on Eeks, and it works well.

Need to figure out how to structure the app's Event handling. Wondering if the event loop proxy needs to itself be a resource that can be polled, or an event handling system that does the work.

#### Approach 1 - EventLoopProxy as a resource

The BrushController can get renamed to BrushSystem.
The System will then map over the eventLoopProxy events, instead of taking each event via the processor interface.

BrushSystem can be just a `Startup` system that connects the event handlers?
if not, then the brush system would need to poll some sort of event queue resource<WindowEvent>.
These events will then allow BrushController to fire events via the `EventSender` resource.
