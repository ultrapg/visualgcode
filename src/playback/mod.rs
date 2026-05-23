#[derive(Clone, Debug)]
pub struct PlaybackState {
    pub playing: bool,
    pub current_segment_index: u32,
    pub total_segments: u32,
    pub current_layer_index: usize,
    pub total_layers: usize,
    pub speed: f32,
    elapsed: f64,
}

impl PlaybackState {
    pub fn new() -> Self {
        Self {
            playing: false,
            current_segment_index: 0,
            total_segments: 0,
            current_layer_index: 0,
            total_layers: 0,
            speed: 1.0,
            elapsed: 0.0,
        }
    }

    pub fn reset(&mut self, total_segments: u32, total_layers: usize) {
        self.current_segment_index = 0;
        self.total_segments = total_segments;
        self.total_layers = total_layers;
        self.current_layer_index = 0;
        self.playing = false;
        self.elapsed = 0.0;
    }

    #[allow(dead_code)]
    pub fn set_progress(&mut self, segment_index: u32) {
        self.current_segment_index = segment_index.min(self.total_segments.saturating_sub(1));
        self.elapsed = 0.0;
    }

    #[allow(dead_code)]
    pub fn step_forward(&mut self) {
        if self.total_segments > 0 {
            self.current_segment_index =
                (self.current_segment_index + 1).min(self.total_segments - 1);
        }
    }

    #[allow(dead_code)]
    pub fn step_backward(&mut self) {
        self.current_segment_index = self.current_segment_index.saturating_sub(1);
    }

    pub fn fast_forward(&mut self) {
        if self.total_segments > 0 {
            let skip = (self.total_segments / 50).max(1);
            self.current_segment_index =
                (self.current_segment_index + skip).min(self.total_segments - 1);
        }
    }

    pub fn rewind(&mut self) {
        let skip = (self.total_segments / 50).max(1);
        self.current_segment_index = self.current_segment_index.saturating_sub(skip);
    }

    pub fn advance(&mut self, dt: f64) {
        if !self.playing || self.total_segments == 0 {
            return;
        }
        self.elapsed += dt * self.speed as f64;
        let segments_per_second = 500.0;
        let increment = (self.elapsed * segments_per_second) as u32;
        if increment > 0 {
            self.current_segment_index =
                (self.current_segment_index + increment).min(self.total_segments - 1);
            self.elapsed -= increment as f64 / segments_per_second;
        }
        if self.current_segment_index >= self.total_segments - 1 {
            self.playing = false;
            self.current_segment_index = self.total_segments - 1;
        }
    }

    pub fn progress(&self) -> f32 {
        if self.total_segments == 0 {
            return 0.0;
        }
        self.current_segment_index as f32 / (self.total_segments - 1).max(1) as f32
    }
}
