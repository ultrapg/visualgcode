use std::sync::{Arc, RwLock};

use egui::Vec2;

use crate::parser::{self, MachineMode, ViewMode};
use crate::playback::PlaybackState;
use crate::renderer;
use crate::renderer::camera::{Camera2D, Camera3D};

pub struct ViewportState {
    pub camera_2d: Camera2D,
    pub camera_3d: Camera3D,
    pub view_mode: ViewMode,
    bounds: Option<parser::BoundingBox>,
}

impl ViewportState {
    pub fn new(bounds: Option<&parser::BoundingBox>) -> Self {
        let center = bounds.map(|b| b.center()).unwrap_or(parser::Point3D { x: 0.0, y: 0.0, z: 0.0 });
        let target = [center.x, center.y, center.z];
        let dist = bounds
            .map(|b| {
                let s = b.size();
                (s.x * s.x + s.y * s.y + s.z * s.z).sqrt().max(1.0) * 1.5
            })
            .unwrap_or(20.0);
        Self {
            camera_2d: Camera2D::new(),
            camera_3d: Camera3D::new(target, dist),
            view_mode: ViewMode::View3D,
            bounds: bounds.cloned(),
        }
    }

    fn bounds_size(&self) -> parser::Point3D {
        self.bounds.as_ref().map(|b| b.size()).unwrap_or(parser::Point3D { x: 1.0, y: 1.0, z: 1.0 })
    }

    fn fit_distance(&self) -> f32 {
        let size = self.bounds_size();
        (size.x * size.x + size.y * size.y).sqrt().max(1.0) * 2.0
    }

    fn center_point(&self) -> [f32; 3] {
        self.bounds.as_ref().map(|b| {
            let c = b.center();
            [c.x, c.y, c.z]
        }).unwrap_or([0.0; 3])
    }

    pub fn reset_view(&mut self) {
        self.camera_3d.reset();
        self.camera_2d.reset();
    }

    pub fn center_view(&mut self) {
        let center = self.center_point();
        self.camera_3d.center_on(center);
        self.camera_2d.pan = [0.0, 0.0, 0.0];
    }

    pub fn fit_view(&mut self) {
        let center = self.center_point();
        let dist = self.fit_distance();
        self.camera_3d.fit(center, dist);
        if let Some(b) = &self.bounds {
            let model_w = (b.max_x - b.min_x).max(0.001);
            let model_h = (b.max_y - b.min_y).max(0.001);
            let model_cx = (b.min_x + b.max_x) * 0.5;
            let model_cy = (b.min_y + b.max_y) * 0.5;
            let zoom_x = 20.0 * 0.8 / model_w;
            let zoom_y = 20.0 * 0.8 / model_h;
            self.camera_2d.zoom = zoom_x.min(zoom_y).max(0.01);
            self.camera_2d.pan = [model_cx, model_cy, 0.0];
        }
    }
}

/// Returns `true` if a file load was requested via the button.
pub fn show_ui(
    ui: &mut egui::Ui,
    shared_state: &Arc<RwLock<renderer::SharedState>>,
    viewport: &mut ViewportState,
    machine_mode: &mut MachineMode,
    playback: &mut PlaybackState,
    program: &Option<parser::GCodeProgram>,
    file_path: &mut String,
    status: &mut String,
) -> bool {
    let mut load_requested = false;

    egui::Panel::left("side_panel")
        .min_size(220.0)
        .default_size(250.0)
        .resizable(true)
        .show_inside(ui, |ui| {
            ui.heading("Visual G-Code");
            ui.separator();
            load_requested = draw_file_section(ui, file_path, status, playback, *machine_mode);
            ui.separator();
            draw_mode_section(ui, machine_mode);
            ui.separator();
            draw_view_section(ui, viewport);
            ui.separator();
            draw_playback_section(ui, playback, shared_state);
            ui.separator();
            draw_info_section(ui, playback, program, *machine_mode, status);
        });

    let avail = ui.available_size();
    let size = Vec2::new(avail.x, avail.y);
    let (rect, response) =
        ui.allocate_exact_size(size, egui::Sense::drag());

    let aspect = size.x / size.y.max(1.0);

    let needs_repaint = handle_viewport_input(&response, ui, viewport, aspect, size.y);

    let view_proj = match viewport.view_mode {
        ViewMode::View2D => viewport.camera_2d.view_proj(aspect),
        ViewMode::View3D => viewport.camera_3d.view_proj(aspect),
    };

    {
        let mut state = shared_state.write().unwrap();
        state.view_proj = view_proj;
        if playback.total_segments > 0 {
            state.max_segment_index = playback.current_segment_index;
        }
    }

    let cb = renderer::RenderCallback::new(shared_state.clone());
    let paint_callback = egui_wgpu::Callback::new_paint_callback(rect, cb);
    ui.painter().add(paint_callback);

    if needs_repaint {
        ui.ctx().request_repaint();
    }

    load_requested
}

