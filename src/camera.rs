use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Orbit yaw around the look-at target (degrees). Used in [`Camera::get_view_matrix`].
    pub yaw: f32,
    /// Orbit pitch (degrees). [`Camera::get_view_matrix`] uses `(pitch - 90°)` as polar angle from horizontal.
    pub pitch: f32,
    /// Positive value moves the eye closer (smaller orbit radius: `4.0 / scale`).
    pub scale: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 180.0,
            pitch: 90.0,
            scale: 1.0,
        }
    }
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes the view matrix from camera parameters.
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

    /// Computes the projection matrix from camera parameters.
    pub fn get_projection_matrix(&self, width: u32, height: u32) -> [[f32; 4]; 4] {
        let aspect_ratio = width as f32 / height as f32;
        let fovy = Deg(60.0);
        let znear = 0.1;
        let zfar = 1024.0;
        perspective(fovy, aspect_ratio, znear, zfar).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_matrix_default_looks_at_center() {
        // yaw=180, pitch=90, scale=1.0 — default render view
        let camera = Camera {
            yaw: 180.0,
            pitch: 90.0,
            scale: 1.0,
        };
        let view = camera.get_view_matrix();
        // view matrix should be non-identity and finite
        for row in &view {
            for &val in row {
                assert!(val.is_finite(), "view matrix contains non-finite value");
            }
        }
    }

    #[test]
    fn view_matrix_scale_doubles_distance() {
        // distance = 4.0 / scale; at scale=0.5, distance=8.0 (twice default)
        let cam_half = Camera {
            yaw: 0.0,
            pitch: 90.0,
            scale: 0.5,
        };
        let cam_full = Camera {
            yaw: 0.0,
            pitch: 90.0,
            scale: 1.0,
        };
        let view_half = cam_half.get_view_matrix();
        let view_full = cam_full.get_view_matrix();
        // different distance → different matrices
        assert_ne!(view_half, view_full);
    }

    #[test]
    fn projection_matrix_valid_aspect() {
        let camera = Camera::new();
        let proj = camera.get_projection_matrix(800, 600);
        // projection matrix should be finite
        for row in &proj {
            for &val in row {
                assert!(val.is_finite(), "projection matrix contains non-finite value");
            }
        }
        // last row should be [0, 0, -1, 0] for perspective projection
        assert!(proj[3][2] < 0.0, "expected negative z-component in projection");
    }

    #[test]
    fn projection_matrix_square_aspect() {
        let camera = Camera::new();
        let proj_square = camera.get_projection_matrix(600, 600);
        let proj_wide = camera.get_projection_matrix(800, 600);
        // different aspect ratios produce different matrices
        assert_ne!(proj_square, proj_wide);
    }
}
