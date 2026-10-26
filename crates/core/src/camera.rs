use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Orbit yaw around the look-at target (degrees). Used in [`Camera::get_view_matrix`].
    pub yaw: f32,
    /// Orbit pitch (degrees). [`Camera::get_view_matrix`] uses `(pitch - 90°)` as polar angle from horizontal.
    pub pitch: f32,
    /// Magnification. Implemented as a lens zoom: the orbit radius stays fixed and the field
    /// of view narrows, so a closer look does not exaggerate perspective.
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

/// Orbit radius used by every view. Zoom is a lens zoom, not a dolly.
const ORBIT_RADIUS: f32 = 4.0;
/// Field of view at `scale == 1.0` (degrees).
const DEFAULT_FOVY_DEG: f32 = 60.0;
/// Field of view stays within [MIN_FOVY, MAX_FOVY] so extreme zooms cannot produce a
/// degenerate (or mirroring) projection.
const MIN_FOVY_DEG: f32 = 1.0;
const MAX_FOVY_DEG: f32 = 120.0;

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes the view matrix from camera parameters.
    pub fn get_view_matrix(&self) -> [[f32; 4]; 4] {
        let distance = ORBIT_RADIUS;
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
        // Lens zoom: keep the orbit radius and narrow the field of view instead.
        let half_angle = ((DEFAULT_FOVY_DEG * 0.5).to_radians().tan() / self.scale).atan();
        let half_angle = half_angle.clamp(
            (MIN_FOVY_DEG * 0.5).to_radians(),
            (MAX_FOVY_DEG * 0.5).to_radians(),
        );
        let fovy = Deg(2.0 * half_angle.to_degrees());
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
    fn zoom_keeps_the_field_of_view_within_bounds() {
        let fovy = |scale: f32| {
            let proj = Camera {
                yaw: 0.0,
                pitch: 90.0,
                scale,
            }
            .get_projection_matrix(800, 600);
            2.0 * (1.0 / proj[1][1]).atan().to_degrees()
        };

        assert!(
            (fovy(1.0) - 60.0).abs() < 0.01,
            "default fovy: {}",
            fovy(1.0)
        );
        assert!(fovy(2.0) < fovy(1.0));
        assert!(
            (fovy(10_000.0) - 1.0).abs() < 0.01,
            "max zoom: {}",
            fovy(10_000.0)
        );
        assert!(
            (fovy(0.0001) - 120.0).abs() < 0.01,
            "min zoom: {}",
            fovy(0.0001)
        );
    }

    #[test]
    fn zoom_narrows_the_lens_without_moving_the_camera() {
        let zoomed = Camera {
            yaw: 0.0,
            pitch: 90.0,
            scale: 2.0,
        };
        let wide = Camera {
            yaw: 0.0,
            pitch: 90.0,
            scale: 1.0,
        };
        // A lens zoom keeps the orbit radius fixed, so the view matrix is unchanged...
        assert_eq!(zoomed.get_view_matrix(), wide.get_view_matrix());
        // ...and only the projection narrows.
        assert_ne!(
            zoomed.get_projection_matrix(800, 600),
            wide.get_projection_matrix(800, 600)
        );
    }

    #[test]
    fn projection_matrix_valid_aspect() {
        let camera = Camera::new();
        let proj = camera.get_projection_matrix(800, 600);
        // projection matrix should be finite
        for row in &proj {
            for &val in row {
                assert!(
                    val.is_finite(),
                    "projection matrix contains non-finite value"
                );
            }
        }
        // last row should be [0, 0, -1, 0] for perspective projection
        assert!(
            proj[3][2] < 0.0,
            "expected negative z-component in projection"
        );
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