fn handle_viewport_input(
    response: &egui::Response,
    ui: &egui::Ui,
    viewport: &mut ViewportState,
    aspect: f32,
    viewport_height: f32,
) -> bool {
    let mut needs_repaint = false;

    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta);
        if scroll.y != 0.0 {
            match viewport.view_mode {
                ViewMode::View2D => {
                    let s = 1.0 + scroll.y * 0.05;
                    viewport.camera_2d.zoom = (viewport.camera_2d.zoom * s).clamp(0.01, 1000.0);
                }
            ViewMode::View3D => {
                let s = 1.0 + scroll.y * 0.03;
                viewport.camera_3d.distance =
                    (viewport.camera_3d.distance / s).clamp(0.1, 10000.0);
            }
            }
            needs_repaint = true;
        }
    }

    if response.dragged() {
        let delta = response.drag_delta();
        let primary = ui.input(|i| i.pointer.primary_down());

        match viewport.view_mode {
            ViewMode::View2D => {
                let z = viewport.camera_2d.zoom.max(0.01);
                let half_h = 10.0 / z;
                let half_w = aspect * 10.0 / z;
                let scale_x = (2.0 * half_w) / viewport_height.max(1.0);
                let scale_y = (2.0 * half_h) / viewport_height.max(1.0);
                viewport.camera_2d.pan[0] -= delta.x * scale_x;
                viewport.camera_2d.pan[1] += delta.y * scale_y;
            }
            ViewMode::View3D => {
                if primary {
                    // Orbit: drag rotates the view around the target
                    viewport.camera_3d.yaw -= delta.x * 0.004;
                    viewport.camera_3d.pitch =
                        (viewport.camera_3d.pitch - delta.y * 0.004).clamp(-1.5, 1.5);
                } else {
                    // Pan: move target in screen-aligned directions (right/up)
                    let dist = viewport.camera_3d.distance.max(0.1);
                    let scale = dist * 0.001;
                    let (sy, cy) = viewport.camera_3d.yaw.sin_cos();
                    let (sp, cp) = viewport.camera_3d.pitch.sin_cos();
                    let rx = -sy;
                    let ry = cy;
                    let ux = -cy * sp;
                    let uy = -sy * sp;
                    let uz = cp;
                    viewport.camera_3d.target[0] -= (delta.x * rx - delta.y * ux) * scale;
                    viewport.camera_3d.target[1] -= (delta.x * ry - delta.y * uy) * scale;
                    viewport.camera_3d.target[2] -= (-delta.y) * uz * scale;
                }
            }
        }
        needs_repaint = true;
    }

    needs_repaint
}

fn draw_file_section(
    ui: &mut egui::Ui,
    file_path: &mut String,
    status: &mut String,
    _playback: &mut PlaybackState,
    _machine_mode: MachineMode,
) -> bool {
    let mut load_requested = false;

    ui.label("G-Code File");
    ui.horizontal(|ui| {
        ui.label("Path:");
        let resp = ui.add(
            egui::TextEdit::singleline(file_path)
                .hint_text("path/to/file.gcode"),
        );
        if resp.changed() {
            *status = String::new();
        }
        if ui.button("Browse...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("G-Code", &["gcode", "gco", "g", "nc", "ngc"])
                .pick_file()
            {
                *file_path = path.to_string_lossy().to_string();
                load_requested = true;
            }
        }
    });
    if ui.button("Load").clicked() && !file_path.is_empty() {
        load_requested = true;
    }

    load_requested
}

fn draw_mode_section(ui: &mut egui::Ui, machine_mode: &mut MachineMode) {
    egui::ComboBox::from_label("Machine Mode")
        .selected_text(machine_mode.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(machine_mode, MachineMode::Plotter, "Pen Plotter");
            ui.selectable_value(machine_mode, MachineMode::Printer3D, "3D Printer");
        });
}

