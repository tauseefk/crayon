# Heterogeneous multi-selection — representation & set-aware dispatch

## 1. Context & goal

Selection today is a single linear path (`SelectionStack` = `[Global, Artboard, Layer]`,
`crayon/src/input/selection.rs`), dispatched DOM-style innermost→outermost. Its accessors
(`selected_layer`, `selected_artboard`) return `Option`, so at most one artboard and one
layer are ever selected.

**Goal (this doc):** express a *heterogeneous multi-selection* — several artboards **and**
several layers held simultaneously — and make **event dispatch respect it** (set-wide
actions fan out over every member). 

**Deliberately out of scope** (deferred to later work): the acquisition UX that lets a user
*build* such a selection (Shift-click, panel multi-highlight, marquee), the Escape
multi-select semantics, and the *execution* of delete/copy/paste (which require
document-mutation and GPU create/destroy infrastructure absent at HEAD/S3). This work is the
data model plus the dispatch mechanism, **validated entirely by tests** that construct
selections directly through the model API.

### Guiding design fact

Bubbling is a *single-target* mechanism: an event has one target and propagates along one
ancestor chain. Multi-select does **not** change the event target — it changes what
*commands operate on*. So the model must separate two concerns the current code conflates:

- **Focus (primary)** — the last-touched item. Anchors the bubble path (pointer / draw /
  tool / zoom / pan) and the stroke target. This *is* today's single path.
- **Set** — every explicitly selected item. What set-wide commands fan out over.

The focus is representable by a sum type; the set sits on top of it.

## 2. Model (`crayon/src/input/selection.rs`)

Rename the type `SelectionStack → Selection`. Keep `SelectionCtx` as the focus-path frame
type the dispatcher matches on; add its non-`Global` subset as the set-member type.

```rust
/// Existing — the focus-path frame type; unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SelectionCtx { Global, Artboard(ArtboardId), Layer(ArtboardId, LayerId) }

/// New — a selectable entity. Heterogeneous set members; no `Global`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Selectable { Artboard(ArtboardId), Layer(ArtboardId, LayerId) }

/// Replaces `SelectionStack`. Lives in `DocumentState.selection`.
pub struct Selection {
    /// Insertion-ordered, deduplicated selected items (heterogeneous).
    items: Vec<Selectable>,
    /// The primary/active item — bubble anchor, stroke target, refocus pivot.
    /// Held **by value, not by index**: no out-of-bounds class, and immune to
    /// index-invalidation when `items` shifts on removal. `None` ⇔ empty ⇔
    /// `Global` focus.
    focus: Option<Selectable>,
}
```

Conversions: `Selectable → SelectionCtx` is total (`Artboard→Artboard`, `Layer→Layer`);
`focus_ctx()` maps `Some(Selectable) → SelectionCtx`, `None → Global`.

**Invariant surface.** The *focus path* is derived from a single `Selectable`, so all the
old malformed-path states (`stack[0] != Global`, missing/mismatched ancestor) are
unrepresentable by construction — the sum-type "illegal states unrepresentable" win is
retained. The set adds exactly two invariants Rust can't encode purely structurally, both
enforced by the private fields + mutator-only construction below:

1. `items` is deduplicated (a `Vec` could hold `[A, A]`).
2. `focus.map_or(true, |f| items.contains(&f))` — focus, when present, is a member.

By-value focus reduces the worst-case violation to a benign stale value (never a panic).
Optionally back `items` with an insertion-ordered set (`indexmap::IndexSet`) to make (1)
structural too, leaving (2) as the single runtime invariant.

### 2.1 Preserved API — single-select parity

These keep `app.rs`, `document_state.rs`, `input_system.rs`, and the existing test suite
compiling and green. Behavior for a size-1 selection is identical to today.

| Method | Semantics (unchanged) |
|---|---|
| `new() -> Self` | empty set, `focus = None` (Global). `Default` delegates. |
| `select_artboard(&mut self, doc, id)` | **replace**: set = `{Layer(id, topmost)}` (auto-focus top layer) or `{Artboard(id)}` if no layers; `clear()` if `id` absent. Focus = that item. |
| `select_layer(&mut self, a, l)` | replace: set = `{Layer(a, l)}`, focus = it. |
| `clear(&mut self)` | set = `{}`, `focus = None`. |
| `pop(&mut self) -> bool` | **focus drill-out**: `Layer(a,_) → items=[Artboard(a)], focus=Artboard`; `Artboard(_) → items=[], focus=None`; `None → false`. Preserves current Esc and the `input_system` test that pops to reach artboard context. |
| `selected_layer(&self) -> Option<(ArtboardId, LayerId)>` | the **focus** layer (stroke target); `None` if focus isn't a layer. |
| `selected_artboard(&self) -> Option<ArtboardId>` | focus's ancestor artboard (panel highlight). |
| `contexts_inner_to_outer(&self) -> impl Iterator<Item = SelectionCtx>` | **derived from the focus**: `Layer→[L,A,G]`, `Artboard→[A,G]`, `None→[G]`. Same sequence the `Vec` produced ⇒ **bubbling unchanged**. Zero-alloc (fixed `[SelectionCtx;3]` truncated by count). |
| `on_layer_deleted(&mut self, id)` | remove `Layer(_,id)` from the set; if it was the focus, refocus to its parent `Artboard`. |
| `on_artboard_deleted(&mut self, id)` | remove `Artboard(id)` **and every `Layer(id,_)`** from the set; refocus/clear as needed. |

