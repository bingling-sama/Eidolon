use cgmath::Point3;

use crate::camera::Camera;
use crate::character::Character;

/// A 3D scene containing multiple characters and a shared camera.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub characters: Vec<Character>,
    pub camera: Camera,
}

impl Scene {
    pub fn new(camera: Camera) -> Self {
        Self {
            characters: Vec::new(),
            camera,
        }
    }

    pub fn with_characters(characters: Vec<Character>, camera: Camera) -> Self {
        Self { characters, camera }
    }

    pub fn add_character(&mut self, character: Character) -> usize {
        self.characters.push(character);
        self.characters.len() - 1
    }

    pub fn remove_character(&mut self, index: usize) -> Option<Character> {
        if index < self.characters.len() {
            Some(self.characters.remove(index))
        } else {
            None
        }
    }

    /// Compute the world-space axis-aligned bounding box enclosing all characters in the scene.
    pub fn compute_bounding_box(&self) -> (Point3<f32>, Point3<f32>) {
        if self.characters.is_empty() {
            return (Point3::new(-0.5, 0.0, -0.5), Point3::new(0.5, 2.0, 0.5));
        }

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for character in &self.characters {
            let pos = character.position;
            let height = if character.nametag.is_some() { 2.3 } else { 1.8 };
            let radius = 0.5;

            min_x = min_x.min(pos.x - radius);
            max_x = max_x.max(pos.x + radius);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y + height);
            min_z = min_z.min(pos.z - radius);
            max_z = max_z.max(pos.z + radius);
        }

        (
            Point3::new(min_x, min_y, min_z),
            Point3::new(max_x, max_y, max_z),
        )
    }

    /// Automatically adjust camera zoom/scale to frame all characters.
    pub fn auto_frame_camera(&mut self, fov_deg: f32, aspect_ratio: f32) {
        let (min, max) = self.compute_bounding_box();
        let extent_x = (max.x - min.x) * 0.5;
        let extent_y = (max.y - min.y) * 0.5;
        let extent_z = (max.z - min.z) * 0.5;
        let radius = extent_x.max(extent_y).max(extent_z).max(0.5);

        let half_fovy = (fov_deg.max(1.0) * 0.5).to_radians();
        let half_fovx = (half_fovy.tan() * aspect_ratio.max(0.01)).atan();
        let limiting_half_fov = half_fovy.min(half_fovx);

        let required_dist = radius / limiting_half_fov.sin();
        let base_dist = 4.0;
        let scale = (base_dist / required_dist).clamp(0.1, 10.0);
        self.camera.scale = scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Vector3;

    #[test]
    fn scene_new_and_with_characters() {
        let cam = Camera {
            yaw: 195.0,
            pitch: 80.0,
            scale: 1.2,
        };
        let scene = Scene::new(cam);
        assert_eq!(scene.characters.len(), 0);
        assert_eq!(scene.camera.yaw, 195.0);

        let c1 = Character::empty_for_test();
        let c2 = Character::empty_for_test();
        let scene2 = Scene::with_characters(vec![c1, c2], cam);
        assert_eq!(scene2.characters.len(), 2);
    }

    #[test]
    fn scene_add_and_remove_character() {
        let mut scene = Scene::default();
        let c1 = Character::empty_for_test();
        let idx0 = scene.add_character(c1);
        assert_eq!(idx0, 0);
        assert_eq!(scene.characters.len(), 1);

        let c2 = Character::empty_for_test();
        let idx1 = scene.add_character(c2);
        assert_eq!(idx1, 1);
        assert_eq!(scene.characters.len(), 2);

        let removed = scene.remove_character(0);
        assert!(removed.is_some());
        assert_eq!(scene.characters.len(), 1);

        let out_of_bounds = scene.remove_character(5);
        assert!(out_of_bounds.is_none());
    }

    #[test]
    fn scene_bounding_box_and_auto_frame() {
        let mut scene = Scene::default();
        let (min, max) = scene.compute_bounding_box();
        assert_eq!(min, Point3::new(-0.5, 0.0, -0.5));
        assert_eq!(max, Point3::new(0.5, 2.0, 0.5));

        let mut c1 = Character::empty_for_test();
        c1.position = Vector3::new(-2.0, 0.0, 0.0);
        let mut c2 = Character::empty_for_test();
        c2.position = Vector3::new(2.0, 0.0, 0.0);

        scene.add_character(c1);
        scene.add_character(c2);

        let (min2, max2) = scene.compute_bounding_box();
        assert!(min2.x <= -2.5);
        assert!(max2.x >= 2.5);

        let initial_scale = scene.camera.scale;
        scene.auto_frame_camera(60.0, 16.0 / 9.0);
        assert!(scene.camera.scale < initial_scale);
    }
}
