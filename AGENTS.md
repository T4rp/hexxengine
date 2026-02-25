# Build, Test, and Coding Guidelines for hexxengine

## Build Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Build with specific profile
cargo build -p hexxengine
```

## Test Commands

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode
cargo test --release

# Run specific test
cargo test test_name

# Run tests with verbose output
cargo test -- --nocapture --verbose
```

## Lint Commands

```bash
# Check for linting issues (Rust default is rustfmt)
cargo check

# Format code
cargo fmt

# Format and check
cargo fmt --check
```

## Project Structure

- `src/main.rs` - Entry point with engine initialization
- `src/game.rs` - Game logic with physics integration
- `src/input.rs` - Input handling and event processing
- `src/physics.rs` - Physics context and calculations
- `src/assets.rs` - Asset loading and management
- `src/scene.rs` - Scene management and entity management
- `src/color.rs` - Color conversion utilities
- `shaders/` - GLSL shader files
- `docs/Todo.md` - Development notes

## Code Style Guidelines

### Imports
- Import standard library modules at top of file
- Import project modules and third-party crates
- Organize imports in logical groups with blank lines
- Use `pub` for public API elements

### Formatting
- Use 4-space indentation
- Keep lines under 80-100 characters when possible
- Add blank lines between logical sections
- Use consistent spacing around operators

### Naming Conventions
- Use `snake_case` for variables and functions
- Use `CamelCase` for types and structs
- Use `UPPER_CASE` for constants and statics
- Use `mCamelCase` for internal module naming

### Types and Structures
- Use `pub struct` for public API structures
- Use `#[derive(...)]` for trait implementations
- Use `Option<T>` and `Result<T, E>` for error handling
- Use `unwrap()` and `expect()` for expected errors

### Error Handling
- Handle `Result` and `Option` types appropriately
- Use `unwrap()` only for expected errors
- Use `expect()` with descriptive error messages
- Use `?` operator for propagating errors

### Modules
- Use `mod` to define modules
- Use `use` statements to import module contents
- Use `pub` to make module visible

### Dependencies
- Uses ash for Vulkan bindings
- Uses ash-window for window integration
- Uses glam for vector math
- Uses gltf for 3D model loading
- Uses image for image processing
- Uses nalgebra for numerical operations
- Uses rand for random number generation
- Uses rapier3d for physics simulation
- Uses thunderdome for arena allocation
- Uses vk-mem for Vulkan memory management
- Uses winit for window and input handling

### Physics Integration
- Physics calculations integrated in game logic
- Using rapier3d for 3D physics simulation
- Physics context in physics.rs module
- Scene management for entity handling

### Asset Management
- Asset loading in assets.rs module
- Model loading via gltf crate
- Image processing via image crate
- Asset management for 3D content

### Shader Files
- GLSL shaders in shaders/ directory
- Shader compilation via compile_shaders.sh script
- Common GLSL shader file (common.glsl) with shared utilities

## Renderer Architecture Analysis

### Key Structures and Components

**Main Rendering Structures:**
- `VulkanContext` - Main rendering context managing the entire pipeline
- `GlobalDescriptors` - Manages descriptor sets for uniform buffers and combined image samplers
- `Texture` - Texture creation and image view management
- `MeshBuffer` - Vertex and index buffer management
- `MeshBatch` - Mesh instance batch management
- `MaterialDescriptor` - Material uniform management
- `TextureDescriptors` - Texture descriptor set management
- `DescriptorSetLayouts` - Descriptor set layout management
- `RenderFrame` - Frame rendering state management per frame

**Shader Modules:**
- Main graphics pipeline shader modules
- Shadow graphics pipeline shader modules
- Skybox graphics pipeline shader modules
- Common GLSL shared shader utilities
- SPV bytecode compilation via compile_shaders.sh

**Rendering Pipeline Features:**
- Instance rendering with up to 10,000 instances (`MAX_INSTANCE_COUNT`)
- Shadow mapping with 1024x1024 resolution
- Multiple descriptor pools for uniform buffers and combined image samplers
- Shader modules compiled from SPV bytecode
- Command pools and buffers for GPU command submission
- Memory allocation with vk_mem allocator
- Double buffering with fences and semaphores (MAX_FRAMES = 2)
- Shadow mapping with custom compare sampler (GREATER mode)
- Depth buffer management with D32_SFLOAT format
- Image layout transitions with barrier management
- Dynamic rendering with VK_KHR_dynamic_rendering
- Camera, Scene, and Material uniform structures

### Vertex and Attribute Structures

**Mesh Vertex Attributes:**
- `Vertex2d` - 2D vertex structure
- `MeshVertex` - 3D vertex structure with position, normal, tex_coords, color
- `InstanceVertex` - Instance vertex structure with model matrix, normal matrix, color, opacity
- Vertex attribute descriptions for Vulkan
- Vertex binding descriptions and stride calculations
- 9 components for instance rendering

**Shader Structures:**
- `MaterialFlags` - Enum with Uv (uvs) and ModelSpace (normals in model space) flags
- `Camera` - Camera uniform data
- `Scene` - Scene uniform data
- `Material` - Material uniforms including flags and parameters

**Shader Compilation:**
- compile_shaders.sh script for SPV bytecode compilation
- Shader modules created from byte data
- Multiple shader modules for different purposes (main_graphics, shadow_graphics, skybox_graphics)
- Common GLSL shader file with shared utilities

### Project Structure Details

**Core Modules:**
- `src/main.rs` - Entry point with engine initialization
- `src/renderer/renderer.rs` - Main rendering pipeline implementation (1512+ lines)
- `src/renderer/mesh.rs` - Mesh and vertex structures (191 lines)
- `src/renderer/` - Renderer module directory
- `shaders/` - GLSL shader files directory
- `docs/Todo.md` - Development notes and roadmap

**Asset Management:**
- Asset loading in assets.rs module
- Model loading via gltf crate
- Image processing via image crate
- Asset management for 3D content

### Physics Integration

**Physics Context:**
- Physics calculations integrated in game logic
- Using rapier3d for 3D physics simulation
- Physics context in physics.rs module
- Scene management for entity handling
- Physics context in physics.rs module

### Development Notes

- See `docs/Todo.md` for development roadmap and notes
- Shader compilation via compile_shaders.sh script
- Common GLSL shader file (common.glsl) with shared utilities
