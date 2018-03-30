use cgmath::{ self, Deg, Matrix4, Point3, Vector3 };

pub struct Camera {
    pub position: Vector3<f32>,
    pub direction: Vector3<f32>,
    pub fovy: Deg<f32>,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn view(&self) -> Matrix4<f32> {
        let position = Point3::new(self.position.x, self.position.y, self.position.z);
        let direction = {
            let direction = self.position + self.direction;
            Point3::new(direction.x, direction.y, direction.z)
        };

        Matrix4::look_at(
            position,
            direction,
            Vector3::new(0.0, 1.0, 0.0)
        )
    }

    pub fn projection(&self, aspect: f32) -> Matrix4<f32> {
        let proj = cgmath::perspective(self.fovy, aspect, self.near, self.far);
        // https://matthewwellings.com/blog/the-new-vulkan-coordinate-system/
        let vulkan_correction = Matrix4::new(
            1.0,    0.0,    0.0,    0.0,
            0.0,    -1.0,   0.0,    0.0,
            0.0,    0.0,    0.5,    0.0,
            0.0,    0.0,    0.5,    1.0,
        );

        vulkan_correction * proj
    }

    pub fn matrix(&self, aspect: f32) -> Matrix4<f32> {
        self.projection(aspect) * self.view()
    }
}
