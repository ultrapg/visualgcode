# visualgcode

An interactive G-code visualizer for pen plotters and 3D printers, built with Rust, egui, and wgpu.

Renders 100k+ toolpath segments at 60 FPS using GPU-accelerated vertex pipelines. Supports both 2D top-down and 3D orbit views with layer-aware playback controls.

## Features

**Machine modes**
- **Pen Plotter** — Z > 0 is pen-up (travel moves), Z ≤ 0 is pen-down (drawing moves)
- **3D Printer** — Extrusion is detected via E-value increases between moves

**Visualization**
- **2D Top-Down view** — Pan and zoom over the XY plane
- **3D Orbit view** — Drag to orbit, scroll to zoom, with smooth perspective camera
- **Layer coloring** — Each Z-layer is rendered in a distinct color for visual separation
- **Travel vs draw distinction** — Travel (non-printing) moves drawn in a dimmed color

**Playback**
- Step-by-step execution — Visualize the toolpath in sequence
- Play / Pause / Fast-Forward / Rewind controls
- Adjustable playback speed
- Layer slider — Jump directly to any Z-layer
- Progress slider — Jump to any segment index instantly
- Auto-rewind — Automatically restarts from the beginning when play is pressed at the end

**File handling**
- Native file-open dialog via `rfd`
- Drag-and-drop `.gcode` / `.nc` / `.cnc` files
- Supports G0 (rapid), G1 (linear feed), G2/G3 (clockwise/counterclockwise arc) moves

**Camera controls**
| Action | 2D View | 3D View |
|---|---|---|
| Pan | Click + drag | Click + drag |
| Zoom | Scroll | Scroll |
| Orbit | — | Right-click + drag |

- **Reset View** — Returns camera to initial position and orientation
- **Center** — Re-centers the camera on the model centroid
- **Fit** — Zooms to frame the entire model within the viewport

**Performance**
- All segments uploaded to the GPU once on load
- Layer visibility and progress culling handled entirely in the vertex shader — no CPU-side vertex re-buffering
- Arc G2/G3 commands tessellated into line segments at ~0.05 rad resolution

## Requirements

- Rust 2021 edition (MSRV: stable, tested on 1.80+)
- A GPU with Vulkan / Metal / DX12 / OpenGL (via GLES/EGL fallback) support
- Linux, macOS, or Windows

### Linux dependencies

On Linux you may need:

```bash
# Ubuntu / Debian
sudo apt install pkg-config libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libwayland-dev libxkbcommon-dev

# Fedora
sudo dnf install pkgconfig libxcb-devel wayland-devel libxkbcommon-devel
```

## Installation & Usage

```bash
# Clone and run
git clone <repo-url> && cd visualgcode
cargo run --release
```

1. Click **Browse** or drop a G-code file onto the window
2. Select the machine mode — **Pen Plotter** or **3D Printer**
3. Switch between **2D Top-Down** and **3D View** as needed
4. Use the layer / progress sliders or Play button to step through the toolpath

## Architecture

```
Cargo.toml
src/
  ├── main.rs              Entry point, wgpu device/limits, window setup
  ├── app.rs               Application state, file loading, per-frame orchestration
  ├── parser/
  │   ├── types.rs         Core data types: Point3D, Segment, Layer, BoundingBox,
  │   │                    GCodeProgram, MachineMode, ViewMode
  │   └── state.rs         Parser state machine: G0/G1/G2/G3 processing, arc
  │                        tessellation, Z-layer grouping, unit tests
  ├── playback/
  │   └── mod.rs           PlaybackState: speed control, play/pause toggling,
  │                        step advancement, ffwd/rwd logic
  ├── renderer/
  │   ├── mod.rs           wgpu pipeline: vertex/index buffers, shader uniforms,
  │                        paint callback with graceful resource recreation
  │   └── camera.rs        Mat4 math utilities, Camera2D (orthographic),
  │                        Camera3D (perspective orbit), look_at, reset/fit
  └── ui/
      └── mod.rs           egui side panel and viewport, mouse interaction,
                           native dialog, all control widgets
```

## Shaders

The GPU pipeline uses two custom WGSL shaders:

**Vertex shader** — Applies the view-projection matrix and culls invisible segments:
- `layer_min` / `layer_max` — Hides layers outside the active range
- `max_segment_index` — Implements step-by-step visibility (only segments up to the progress index are shown)

**Fragment shader** — Draws each segment in its layer color, with travel moves rendered dimmed.

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `eframe` | 0.34 | Window, event loop, egui integration (with `wgpu` feature) |
| `egui` | 0.34 | Immediate-mode GUI framework |
| `egui-wgpu` | 0.34 | Custom wgpu paint callback integration |
| `wgpu` | 29 | GPU rendering, pipeline, and buffer management |
| `gcode` | 0.7 | G-code lexing and tokenization |
| `bytemuck` | 1 | Safe `Pod` derive for vertex/uniform buffer casts |
| `rfd` | 0.15 | Native file open/save dialogs |

## Key Design Decisions

- **`egui-wgpu` + `Callback::new_paint_callback`** chosen over Bevy for a lighter dependency graph and direct wgpu control.
- **Shader-based culling** avoids CPU-side vertex re-upload when the user adjusts layer or progress sliders — only the uniform buffer is updated (4 floats).
- **Arc tessellation** at ~0.05 rad step resolution balances smoothness and vertex count.
- **`downlevel_webgl2_defaults()` limits** required on some hardware with only 4 max color attachments via GLES/EGL fallback.
- **Shared state** (`Arc<RwLock<SharedState>>`) bridges egui's immediate-mode UI with wgpu's retained GPU resources.

## Testing

```bash
cargo test
```

Tests cover the parser: plotting moves, 3D printer extrusion detection, arc tessellation, and empty input handling.

## License

GNU General Public License v3.0
