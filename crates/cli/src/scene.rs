use eidolon_core::{
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, Posture, SkinType},
    error::EidolonError,
    renderer::Renderer,
};

use crate::args::SceneArgs;

#[derive(Debug)]
pub struct SceneState {
    pub skin_type: SkinType,
    pub nametag: Option<String>,
    pub nametag_see_through: bool,
    pub back_equipment_kind: Option<BackEquipmentKind>,
    pub posture: Posture,
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Vector3<f32>,
}

#[derive(Clone, Copy, Default)]
pub struct PostureOverrides {
    pub cape_pitch: Option<f32>,
    pub elytra_pitch: Option<f32>,
    pub elytra_spread: Option<f32>,
    pub head_yaw: Option<f32>,
    pub head_pitch: Option<f32>,
    pub left_arm_roll: Option<f32>,
    pub left_arm_pitch: Option<f32>,
    pub right_arm_roll: Option<f32>,
    pub right_arm_pitch: Option<f32>,
    pub left_leg_pitch: Option<f32>,
    pub right_leg_pitch: Option<f32>,
}

impl From<&SceneArgs> for PostureOverrides {
    fn from(scene: &SceneArgs) -> Self {
        Self {
            cape_pitch: scene.cape_pitch,
            elytra_pitch: scene.elytra_pitch,
            elytra_spread: scene.elytra_spread,
            head_yaw: scene.head_yaw,
            head_pitch: scene.head_pitch,
            left_arm_roll: scene.left_arm_roll,
            left_arm_pitch: scene.left_arm_pitch,
            right_arm_roll: scene.right_arm_roll,
            right_arm_pitch: scene.right_arm_pitch,
            left_leg_pitch: scene.left_leg_pitch,
            right_leg_pitch: scene.right_leg_pitch,
        }
    }
}

impl PostureOverrides {
    pub fn apply(self, base: Posture) -> Posture {
        Posture {
            cape_pitch: self.cape_pitch.unwrap_or(base.cape_pitch),
            body_pitch: base.body_pitch,
            elytra_pitch: self.elytra_pitch.unwrap_or(base.elytra_pitch),
            elytra_yaw: base.elytra_yaw,
            elytra_roll: self.elytra_spread.unwrap_or(base.elytra_roll),
            head_yaw: self.head_yaw.unwrap_or(base.head_yaw),
            head_pitch: self.head_pitch.unwrap_or(base.head_pitch),
            left_arm_roll: self.left_arm_roll.unwrap_or(base.left_arm_roll),
            left_arm_pitch: self.left_arm_pitch.unwrap_or(base.left_arm_pitch),
            right_arm_roll: self.right_arm_roll.unwrap_or(base.right_arm_roll),
            right_arm_pitch: self.right_arm_pitch.unwrap_or(base.right_arm_pitch),
            left_leg_pitch: self.left_leg_pitch.unwrap_or(base.left_leg_pitch),
            right_leg_pitch: self.right_leg_pitch.unwrap_or(base.right_leg_pitch),
        }
    }
}

impl SceneState {
    pub fn apply_to(&self, character: &mut Character) {
        character.nametag = self.nametag.clone();
        character.nametag_see_through = self.nametag_see_through;
        character.posture = self.posture;
        character.position = self.position;
        character.rotation = self.rotation;
    }
}

pub fn apply_posture_overrides(base: Posture, scene: &SceneArgs) -> Posture {
    PostureOverrides::from(scene).apply(base)
}

pub fn scene_state_and_camera_from_scene(scene: &SceneArgs) -> (SceneState, Camera) {
    let base: Posture = scene.posture.into();
    let posture = apply_posture_overrides(base, scene);
    let state = SceneState {
        skin_type: if scene.slim {
            SkinType::Slim
        } else {
            SkinType::Classic
        },
        nametag: scene.nametag.clone(),
        nametag_see_through: !scene.disable_nametag_see_through,
        back_equipment_kind: scene
            .cape
            .as_ref()
            .map(|_| BackEquipmentKind::Cape)
            .or_else(|| scene.elytra.as_ref().map(|_| BackEquipmentKind::Elytra)),
        posture,
        position: cgmath::Vector3::new(scene.pos_x, scene.pos_y, scene.pos_z),
        rotation: cgmath::Vector3::new(scene.rot_x, scene.rot_y, scene.rot_z),
    };

    let camera = Camera {
        yaw: scene.cam_yaw,
        pitch: scene.cam_pitch,
        scale: scene.cam_zoom,
    };
    (state, camera)
}

