use gcode::Value;

#[derive(Clone, Copy, Debug, Default)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub start: Point3D,
    pub end: Point3D,
    pub is_drawing: bool,
    pub layer_index: u32,
    pub segment_index: u32,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Layer {
    pub segment_start: usize,
    pub segment_end: usize,
    pub z_height: f32,
}

#[derive(Clone, Debug)]
pub struct BoundingBox {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl BoundingBox {
    pub fn new() -> Self {
        Self {
            min_x: f32::MAX,
            max_x: f32::MIN,
            min_y: f32::MAX,
            max_y: f32::MIN,
            min_z: f32::MAX,
            max_z: f32::MIN,
        }
    }

    pub fn extend(&mut self, p: &Point3D) {
        self.min_x = self.min_x.min(p.x);
        self.max_x = self.max_x.max(p.x);
        self.min_y = self.min_y.min(p.y);
        self.max_y = self.max_y.max(p.y);
        self.min_z = self.min_z.min(p.z);
        self.max_z = self.max_z.max(p.z);
    }

    pub fn size(&self) -> Point3D {
        Point3D {
            x: (self.max_x - self.min_x).max(1.0),
            y: (self.max_y - self.min_y).max(1.0),
            z: (self.max_z - self.min_z).max(1.0),
        }
    }

    pub fn center(&self) -> Point3D {
        Point3D {
            x: (self.min_x + self.max_x) * 0.5,
            y: (self.min_y + self.max_y) * 0.5,
            z: (self.min_z + self.max_z) * 0.5,
        }
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct GCodeProgram {
    pub segments: Vec<Segment>,
    pub layers: Vec<Layer>,
    pub bounds: BoundingBox,
}

impl Default for GCodeProgram {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            layers: Vec::new(),
            bounds: BoundingBox::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineMode {
    Plotter,
    Printer3D,
}

impl MachineMode {
    pub fn label(&self) -> &'static str {
        match self {
            MachineMode::Plotter => "Pen Plotter",
            MachineMode::Printer3D => "3D Printer",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    View2D,
    View3D,
}

impl ViewMode {
    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::View2D => "2D Top-Down",
            ViewMode::View3D => "3D View",
        }
    }
}

#[allow(dead_code)]
pub fn get_value(val: &Value) -> Option<f32> {
    match val {
        Value::Literal(n) => Some(*n),
        Value::Variable(_) => None,
        _ => None,
    }
}
