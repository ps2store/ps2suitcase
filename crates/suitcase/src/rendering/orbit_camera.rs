use cgmath::{point3, vec3, EuclideanSpace, Matrix4, Point3, Vector3};

#[derive(Clone, Copy, Debug)]
pub struct OrbitCamera {
    pub target: Vector3<f32>,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl OrbitCamera {
    pub fn update(&mut self, delta_yaw: f32, delta_pitch: f32, delta_zoom: f32) {
        self.yaw += delta_yaw;
        self.pitch += delta_pitch;

        // Clamp pitch to avoid gimbal lock
        self.pitch = self.pitch.clamp(-1.54, 1.54); // limit to ~88 degrees
        self.distance = (self.distance - delta_zoom).clamp(self.min_distance, self.max_distance);
    }

    pub fn position(&self) -> Vector3<f32> {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        Vector3::new(x, y, z) + self.target
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        let position = self.position();
        Matrix4::look_at_rh(
            point3(position.x, position.y, position.z),
            Point3::from_vec(self.target),
            vec3(0.0, 1.0, 0.0),
        )
    }

    pub fn reset_view(&mut self) {
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.distance = 10.0;
    }
}