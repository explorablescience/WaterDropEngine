# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

WaterDropEngine (WDE) is a personal 3D game engine written in Rust. It is built on top of **Bevy's ECS**, uses **wgpu** for rendering via a custom renderer, and **Rapier** for physics. The codebase is a Cargo workspace organized into a main application and multiple crates.

## Commands

```bash
# Build and run (debug mode — recommended for development)
cargo run --bin waterdropengine --package waterdropengine --features log-debug,debug,watch

# Build and run (release)
cargo build --release --bin waterdropengine --package waterdropengine --features log-debug

# Run examples
cargo run --bin wde-examples --package wde-examples --features log-debug,debug,watch

# Run tests
cargo test --workspace --all-targets

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets

# Format (requires nightly)
cargo +nightly fmt --all
cargo +nightly fmt --all --check

# Generate docs
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
```

### Feature flags (for `waterdropengine` and `wde` crates)

| Flag | Effect |
|---|---|
| `log-debug` | Enable debug-level logging |
| `debug` | Enable Bevy debug features |
| `watch` | Enable hot-reloading of assets |
| `tracing` | Enable Tracy/Puffin profiling |
| `gpu-debug` | Enable wgpu validation layer |
| `editor` | Enable the in-game editor UI |

## Workspace structure

```
Cargo.toml              # Workspace root (members: src/, crates/*/*, examples/)
src/                    # Main application (binary: waterdropengine)
  src/main.rs           # Entry point — creates App, adds WdeDefaultPlugins + TestPlugin
  src/test.rs           # TestPlugin: spawns scene entities for development testing
crates/
  core/
    wde/                # Facade crate — re-exports all sub-crates as WdeDefaultPlugins
    wde-renderer/       # Custom wgpu renderer (render graph, pipelines, sync between worlds)
    wde-scene/          # Scene management (entity hierarchy, serialization)
    wde-editor/         # In-game editor UI panels (ECS inspector, logs, profiler, render graph view)
  render/
    wde-camera/         # Camera component and view/projection math
    wde-camera-controller/ # ThirdPersonController and FreeCameraController
    wde-gltf/           # glTF model loader
    wde-pbr/            # Deferred PBR renderer (GBuffer, lighting)
    wde-pbr-outline/    # Stencil-based object outline effect
    wde-gizmos/         # Debug gizmo rendering
  terrain/
    wde-terrain/        # Terrain tiles with heightmaps and splat maps, physics integration
    wde-terrain-editor/ # GPU compute-based terrain painting tools
    wde-terrain-grid/   # LOD grid placement and management for terrain objects
  wrappers/
    wde-wgpu/           # Low-level wgpu abstractions (instance, pipelines, buffers, textures)
    wde-physics/        # Rapier physics wrapper (colliders, raycasting)
    wde-logger/         # Tracing-based logger with editor overlay and panic handler
    wde-egui/           # Egui wrapper (superseded by wde-editor)
examples/               # Standalone examples (custom render pass, PBR batches, ray casting…)
res/                    # Assets loaded at runtime via Bevy's AssetServer (path: "res/")
```

## Architecture

### Plugin system

Everything is a Bevy `Plugin`. The entry point is `WdeDefaultPlugins` ([crates/core/wde/src/lib.rs](crates/core/wde/src/lib.rs)), which composes `CustomBevyPlugins` (minimal Bevy setup) and `CustomWdePlugins` (all WDE sub-plugins). `wde_scene::ScenePlugin` is always added last.

### Dual-world renderer

The renderer runs in a **separate thread** via `PipelinedRenderingPlugin`. There are two Bevy worlds:
- **Main world** — runs game logic.
- **Render world** (in `RenderApp`) — runs the `Extract` then `Render` schedules concurrently with the next main-world frame.

The `Render` schedule has four ordered sets: `ExtractAuto → Prepare → Render → Submit`.

### Sync / Extract

Data flows from the main world to the render world through two mechanisms in `wde-renderer/src/sync/`:

1. **Resources**: Implement `ExtractResource` and add `ExtractResourcePlugin::<T>` to auto-clone a resource each frame.
2. **Entities/components**: Add `SyncComponentPlugin::<C>` to tag entities that have `C` with `SyncToRenderWorld`, which mirrors them in the render world. Then implement `ExtractComponent` + `SyncComponent` to copy component data across, or use the `#[derive(ExtractComponent)]` macro for simple clone-based extraction. `RenderEntity` (on main world entity) and `MainEntity` (on render world entity) link the two.

Use `ExtractWorld<Query<...>>` as a system param inside `Extract` schedule systems to read the main world from the render thread.

### Render graph

The `RenderGraph` resource ([crates/core/wde-renderer/src/passes/render_graph.rs](crates/core/wde-renderer/src/passes/render_graph.rs)) is the execution backbone:
- **`RenderPass`** — defines attachments and load ops; ordered by a numeric `id()` (lower = first).
- **`RenderSubPass`** — defines a sequence of `SubPassCommand`s (set pipeline, bind group, mesh, draw calls, custom fn) executed inside a render pass.
- Register with `render_graph.add_pass::<MyPass>()` and `render_graph.add_sub_pass::<MySubPass, MyPass>()` in the render world.

### Pipeline management

`PipelineManager` ([crates/core/wde-renderer/src/passes/pipeline_manager.rs](crates/core/wde-renderer/src/passes/pipeline_manager.rs)) queues and asynchronously builds `RenderPipeline` / `ComputePipeline` objects from `RenderPipelineDescriptor` / `ComputePipelineDescriptor`. Use `RenderPipelineRegisterPlugin<P>` to register a pipeline asset type that implements `RenderAsset`.

### Asset system

GPU assets implement the `RenderAsset` trait (`prepare()` creates the GPU object from a CPU Bevy asset). They are stored in `RenderAssets<A>` in the render world. `RenderBinding` + `RenderBindingRegisterPlugin` manage wgpu bind groups backed by GPU buffers and textures. Shaders are loaded via Bevy's `AssetServer` from the `res/` directory.

### Physics

`wde-physics` wraps Rapier. Attach a `Collider` component (cuboid, sphere, heightfield, etc.) with a `ColliderGroup` to any entity with a `Transform`. Use `PhysicsWorld` resource for raycasting. Terrain uses `TERRAIN_COLLIDER_GROUP` (GROUP_1) and buildings use `TERRAIN_BUILDINGS_COLLIDER_GROUP` (GROUP_2).

## Code style

- Edition 2024, formatted with `cargo +nightly fmt` (config in [rustfmt.toml](rustfmt.toml): trailing commas off, Unix newlines, field init shorthand).
- Clippy is run with `-D warnings` — zero warnings expected.
- The `#![allow(clippy::type_complexity)]` suppression is present in `main.rs`; prefer not proliferating it.
