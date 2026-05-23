use gcode::{core::Number, parse, Code, Value};

use super::types::*;

struct GCodeParser {
    pos: Point3D,
    last_e: f32,
    relative: bool,
    layer_start_z: Option<f32>,
    layer_index: u32,
    segment_index: u32,
    segments: Vec<Segment>,
    layers: Vec<Layer>,
    bounds: BoundingBox,
    current_layer_first_segment: usize,
}

impl GCodeParser {
    fn new() -> Self {
        Self {
            pos: Point3D::default(),
            last_e: 0.0,
            relative: false,
            layer_start_z: None,
            layer_index: 0,
            segment_index: 0,
            segments: Vec::new(),
            layers: Vec::new(),
            bounds: BoundingBox::new(),
            current_layer_first_segment: 0,
        }
    }

    fn flush_layer(&mut self) {
        let start = self.current_layer_first_segment;
        let end = self.segments.len();
        if end > start {
            self.layers.push(Layer {
                segment_start: start,
                segment_end: end,
                z_height: self.layer_start_z.unwrap_or(0.0),
            });
        }
    }

    fn ensure_layer(&mut self, z: f32) {
        let eps = 0.001;
        match self.layer_start_z {
            None => {
                self.layer_start_z = Some(z);
            }
            Some(prev_z) if (prev_z - z).abs() > eps => {
                self.flush_layer();
                self.layer_index += 1;
                self.layer_start_z = Some(z);
                self.current_layer_first_segment = self.segments.len();
            }
            _ => {}
        }
    }

    fn record_move(&mut self, end: Point3D, is_drawing: bool) {
        let seg = Segment {
            start: self.pos,
            end,
            is_drawing,
            layer_index: self.layer_index,
            segment_index: self.segment_index,
        };
        self.segment_index += 1;
        self.bounds.extend(&end);
        self.segments.push(seg);
        self.pos = end;
    }

    fn handle_g0(&mut self, end: Point3D) {
        self.ensure_layer(end.z);
        self.record_move(end, false);
    }

    fn handle_g1(&mut self, end: Point3D, e: Option<f32>, mode: MachineMode) {
        self.ensure_layer(end.z);
        let is_drawing = match mode {
            MachineMode::Plotter => end.z <= 0.001,
            MachineMode::Printer3D => {
                if let Some(e) = e {
                    e > self.last_e + 0.001
                } else {
                    false
                }
            }
        };
        if let Some(e) = e {
            self.last_e = e;
        }
        self.record_move(end, is_drawing);
    }

    fn handle_arc(
        &mut self,
        end: Point3D,
        i: f32,
        j: f32,
        clockwise: bool,
        e: Option<f32>,
        mode: MachineMode,
    ) {
        self.ensure_layer(end.z);
        let is_drawing = match mode {
            MachineMode::Plotter => end.z <= 0.001,
            MachineMode::Printer3D => {
                if let Some(e) = e {
                    e > self.last_e + 0.001
                } else {
                    false
                }
            }
        };
        if let Some(e) = e {
            self.last_e = e;
        }

        let cx = self.pos.x + i;
        let cy = self.pos.y + j;
        let dx_start = self.pos.x - cx;
        let dy_start = self.pos.y - cy;
        let dx_end = end.x - cx;
        let dy_end = end.y - cy;

        let start_angle = dy_start.atan2(dx_start);
        let mut end_angle = dy_end.atan2(dx_end);

        let radius = (dx_start * dx_start + dy_start * dy_start).sqrt();
        if radius < 0.001 {
            self.record_move(end, is_drawing);
            return;
        }

        if clockwise {
            while end_angle > start_angle - 1e-6 {
                end_angle -= std::f32::consts::TAU;
            }
        } else {
            while end_angle < start_angle + 1e-6 {
                end_angle += std::f32::consts::TAU;
            }
        }

        let angle_step = 0.05;
        let total_arc = end_angle - start_angle;
        let steps = (total_arc.abs() / angle_step).ceil() as usize;
        let steps = steps.max(1);

        let mut prev = self.pos;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let angle = start_angle + total_arc * t;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin();
            let z = prev.z + (end.z - prev.z) * t;
            let p = Point3D { x, y, z };
            let seg = Segment {
                start: prev,
                end: p,
                is_drawing,
                layer_index: self.layer_index,
                segment_index: self.segment_index,
            };
            self.segment_index += 1;
            self.bounds.extend(&p);
            self.segments.push(seg);
            prev = p;
        }
        self.pos = prev;
    }
}