### 2.2 New API — build & read the set, drive fan-out

Used by tests (to construct heterogeneous selections) and by the dispatch layer (to fan out).

| Method | Semantics |
|---|---|
| `replace(&mut self, s: Selectable)` | set = `{s}`, focus = `s`. |
| `add(&mut self, s: Selectable)` | insert if absent (dedup); focus = `s`. |
| `toggle(&mut self, s: Selectable)` | remove if present (refocus to last remaining, or `None`), else `add`. |
| `contains(&self, s: Selectable) -> bool` | membership. |
| `is_empty(&self) -> bool` | `items.is_empty()`. |
| `focus(&self) -> Option<Selectable>` | the primary item. |
| `items(&self) -> &[Selectable]` | the full set (insertion order). |
| `selected_artboards(&self) -> impl Iterator<Item = ArtboardId>` | artboard members. |
| `selected_layers(&self) -> impl Iterator<Item = (ArtboardId, LayerId)>` | layer members. |
| `move_targets(&self) -> impl Iterator<Item = Selectable>` | **dedup for moves**: all artboard members, plus only those layer members whose parent artboard is *not* a member (an artboard move already carries its child layers — avoids double-applying). |

`items`/`focus` stay private; the two invariants above (§2, dedup + focus-membership) are
enforced entirely inside these mutators — illegal states are unconstructable from outside.
With by-value focus, `toggle`/`on_*_deleted` remove by find-and-remove and refocus to
`items.last()`, with no index bookkeeping to invalidate.

### 2.3 Wiring the rename

- `resources/document_state.rs`: field `selection: Selection`; `DocumentState::new` still
  calls `select_artboard` for the first artboard at boot.
- `input/dispatch.rs`: `DispatchEnv.selection: &'a Selection`; add
  `pub const NUDGE_STEP: f32` (screen-px arrow step) or place it in the handler.
- `app.rs`: the `SelectArtboard`/`SelectLayer`/`ClearSelection` `user_event` arms call the
  same-named `Selection` methods; `DispatchEnv` construction passes `&doc.selection`. Esc
  keeps calling `pop()`. No behavior change.

## 3. Set-aware dispatch (`resources/input_system.rs`)

Two phases, one selection — conceptually the set is an innermost frame ahead of the focus
path:

```
[ Selection set ] → [ Layer(focus) ] → [ Artboard ] → [ Global ]
   set-wide cmds        pointer / draw / tool / zoom / pan   (all unchanged)
```

```rust
fn dispatch(&mut self, action: &InputAction, env: &DispatchEnv) {
    // Phase 1 — set-wide commands fan out over every member.
    if let Handled::Yes = self.selection_handler.handle(action, env) { return; }
    // Phase 2 — focus-path bubble; spatial/tool semantics untouched.
    for ctx in env.selection.contexts_inner_to_outer() {
        let handler: &mut dyn ContextHandler = match ctx {
            SelectionCtx::Layer(..)    => &mut self.layer_handler,
            SelectionCtx::Artboard(..) => &mut self.artboard_handler,
            SelectionCtx::Global       => &mut self.global_handler,
        };
        if let Handled::Yes = handler.handle(ctx, action, env) { return; }
    }
}
```

Add a `selection_handler: SelectionHandler` field to `InputSystem` and construct it in `new()`.

### 3.1 `SelectionHandler` (`crayon/src/input/selection_handler.rs`)

The concrete proof that "dispatch respects the set." It claims exactly one set-wide command
now — **arrow-key nudge**, chosen because it has zero external dependencies (reuses existing
`MoveLayer`/`MoveArtboard` events; no document mutation, no GPU). Everything else returns
`No`, so the focus-path bubble (draw, cmd-drag, zoom, pan, hit-select) is entirely undisturbed.

