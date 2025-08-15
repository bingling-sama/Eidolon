use crate::utils::view_matrix;

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
            yaw: 0.0,
            pitch: 0.0,
            scale: 1.0,
        }
    }

    pub fn get_view_matrix(&self) -> [[f32; 4]; 4] {
        let distance = 4.0 / self.scale;
        let yaw_rad = -self.yaw.to_radians();
        let pitch_rad = (self.pitch - 90.0).to_radians();

        let eye = [
            distance * yaw_rad.sin() * pitch_rad.cos(),
            1.0 + distance * pitch_rad.sin(),
            distance * yaw_rad.cos() * pitch_rad.cos(),
        ];

        let center = [0.0, 1.0, 0.0];
        let up = [0.0, 1.0, 0.0];

        let direction = [center[0] - eye[0], center[1] - eye[1], center[2] - eye[2]];

        view_matrix(&eye, &direction, &up)
    }

    pub fn get_projection_matrix(&self, width: u32, height: u32) -> [[f32; 4]; 4] {
        let aspect_ratio = height as f32 / width as f32;
        let fov: f32 = std::f32::consts::PI / 3.0;
        let zfar = 1024.0;
        let znear = 0.1;
        let f = 1.0 / (fov / 2.0).tan();

        [
            [f * aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
            [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
        ]
    }
}