pub fn load_character(
    renderer: &Renderer,
    skin_path: &str,
    scene: &SceneArgs,
) -> Result<(Character, Camera), EidolonError> {
    let (state, camera) = scene_state_and_camera_from_scene(scene);
    let body = renderer.load_body(skin_path, state.skin_type)?;
    let mut character = Character::from_body(body);
    state.apply_to(&mut character);

    let atlas_path = scene.cape.as_deref().or(scene.elytra.as_deref());
    if let (Some(path), Some(kind)) = (atlas_path, state.back_equipment_kind) {
        let atlas = renderer.load_back_equipment_atlas(path)?;
        character.equip(BackEquipment::new(atlas, kind));
    }

    Ok((character, camera))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::PostureCli;
    use eidolon_core::animation::AnimationPreset;

    fn default_scene() -> SceneArgs {
        SceneArgs {
            slim: false,
            nametag: None,
            disable_nametag_see_through: false,
            cape: None,
            elytra: None,
            cape_pitch: None,
            elytra_pitch: None,
            elytra_spread: None,
            cam_yaw: 180.0,
            cam_pitch: 90.0,
            cam_zoom: 1.0,
            posture: PostureCli::Idle,
            head_yaw: None,
            head_pitch: None,
            left_arm_roll: None,
            left_arm_pitch: None,
            right_arm_roll: None,
            right_arm_pitch: None,
            left_leg_pitch: None,
            right_leg_pitch: None,
            pos_x: 0.0,
            pos_y: 0.0,
            pos_z: 0.0,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
        }
    }

    #[test]
    fn scene_defaults_to_classic_idle() {
        let (state, cam) = scene_state_and_camera_from_scene(&default_scene());
        assert_eq!(state.skin_type, SkinType::Classic);
        assert_eq!(state.posture.head_yaw, 0.0);
        assert_eq!(state.posture.head_pitch, 0.0);
        assert_eq!(cam.yaw, 180.0);
        assert_eq!(cam.pitch, 90.0);
        assert_eq!(cam.scale, 1.0);
    }

    #[test]
    fn scene_slim_skin_type() {
        let mut scene = default_scene();
        scene.slim = true;
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.skin_type, SkinType::Slim);
    }

    #[test]
    fn scene_nametag_is_passed_to_character() {
        let mut scene = default_scene();
        scene.nametag = Some("Steve".to_owned());
        let (state, _) = scene_state_and_camera_from_scene(&scene);

        assert_eq!(state.nametag.as_deref(), Some("Steve"));
    }

    #[test]
    fn scene_can_disable_nametag_see_through() {
        let default_state = scene_state_and_camera_from_scene(&default_scene()).0;
        assert!(default_state.nametag_see_through);

        let mut scene = default_scene();
        scene.disable_nametag_see_through = true;
        let state = scene_state_and_camera_from_scene(&scene).0;
        assert!(!state.nametag_see_through);
    }

    #[test]
    fn scene_cape_path_enables_cape_rendering() {
        let mut scene = default_scene();
        scene.cape = Some("cape.png".to_owned());
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.back_equipment_kind, Some(BackEquipmentKind::Cape));
    }

    #[test]
    fn scene_elytra_path_enables_elytra_rendering() {
        let mut scene = default_scene();
        scene.elytra = Some("elytra.png".to_owned());
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.back_equipment_kind, Some(BackEquipmentKind::Elytra));
    }

    #[test]
    fn scene_camera_values_passed_through() {
        let mut scene = default_scene();
        scene.cam_yaw = 45.0;
        scene.cam_pitch = 30.0;
        scene.cam_zoom = 2.5;
        let (_, cam) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(cam.yaw, 45.0);
        assert_eq!(cam.pitch, 30.0);
        assert_eq!(cam.scale, 2.5);
    }

    #[test]
    fn scene_posture_wave_overrides_base() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Wave;
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert!(
            state.posture.left_arm_pitch > 0.0,
            "Wave posture: left arm raised"
        );
        assert_eq!(
            state.posture.right_arm_pitch, 0.0,
            "Wave posture: right arm still"
        );
        assert_eq!(state.posture, PostureCli::Wave.into());
    }

    #[test]
    fn scene_run_uses_representative_frame() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Run;
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture, PostureCli::Run.into());
    }

    #[test]
    fn scene_cape_pitch_overrides_posture() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Run;
        scene.cape_pitch = Some(25.0);
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture.cape_pitch, 25.0);
    }

    #[test]
    fn scene_joint_override_overrides_posture() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Idle;
        scene.head_yaw = Some(42.0);
        scene.left_leg_pitch = Some(-15.0);
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture.head_yaw, 42.0);
        assert_eq!(state.posture.left_leg_pitch, -15.0);
        assert_eq!(state.posture.head_pitch, 0.0); // not overridden
    }

    #[test]
    fn animation_sample_is_modified_only_by_explicit_overrides() {
        let mut scene = default_scene();
        scene.head_yaw = Some(42.0);
        scene.elytra_spread = Some(33.0);
        let sampled = AnimationPreset::Walk.sample(AnimationPreset::Walk.cycle_duration() / 4);

        let posture = apply_posture_overrides(sampled, &scene);

        assert_eq!(posture.head_yaw, 42.0);
        assert_eq!(posture.elytra_roll, 33.0);
        assert_eq!(posture.left_leg_pitch, sampled.left_leg_pitch);
        assert_eq!(posture.right_arm_pitch, sampled.right_arm_pitch);
    }

    #[test]
    fn scene_world_position_and_rotation() {
        let mut scene = default_scene();
        scene.pos_x = 1.0;
        scene.pos_y = 2.0;
        scene.pos_z = 3.0;
        scene.rot_x = 10.0;
        scene.rot_y = 20.0;
        scene.rot_z = 30.0;
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.position.x, 1.0);
        assert_eq!(state.position.y, 2.0);
        assert_eq!(state.position.z, 3.0);
        assert_eq!(state.rotation.x, 10.0);
        assert_eq!(state.rotation.y, 20.0);
        assert_eq!(state.rotation.z, 30.0);
    }
}
