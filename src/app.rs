use std::sync::{Arc, RwLock};

use crate::parser::{self, MachineMode};
use crate::playback::PlaybackState;
use crate::renderer;
use crate::ui::ViewportState;

pub struct App {
    program: Option<parser::GCodeProgram>,
    file_path: String,
    status: String,
    machine_mode: MachineMode,
    viewport: ViewportState,
    playback: PlaybackState,
    shared_state: Arc<RwLock<renderer::SharedState>>,
    last_frame_time: std::time::Instant,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let shared = Arc::new(RwLock::new(renderer::SharedState::new(None)));

        if let Some(render_state) = &cc.wgpu_render_state {
            let device = &render_state.device;
            let target_format = render_state.target_format;

            let resources = renderer::create_render_resources(device, target_format);
            render_state
                .renderer
                .write()
                .callback_resources
                .insert(resources);

            shared.write().unwrap().target_format = Some(target_format);
        }

        Self {
            program: None,
            file_path: String::new(),
            status: String::from("Load a G-code file to begin"),
            machine_mode: MachineMode::Printer3D,
            viewport: ViewportState::new(None),
            playback: PlaybackState::new(),
            shared_state: shared,
            last_frame_time: std::time::Instant::now(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f64();
        self.last_frame_time = now;

        // Advance playback timer
        if self.playback.playing && self.playback.total_segments > 0 {
            let prev = self.playback.current_segment_index;
            self.playback.advance(dt);
            if self.playback.current_segment_index != prev {
                // Step-by-step: reveal segments up to current position
                let mut state = self.shared_state.write().unwrap();
                state.max_segment_index = self.playback.current_segment_index;
            }
            ui.ctx().request_repaint();
        }

        let prev_mode = self.machine_mode;

        let load_requested = crate::ui::show_ui(
            ui,
            &self.shared_state,
            &mut self.viewport,
            &mut self.machine_mode,
            &mut self.playback,
            &mut self.file_path,
            &mut self.status,
        );

        // React to file load request from the button
        if load_requested && !self.file_path.is_empty() {
            let path = self.file_path.clone();
            self.load_file(&path);
        }

        // Re-parse if machine mode changed and we have a file loaded
        if self.machine_mode != prev_mode && self.program.is_some() && !self.file_path.is_empty() {
            let path = self.file_path.clone();
            self.load_file(&path);
        }

        // Check for dropped files via egui input system
        let mut drop_path = String::new();
        ui.input(|i| {
            for dropped in &i.raw.dropped_files {
                if let Some(path) = &dropped.path {
                    drop_path = path.to_string_lossy().to_string();
                }
            }
        });
        if !drop_path.is_empty() {
            self.file_path = drop_path.clone();
            self.load_file(&drop_path);
        }
    }
}

impl App {
    fn load_file(&mut self, path: &str) {
        match std::fs::read_to_string(path) {
            Ok(source) => {
                match parser::parse_gcode(&source, self.machine_mode) {
                    Ok(program) => {
                        let total = program.segments.len();
                        let layers = program.layers.len();
                        let bounds = program.bounds.clone();

                        self.program = Some(program.clone());
                        self.playback.reset(total as u32, layers);
                        // Default to showing full progress (all segments visible)
                        self.playback.current_segment_index = (total as u32).saturating_sub(1);
                        self.viewport = ViewportState::new(Some(&bounds));
                        self.status =
                            format!("Loaded: {} segments, {} layers", total, layers);

                        // Signal renderer to upload new data
                        let mut state = self.shared_state.write().unwrap();
                        state.program = Some(program);
                        state.needs_upload = true;
                        state.layer_min = 0;
                        state.layer_max = u32::MAX;
                        state.max_segment_index = self.playback.current_segment_index;
                    }
                    Err(e) => {
                        self.status = format!("Parse error: {}", e);
                    }
                }
            }
            Err(e) => {
                self.status = format!("File error: {}", e);
            }
        }
    }
}
