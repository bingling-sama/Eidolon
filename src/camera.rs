use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// 摄像机视角绕角色旋转角度（XZ 平面绕 Y 轴旋转），0~360，0 是正前，90 是正右，180 是正后，270 是正左
    pub yaw: f32,
    /// 摄像机视角绕角色俯仰角度（YZ 平面绕 X 轴旋转），0~180，90 是正前，0 是脚下，180 是头顶
    pub pitch: f32,
    /// 缩放比例，0~1
    pub scale: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            yaw: 210.0,
            pitch: 75.0,
            scale: 1.0,
        }
    }

    pub fn get_view_matrix(&self) -> [[f32; 4]; 4] {
        let distance = 4.0 / self.scale;
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = (self.pitch - 90.0).to_radians();

        let eye_x = distance * yaw_rad.sin() * pitch_rad.cos();
        let eye_y = 1.0 + distance * pitch_rad.sin();
        let eye_z = distance * yaw_rad.cos() * pitch_rad.cos();

        let eye = Point3::new(eye_x, eye_y, eye_z);
        let center = Point3::new(0.0, 1.0, 0.0);
        let up = Vector3::new(0.0, 1.0, 0.0);

        Matrix4::look_at_rh(eye, center, up).into()
    }

    pub fn get_projection_matrix(&self, width: u32, height: u32) -> [[f32; 4]; 4] {
        let aspect_ratio = width as f32 / height as f32;
        let fovy = Deg(60.0);
        let znear = 0.1;
        let zfar = 1024.0;
        perspective(fovy, aspect_ratio, znear, zfar).into()
    }
}
