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

```rust
use std::sync::{Arc, RwLock};
use std::any::{Any, TypeId};
use std::collections::HashMap;

pub struct App {
    resources: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    systems: Vec<Box<dyn System>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            systems: Vec::new(),
        }
    }

    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, resource: T) {
        self.resources.insert(
            TypeId::of::<T>(),
            Arc::new(RwLock::new(resource))
        );
    }

    // Get a resource (read-only)
    pub fn resource<T: Send + Sync + 'static>(&self) -> Arc<RwLock<T>> {
        self.resources
            .get(&TypeId::of::<T>())
            .expect("Resource not found")
            .clone()
            .downcast::<RwLock<T>>()
            .unwrap()
    }

    pub fn add_system(&mut self, system: impl System + 'static) {
        self.systems.push(Box::new(system));
    }

    pub fn run(&self) {
        for system in &self.systems {
            system.run(self);
        }
    }
}

pub trait System: Send + Sync {
    fn run(&self, app: &App);
}

// Example usage:
struct AppState {
    counter: i32,
}

struct MySystem;

impl System for MySystem {
    fn run(&self, app: &App) {
        let state = app.resource::<AppState>();
        let mut state = state.write().unwrap();
        state.counter += 1;
        println!("Counter: {}", state.counter);
    }
}

fn main() {
    let mut app = App::new();
    
    app.insert_resource(AppState { counter: 0 });
    
    app.add_system(MySystem);
    
    app.run();
}
```