fn draw_view_section(ui: &mut egui::Ui, viewport: &mut ViewportState) {
    egui::ComboBox::from_label("View Mode")
        .selected_text(viewport.view_mode.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut viewport.view_mode, ViewMode::View2D, "2D Top-Down");
            ui.selectable_value(&mut viewport.view_mode, ViewMode::View3D, "3D View");
        });

    ui.horizontal(|ui| {
        if ui.button("Reset View").clicked() {
            viewport.reset_view();
            ui.ctx().request_repaint();
        }
        if ui.button("Center").clicked() {
            viewport.center_view();
            ui.ctx().request_repaint();
        }
        if ui.button("Fit").clicked() {
            viewport.fit_view();
            ui.ctx().request_repaint();
        }
    });
}

fn draw_playback_section(
    ui: &mut egui::Ui,
    playback: &mut PlaybackState,
    shared_state: &Arc<RwLock<renderer::SharedState>>,
) {
    ui.label("Playback");

    ui.horizontal(|ui| {
        ui.add_enabled_ui(playback.total_segments > 0, |ui| {
            if ui.button("⏮").clicked() {
                playback.skip_to_start();
                playback.playing = false;
                let mut state = shared_state.write().unwrap();
                state.max_segment_index = playback.current_segment_index;
                ui.ctx().request_repaint();
            }
            if ui.button("|◀").clicked() {
                playback.step_backward_one();
                let mut state = shared_state.write().unwrap();
                state.max_segment_index = playback.current_segment_index;
                ui.ctx().request_repaint();
            }
            let play_label = if playback.playing { "⏸" } else { "▶" };
            if ui.button(play_label).clicked() {
                if !playback.playing && playback.current_segment_index >= playback.total_segments.saturating_sub(1) {
                    playback.current_segment_index = 0;
                }
                playback.playing = !playback.playing;
                let mut state = shared_state.write().unwrap();
                state.max_segment_index = playback.current_segment_index;
                ui.ctx().request_repaint();
            }
            if ui.button("▶|").clicked() {
                playback.step_forward_one();
                let mut state = shared_state.write().unwrap();
                state.max_segment_index = playback.current_segment_index;
                ui.ctx().request_repaint();
            }
            if ui.button("⏭").clicked() {
                playback.skip_to_end();
                playback.playing = false;
                let mut state = shared_state.write().unwrap();
                state.max_segment_index = playback.current_segment_index;
                ui.ctx().request_repaint();
            }
        });
    });

    ui.horizontal(|ui| {
        ui.label("Speed:");
        ui.add(
            egui::Slider::new(&mut playback.speed, 0.1..=20.0)
                .step_by(0.1)
                .text("x"),
        );
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut playback.loop_playback, "Loop");
    });

    if ui
        .add(
            egui::Slider::new(&mut playback.current_segment_index, 0..=playback.total_segments.saturating_sub(1).max(0))
                .text("Progress"),
        )
        .changed()
    {
        let mut state = shared_state.write().unwrap();
        state.max_segment_index = playback.current_segment_index;
        ui.ctx().request_repaint();
    }
}

fn draw_info_section(
    ui: &mut egui::Ui,
    playback: &PlaybackState,
    program: &Option<parser::GCodeProgram>,
    machine_mode: MachineMode,
    status: &str,
) {
    ui.label("Info");
    ui.label(status);

    if playback.total_segments > 0 {
        ui.label(format!("Segments: {}", playback.total_segments));
        ui.label(format!("Layers: {}", playback.total_layers));
        ui.label(format!(
            "Progress: {:.1}%",
            playback.progress() * 100.0
        ));

        // Current segment indicator
        if let Some(prog) = program {
            let idx = playback.current_segment_index.min(prog.segments.len() as u32 - 1) as usize;
            if idx < prog.segments.len() {
                let seg = &prog.segments[idx];
                match machine_mode {
                    MachineMode::Printer3D => {
                        let li = seg.layer_index as usize;
                        if li < prog.layers.len() {
                            let layer = &prog.layers[li];
                            ui.label(format!(
                                "Layer {}/{} (Z={:.2})",
                                li + 1,
                                prog.layers.len(),
                                layer.z_height,
                            ));
                        }
                    }
                    MachineMode::Plotter => {
                        let state = if seg.is_drawing { "Down" } else { "Up" };
                        ui.label(format!("Pen {}", state));
                    }
                }
            }
        }
    }
}
