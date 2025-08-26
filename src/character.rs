use crate::texture::Texture;
use clap::ValueEnum;
use glium::backend::glutin::headless::Headless;
use cgmath::Vector3;

/// Minecraft 皮肤类型
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SkinType {
    /// 默认皮肤类型（Steve 样式）
    Classic,
    /// 细手臂皮肤类型（Alex 样式）
    Slim,
}

/// Minecraft 角色姿势
///
/// 定义了角色的各个身体部位的旋转角度
#[derive(Debug, Clone, Copy)]
pub struct Posture {
    /// 角色头部摇头角度（XZ 平面绕 Y 轴旋转），0~180，90 是正前，0 是正左，180 是正右
    pub head_yaw: f32,
    /// 角色头部俯仰角度（YZ 平面绕 X 轴旋转），0~180，90 是正前，0 是垂直向下看，180 是垂直向上看
    pub head_pitch: f32,
    /// 左手侧举角度（XY 平面绕 Z 轴旋转），0~180，90 是向右侧平举，0 是垂直向下，180 是垂直向上抬起
    pub left_arm_roll: f32,
    /// 左手摆臂角度（YZ 平面绕 X 轴旋转），0~360，0 是垂直向下，90 是水平前摆，180 是垂直向上，270 是水平向后
    pub left_arm_pitch: f32,
    /// 右手侧举角度（XY 平面绕 Z 轴旋转），0~180，90 是向右侧平举，0 是垂直向下，180 是垂直向上抬起
    pub right_arm_roll: f32,
    /// 右手摆臂角度（YZ 平面绕 X 轴旋转），0~360，0 是垂直向下，90 是水平前摆，180 是垂直向上，270 是水平向后
    pub right_arm_pitch: f32,
    /// 左腿抬腿角度（YZ 平面绕 X 轴旋转），0~180，90 是垂直于地面，0 是水平前摆，180 是水平后摆
    pub left_leg_pitch: f32,
    /// 右腿抬腿角度（YZ 平面绕 X 轴旋转），0~180，90 是垂直于地面，0 是水平前摆，180 是水平后摆
    pub right_leg_pitch: f32,
}

/// 预定义的默认姿势
pub struct DefaultPostures;

impl DefaultPostures {
    /// 站立姿势 - 所有角度为 0
    pub const STAND: Posture = Posture {
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 0.0,
        left_arm_pitch: 0.0,
        right_arm_roll: 0.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 0.0,
        right_leg_pitch: 0.0,
    };
}

/// Represents a Minecraft character
pub struct Character {
    /// 皮肤纹理，可选是因为可以只渲染披风
    pub skin: Option<Texture>,
    /// 皮肤类型，如果给了皮肤还是 None 的话就自动判断皮肤类型
    pub skin_type: SkinType,
    /// 披风文件，可选是因为可以只渲染皮肤
    pub cape: Option<Vec<u8>>,
    /// 名称标签，None 就是不加 NameTag
    pub nametag: Option<String>,
    /// 当前姿势
    pub posture: Posture,
    /// 角色在世界中的位置
    pub position: Vector3<f32>,
}

impl Character {
    pub fn new() -> Self {
        Self {
            skin: None,
            skin_type: SkinType::Classic,
            cape: None,
            nametag: None,
            posture: DefaultPostures::STAND,
            position: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn load_skin_from_file(
        &mut self,
        path: &str,
        display: &Headless,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.skin = Some(Texture::load_from_file(display, path)?);
        Ok(())
    }
}
