# HexxEngine — Agent Instructions

## Workspace

- Rust workspace, 4 members: `engine` (lib), `level-editor`, `character-controller-demo`, `physics-demo` (binaries)
- `engine` re-exports all deps publicly (`pub use ash`, `glam`, `rapier3d`, etc.)
- All 3 demos depend on `engine` via `{ path = "../engine" }`
- Rust edition 2024

## Commands

```
cargo build --workspace                    # build all
cargo run -p level-editor                  # level editor
cargo run -p character-controller-demo     # FPS demo
cargo run -p physics-demo                  # physics demo
cargo check --workspace                    # fastest verification
```

No tests. No CI. No lint/typecheck config.

## Shaders

- Source: `shaders/*.vert`, `shaders/*.frag`, `shaders/common*.glsl`
- Compiled SPIR-V: `assets/*.spv` (gitignored)
- Compile: `bash compile_shaders.sh` (requires `glslc`)
- Windows: `compile_shaders.bat`

## Architecture

- **Vulkan** via `ash` + `vk-mem` + `ash-window`. Vulkan 1.3. Dynamic rendering + Synchronization2. Validation layers hardcoded `ON` in `VulkanContext::new()`.
- **Renderer** (`engine/src/renderer/`):
  - `scene3d/` — 3D rendering: shadow map, skybox, instanced meshes, transparent rendering
  - `scene2d/` — 2D rendering: UI quads, text (FreeType SDF glyph atlas)
  - `renderer.rs` — `VulkanContext` struct, main draw loop, swapchain management, mesh/texture/material loading APIs
- **Scene** (`engine/src/scene.rs`): `RenderScene`, `Camera`, `Lighting`, `MeshNode`, `UiFrame`, `UiText`
- **Physics** (`engine/src/physics/`): Rapier3d integration. `character_controller.rs` for FPS movement.
- **Entities** (`engine/src/entities.rs`): `Part` struct (Transform + Mesh + RigidBody). Generational handles via `thunderdome::Arena`.
- **Components** (`engine/src/components.rs`): `TransformComponent`, `MeshComponent`, `RigidBodyComponent`
- **Assets** (`engine/src/assets.rs`): `ASSET_PATH` = `$CARGO_MANIFEST_DIR/../assets`. GLTF loading, skybox loading.

## Gotchas

- `USE_VALIDATION_LAYERS` is `const true` in `renderer.rs` — needs `VK_LAYER_KHRONOS_validation` installed
- Physical device selection is hardcoded: `physical_devices[0]`
- Descriptor pool ratio in `DESCRIPTOR_RATIOS` is static — may need tuning as features grow
- `render_frames` uses ring buffer of `MAX_FRAMES = 2`
- All demos use `ControlFlow::Poll` (busy render loop)
- `assets/` is gitignored — binary assets (`.spv`, `.bin`, `.gltf`, fonts) must be present at runtime
- `paidtype` crate is a git dependency: `https://github.com/T4rp/paidtype2`
- `.ignore` file ignores `/assets` — may conflict with actual asset usage (verify intent)

## Progress (`docs/Todo.md`)

Track unfinished items there. Key remaining work: multiple lights, material properties, GLTF tree loading, dynamic descriptor pool, descriptor indexing, mouse capture, declarative UI, frustum culling, post-processing, audio.
