pub type Mat4 = [[f32; 4]; 4];
pub type Vec3 = [f32; 3];

pub fn vec3_sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[allow(dead_code)]
pub fn vec3_add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn vec3_scale(v: Vec3, s: f32) -> Vec3 {
    [v[0] * s, v[1] * s, v[2] * s]
}

pub fn vec3_dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn vec3_length(v: Vec3) -> f32 {
    vec3_dot(v, v).sqrt()
}

pub fn vec3_normalize(v: Vec3) -> Vec3 {
    let len = vec3_length(v);
    if len < 1e-10 {
        return [0.0, 0.0, 0.0];
    }
    vec3_scale(v, 1.0 / len)
}

pub fn vec3_cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub const fn mat4_identity() -> Mat4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn mat4_multiply(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut result = [[0.0; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k][row] * b[col][k];
            }
            result[col][row] = sum;
        }
    }
    result
}

pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let f = vec3_normalize(vec3_sub(target, eye));
    let s = vec3_normalize(vec3_cross(f, up));
    let u = vec3_cross(s, f);
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-vec3_dot(s, eye), -vec3_dot(u, eye), vec3_dot(f, eye), 1.0],
    ]
}

pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y_rad * 0.5).tan();
    // WGPU uses [0, 1] NDC depth range (not OpenGL's [-1, 1])
    let range_inv = 1.0 / (near - far);
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far * range_inv, -1.0],
        [0.0, 0.0, near * far * range_inv, 0.0],
    ]
}

pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    // WGPU uses [0, 1] NDC depth range
    [
        [2.0 / (right - left), 0.0, 0.0, 0.0],
        [0.0, 2.0 / (top - bottom), 0.0, 0.0],
        [0.0, 0.0, 1.0 / (near - far), 0.0],
        [
            (left + right) / (left - right),
            (bottom + top) / (bottom - top),
            near / (near - far),
            1.0,
        ],
    ]
}

#[allow(dead_code)]
pub fn rotate_y(angle: f32) -> Mat4 {
    let c = angle.cos();
    let s = angle.sin();
    [
        [c, 0.0, -s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [s, 0.0, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[allow(dead_code)]
pub fn rotate_x(angle: f32) -> Mat4 {
    let c = angle.cos();
    let s = angle.sin();
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, c, s, 0.0],
        [0.0, -s, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[allow(dead_code)]
pub fn translate(x: f32, y: f32, z: f32) -> Mat4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [x, y, z, 1.0],
    ]
}

#[allow(dead_code)]
pub fn scale(s: f32) -> Mat4 {
    [
        [s, 0.0, 0.0, 0.0],
        [0.0, s, 0.0, 0.0],
        [0.0, 0.0, s, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub struct Camera3D {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub target: Vec3,
    initial_target: Vec3,
    initial_distance: f32,
}

impl Camera3D {
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            distance,
            yaw: 0.0,
            pitch: 0.5,
            target,
            initial_target: target,
            initial_distance: distance,
        }
    }

    pub fn reset(&mut self) {
        self.target = self.initial_target;
        self.distance = self.initial_distance;
        self.yaw = 0.0;
        self.pitch = 0.5;
    }

    pub fn center_on(&mut self, target: Vec3) {
        self.target = target;
    }

    pub fn fit(&mut self, target: Vec3, distance: f32) {
        self.target = target;
        self.distance = distance;
        self.yaw = 0.0;
        self.pitch = 0.5;
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let eye_x = self.target[0] + self.distance * self.yaw.cos() * self.pitch.cos();
        let eye_y = self.target[1] + self.distance * self.yaw.sin() * self.pitch.cos();
        let eye_z = self.target[2] + self.distance * self.pitch.sin();
        let eye = [eye_x, eye_y, eye_z];

        let proj = perspective(1.0, aspect, 0.1, self.distance * 5.0);
        let view = look_at(eye, self.target, [0.0, 0.0, 1.0]);
        mat4_multiply(&proj, &view)
    }
}

pub struct Camera2D {
    pub pan: Vec3,
    pub zoom: f32,
}

impl Camera2D {
    pub fn new() -> Self {
        Self {
            pan: [0.0, 0.0, 0.0],
            zoom: 1.0,
        }
    }

    pub fn reset(&mut self) {
        self.pan = [0.0, 0.0, 0.0];
        self.zoom = 1.0;
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let half_w = aspect * 10.0 / self.zoom;
        let half_h = 10.0 / self.zoom;
        let left = -half_w + self.pan[0];
        let right = half_w + self.pan[0];
        let bottom = -half_h + self.pan[1];
        let top = half_h + self.pan[1];
        orthographic(left, right, bottom, top, -1000.0, 1000.0)
    }
}