```rust
pub struct SelectionHandler;

impl SelectionHandler {
    pub fn handle(&mut self, action: &InputAction, env: &DispatchEnv) -> Handled {
        let InputAction::Key { code, pressed: true } = *action else { return Handled::No };
        let Some(screen_delta) = arrow_delta(code) else { return Handled::No }; // else bubble
        if env.selection.is_empty() { return Handled::No; }

        let world_delta = env.camera.screen_delta_to_world(screen_delta * NUDGE_STEP);
        for target in env.selection.move_targets() {
            match target {
                Selectable::Artboard(a) => env.sender.send(
                    ControllerEvent::MoveArtboard { artboard: a, world_delta }),
                Selectable::Layer(_, l) => env.sender.send(
                    ControllerEvent::MoveLayer { layer: l, world_delta }),
            }
        }
        Handled::Yes
    }
}
```

`arrow_delta` maps `Arrow{Left,Right,Up,Down} → unit screen vector`; any other key → `None`
(so non-arrow keys such as Cmd+R fall through to the focus-path layer handler unchanged).
`move_targets()` supplies the artboard/child-layer dedup (§2.2).

Delete / copy / paste are **not** implemented here; when their infrastructure lands they
attach to this same fan-out (iterate `selected_*`, emit per-item events) — the mechanism is
what this stage establishes and locks down with tests.

## 4. Out of scope (explicitly deferred)

- Acquisition UX: Shift-click `toggle`, plain-click already `replace`s; panel multi-highlight;
  marquee/rubber-band. (`toggle`/`add` exist on the model but are called only from tests here.)
- Escape multi-select semantics (kept as today's focus drill-out via `pop`).
- `Delete`/`Copy`/`Paste`: new events, `Document::remove_*`/`insert`/`duplicate`,
  `GpuOp::{DestroyLayer, CreateLayer}` + `PaintSystem` handling, a `Clipboard` resource.
- No new `ControllerEvent`/`CustomEvent` variants are required for this scope.

## 5. Files

| File | Change |
|---|---|
| `crayon/src/input/selection.rs` | rewrite: `Selection` + `Selectable` (§2) |
| `crayon/src/input/selection_handler.rs` | **new**: set-wide dispatch (§3.1) |
| `crayon/src/input/mod.rs` | add `pub mod selection_handler;` |
| `crayon/src/input/dispatch.rs` | `selection: &Selection`; `NUDGE_STEP` |
| `crayon/src/resources/input_system.rs` | two-phase `dispatch`; `selection_handler` field; tests |
| `crayon/src/resources/document_state.rs` | field type `Selection` |
| `crayon/src/app.rs` | rename references in `SelectArtboard/SelectLayer/ClearSelection` arms + `DispatchEnv` build |

## 6. Tests — the validation

### 6.1 `input/selection.rs` unit tests

- **Port** every existing test (focus path via `contexts_inner_to_outer`, `pop` drill-out,
  `selected_layer/artboard`, `on_*_deleted`) — assertions unchanged; they now exercise the
  size-1 path of the set model.
- **New — heterogeneous set:** build `{Artboard(1), Layer(1,3), Artboard(4)}` via
  `replace` + `add`; assert `selected_artboards()` = `{1,4}`, `selected_layers()` = `{(1,3)}`,
  `focus()` = last added, `contains` correct.
- **New — `toggle`:** add then toggle-off a member; focus falls back to a remaining member;
  toggling the last member → `is_empty()` and focus `None`.
- **New — `move_targets` dedup:** for `{Artboard(1), Layer(1,3)}`, `move_targets()` yields
  only `Artboard(1)` (the child layer is subsumed); for `{Artboard(1), Layer(4,7)}` (layer of
  a *different*, unselected artboard) it yields both.
- **New — focus-path from artboard-only focus:** focus `Artboard(a)` ⇒
  `contexts_inner_to_outer` = `[Artboard(a), Global]` (bubble still reaches Global).

### 6.2 `resources/input_system.rs` dispatch tests

- **Set-wide nudge:** construct a heterogeneous selection directly, dispatch
  `InputAction::Key { ArrowRight, pressed: true }`, assert exactly one deduped
  `MoveArtboard`/`MoveLayer` per `move_targets()` member, each with the expected world delta
  (`screen_delta_to_world(unit * NUDGE_STEP)`).
- **Empty selection + arrow → no events** (handler returns `No`, nothing bubbles into a move).
- **Non-arrow key bubbles:** with a layer focus, `Cmd+R` still yields `ClearLayer` (falls
  through the set phase to the layer handler) — the set phase must not swallow it.
- **Spatial dispatch intact:** existing pointer/scroll bubble tests stay green, proving the
  set phase doesn't perturb draw / cmd-drag / zoom / pan / hit-select.

## 7. Verification

```
cargo test -p crayon input::selection
cargo test -p crayon input_system
cargo clippy -p crayon        # watch for needless-lifetime / needless-match on the
                              # rewritten contexts_inner_to_outer
```

All green + clean ⇒ the heterogeneous selection is expressible and dispatch provably
respects it, with no change to existing single-select spatial behavior.
