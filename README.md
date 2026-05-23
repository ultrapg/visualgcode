# visualgcode

An interactive G-code visualizer for pen plotters and 3D printers, built with Rust, egui, and wgpu.

Renders 100k+ toolpath segments at 60 FPS using GPU-accelerated vertex pipelines. Supports both 2D top-down and 3D orbit views with step-by-step playback controls.

## Features

**Machine modes**
- **Pen Plotter** — Z > 0 is pen-up (travel moves), Z ≤ 0 is pen-down (drawing moves)
- **3D Printer** — Extrusion is detected via E-value increases between moves

**Visualization**
- **2D Top-Down view** — Pan and zoom over the XY plane
- **3D Orbit view** — Click + drag to orbit, scroll to zoom, right-click + drag to pan
- **Travel vs draw distinction** — Travel moves rendered in a dimmed color, drawing moves in blue
- **Pen up/down indicator** (Plotter mode) or **layer indicator** (Printer mode) in the info panel

**Playback**
- Step-by-step execution — Visualize the toolpath in sequence
- Single-step forward/backward (`|◀` / `▶|`) and skip to start/end (`⏮` / `⏭`)
- Play / Pause (`▶` / `⏸`), auto-rewind when pressing play at the end
- Adjustable playback speed (0.1x to 20x) and configurable segments-per-second rate
- Loop mode — automatically restart from the beginning when reaching the end
- Progress slider — jump to any segment instantly

**File handling**
- Native file-open dialog via `rfd`
- Drag-and-drop `.gcode` / `.nc` / `.cnc` files
- Supports G0 (rapid), G1 (linear feed), G2/G3 (clockwise/counterclockwise arc) moves

**Camera controls**
| Action | 2D View | 3D View |
|---|---|---|
| Pan | Click + drag | Right-click + drag |
| Zoom | Scroll | Scroll |
| Orbit | — | Click + drag |

- **Reset View** — Returns camera to initial position and orientation
- **Center** — Re-centers the camera on the model centroid
- **Fit** — Zooms to frame the entire model within the viewport

**Performance**
- All segments uploaded to the GPU once on load
- Layer and progress culling handled entirely in the vertex shader — no CPU-side vertex re-buffering
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
4. Use the progress slider, step buttons, or Play to explore the toolpath

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
  │   └── mod.rs           PlaybackState: speed, loop, skip/step, play/pause
  ├── renderer/
  │   ├── mod.rs           wgpu pipeline, vertex/uniform buffers, paint callback
  │   └── camera.rs        Mat4 math, Camera2D (orthographic), Camera3D (orbit),
  │                        look_at, perspective/ortho for WGPU [0,1] depth
  └── ui/
      └── mod.rs           egui side panel, viewport, mouse interaction,
                           file dialog, playback controls, status indicators
```

## Shaders

The GPU pipeline uses two custom WGSL shaders:

**Vertex shader** — Applies the view-projection matrix and culls invisible segments:
- `layer_min` / `layer_max` — Hides layers outside the active range
- `max_segment_index` — Implements step-by-step visibility (only segments up to the progress index are shown)

**Fragment shader** — Draws each segment with drawing/travel color.

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
- **Shader-based culling** avoids CPU-side vertex re-upload when the user adjusts the progress slider — only the uniform buffer is updated (4 floats).
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
