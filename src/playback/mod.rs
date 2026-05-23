#[derive(Clone, Debug)]
pub struct PlaybackState {
    pub playing: bool,
    pub current_segment_index: u32,
    pub total_segments: u32,
    pub total_layers: usize,
    pub speed: f32,
    pub loop_playback: bool,
    pub segments_per_second: f32,
    elapsed: f64,
}

impl PlaybackState {
    pub fn new() -> Self {
        Self {
            playing: false,
            current_segment_index: 0,
            total_segments: 0,
            total_layers: 0,
            speed: 1.0,
            loop_playback: false,
            segments_per_second: 500.0,
            elapsed: 0.0,
        }
    }

    pub fn reset(&mut self, total_segments: u32, total_layers: usize) {
        self.current_segment_index = 0;
        self.total_segments = total_segments;
        self.total_layers = total_layers;
        self.playing = false;
        self.elapsed = 0.0;
    }

    pub fn step_forward_one(&mut self) {
        if self.total_segments > 0 {
            self.current_segment_index =
                (self.current_segment_index + 1).min(self.total_segments - 1);
        }
    }

    pub fn step_backward_one(&mut self) {
        self.current_segment_index = self.current_segment_index.saturating_sub(1);
    }

    pub fn skip_to_end(&mut self) {
        if self.total_segments > 0 {
            self.current_segment_index = self.total_segments - 1;
        }
    }

    pub fn skip_to_start(&mut self) {
        self.current_segment_index = 0;
    }

    pub fn advance(&mut self, dt: f64) {
        if !self.playing || self.total_segments == 0 {
            return;
        }
        self.elapsed += dt * self.speed as f64;
        let inc = (self.elapsed * self.segments_per_second as f64) as u32;
        if inc > 0 {
            let next = self.current_segment_index + inc;
            if next >= self.total_segments - 1 {
                if self.loop_playback {
                    self.current_segment_index = 0;
                } else {
                    self.current_segment_index = self.total_segments - 1;
                    self.playing = false;
                }
            } else {
                self.current_segment_index = next;
            }
            self.elapsed = 0.0;
        }
    }

    pub fn progress(&self) -> f32 {
        if self.total_segments == 0 {
            return 0.0;
        }
        self.current_segment_index as f32 / (self.total_segments - 1).max(1) as f32
    }
}