pub fn parse_gcode(source: &str, mode: MachineMode) -> Result<GCodeProgram, String> {
    let program = parse(source).map_err(|e| format!("Parse error: {:?}", e))?;

    let mut p = GCodeParser::new();

    for block in &program.blocks {
        let mut target_x: Option<f32> = None;
        let mut target_y: Option<f32> = None;
        let mut target_z: Option<f32> = None;
        let mut target_e: Option<f32> = None;
        let mut target_i: Option<f32> = None;
        let mut target_j: Option<f32> = None;
        let mut target_r: Option<f32> = None;
        let mut _target_f: Option<f32> = None;
        let mut has_g00 = false;
        let mut has_g01 = false;
        let mut has_g02 = false;
        let mut has_g03 = false;

        // Collect from code arguments (G0 X.. Y.. etc)
        for code in &block.codes {
            if let Code::General(g) = code {
                for arg in &g.args {
                    if let Value::Literal(v) = arg.value {
                        match arg.letter {
                            'X' => target_x = Some(v),
                            'Y' => target_y = Some(v),
                            'Z' => target_z = Some(v),
                            'E' => target_e = Some(v),
                            'I' => target_i = Some(v),
                            'J' => target_j = Some(v),
                            'R' => target_r = Some(v),
                            'F' => _target_f = Some(v),
                            _ => {}
                        }
                    }
                }
                match g.number {
                    num if num == Number::new(0) => has_g00 = true,
                    num if num == Number::new(1) => has_g01 = true,
                    num if num == Number::new(2) => has_g02 = true,
                    num if num == Number::new(3) => has_g03 = true,
                    _ => {}
                }
            }
        }

        // Collect from bare word addresses (some gcode puts them at block level)
        for w in &block.word_addresses {
            if let Value::Literal(v) = w.value {
                match w.letter {
                    'X' => target_x = Some(v),
                    'Y' => target_y = Some(v),
                    'Z' => target_z = Some(v),
                    'E' => target_e = Some(v),
                    'I' => target_i = Some(v),
                    'J' => target_j = Some(v),
                            'R' => target_r = Some(v),
                            'F' => _target_f = Some(v),
                    _ => {}
                }
            }
        }

        let has_move = has_g00 || has_g01 || has_g02 || has_g03;

        if !has_move {
            // Check for G90/G91
            for code in &block.codes {
                if let Code::General(g) = code {
                    if g.number == Number::new(90) {
                        p.relative = false;
                    } else if g.number == Number::new(91) {
                        p.relative = true;
                    }
                }
            }
            continue;
        }

        let nx = target_x.unwrap_or(p.pos.x);
        let ny = target_y.unwrap_or(p.pos.y);
        let nz = target_z.unwrap_or(p.pos.z);
        let end = if p.relative {
            Point3D {
                x: p.pos.x + target_x.unwrap_or(0.0),
                y: p.pos.y + target_y.unwrap_or(0.0),
                z: p.pos.z + target_z.unwrap_or(0.0),
            }
        } else {
            Point3D {
                x: nx,
                y: ny,
                z: nz,
            }
        };

        if has_g00 {
            p.handle_g0(end);
        } else if has_g01 {
            p.handle_g1(end, target_e, mode);
        } else if has_g02 || has_g03 {
            let i = target_i.unwrap_or(0.0);
            let j = target_j.unwrap_or(0.0);
            if target_r.is_some() && i == 0.0 && j == 0.0 {
                // R mode arc
                let r = target_r.unwrap();
                let dx = end.x - p.pos.x;
                let dy = end.y - p.pos.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > 0.0 && r.abs() >= dist * 0.5 {
                    let h = (r * r - (dist * 0.5) * (dist * 0.5)).sqrt();
                    let mx = (p.pos.x + end.x) * 0.5;
                    let my = (p.pos.y + end.y) * 0.5;
                    let perp_x = -dy / dist * h;
                    let perp_y = dx / dist * h;
                    // Use perpendicular offset for center
                    // For CW (G2) vs CCW (G3), the center side differs.
                    // Simplification: always pick one side.
                    let (ci, cj) = if has_g02 {
                        (mx - p.pos.x + perp_x, my - p.pos.y + perp_y)
                    } else {
                        (mx - p.pos.x - perp_x, my - p.pos.y - perp_y)
                    };
                    p.handle_arc(end, ci, cj, has_g02, target_e, mode);
                } else {
                    p.handle_g1(end, target_e, mode);
                }
            } else {
                p.handle_arc(end, i, j, has_g02, target_e, mode);
            }
        }
    }

    p.flush_layer();

    Ok(GCodeProgram {
        segments: p.segments,
        layers: p.layers,
        bounds: p.bounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plotter_square() {
        let source = "G21\nG90\nG00 Z1.0\nG00 X0 Y0\nG00 Z0.0\nG01 X10 Y0\nG01 X10 Y10\nG01 X0 Y10\nG01 X0 Y0\nG00 Z1.0\n";
        let program = parse_gcode(source, MachineMode::Plotter).unwrap();
        // 8 moves: 3 G0 travel, 4 G1 drawing, 1 G0 travel
        assert_eq!(program.segments.len(), 8);
        // 3 layers: Z=1 travel, Z=0 drawing, Z=1 travel
        assert!(program.layers.len() >= 2);
        // first 3 are travel moves (G0 rapid), rest are drawing (G1 with Z<=0)
        assert_eq!(program.segments[0].is_drawing, false);
        assert_eq!(program.segments[1].is_drawing, false);
        assert_eq!(program.segments[2].is_drawing, false);
        assert_eq!(program.segments[3].is_drawing, true);
        assert_eq!(program.segments[4].is_drawing, true);
    }

    #[test]
    fn test_parse_printer_layers() {
        let source = "G21\nG90\nG1 Z0.2 E0.1\nG1 X10 Y0 E0.5\nG1 Z0.4 E0.6\nG1 X0 Y10 E1.0\n";
        let program = parse_gcode(source, MachineMode::Printer3D).unwrap();
        assert_eq!(program.segments.len(), 4);
        assert!(program.layers.len() >= 2);
        // Check E-based extrusion detection
        assert_eq!(program.segments[0].is_drawing, true);  // E 0.1 > 0
        assert_eq!(program.segments[1].is_drawing, true);  // E 0.5 > 0.1
        assert_eq!(program.segments[2].is_drawing, true);  // E 0.6 > 0.5 (new layer)
        assert_eq!(program.segments[3].is_drawing, true);  // E 1.0 > 0.6
    }

    #[test]
    fn test_parse_arc() {
        let source = "G21\nG90\nG00 X0 Y0\nG02 X10 Y0 I5 J0\n";
        let program = parse_gcode(source, MachineMode::Plotter).unwrap();
        // G02 should create ~ π rad / 0.05 ≈ 63 segments
        assert!(program.segments.len() > 10);
        assert_eq!(program.segments[0].is_drawing, false); // G00 travel
    }

    #[test]
    fn test_empty_program() {
        let program = parse_gcode("", MachineMode::Plotter).unwrap();
        assert_eq!(program.segments.len(), 0);
        assert_eq!(program.layers.len(), 0);
    }
}
