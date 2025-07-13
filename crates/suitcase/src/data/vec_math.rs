use cgmath::Vector3;

pub trait ToGlow<T> where T: Copy {
    fn to_glow(&self) -> [T; 3];
}

impl<T> ToGlow<T> for Vector3<T> where T: Copy {
    fn to_glow(&self) -> [T; 3] {
        [self.x, self.y, self.z]
    }
}
