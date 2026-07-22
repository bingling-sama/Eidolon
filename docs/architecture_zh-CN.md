# 架构说明

Eidolon 由一个命令行程序和一个可复用渲染库组成。命令行负责解析场景参数并调用库完成单帧
渲染。库负责 GPU 初始化、皮肤加载、模型加载、单帧渲染与输出策略。

## Crate 结构

- `src/main.rs` 定义 `eidolon` 可执行程序，以及 `render`、`preview`、`convert` 子命令。
- `src/animation.rs` 提供无状态 `AnimationPreset` 采样、有状态 `AnimationTimeline` 播放，
  以及确定性的 `AnimationSchedule` 帧时刻。
- `src/wasm.rs` 是可选的 `wasm` feature adapter；目标为 `wasm32` 时通过 `wasm-bindgen` 暴露
  `CanvasRenderer`。
- `src/output.rs` 负责输出意图、路径、格式校验、动画 timing、编码和文件写入。
- `src/lib.rs` 暴露库模块。
- `src/camera.rs` 为轨道相机计算 view 与 projection 矩阵。
- `src/body.rs` 定义持有 Skin 与 classic/slim 几何选择的 `Body`。
- `src/back_equipment.rs` 定义披风与鞘翅共享的后背装备图集及呈现形式。
- 私有的 `src/cape.rs` 和 `src/elytra.rs` 使用 skinview3d 图集映射构建 Renderer 持有的网格。
- `src/character.rs` 定义资源所有权、姿势预设和角色变换；Character 始终至少包含 Body 或后背装备。
- `src/model.rs` 从 `resources/` 加载 classic 与 slim OBJ 模型。
- 私有的 `src/texture.rs` 上传 PNG 图集，并在需要时自动展开旧版单层 Skin。
- `src/renderer/` 包含共享的 `wgpu` 渲染器、nametag 光栅化与 billboard 变换、读回、管线和 uniform 逻辑。
- `src/converter.rs` 实现单层皮肤图集到双层图集的转换。
- `benches/performance_benchmark.rs` 使用 Criterion 测量批量渲染和单周期动画导出。

## 渲染流程

执行 `cargo run -- render ...` 时，流程如下：

1. `src/main.rs` 将命令行参数解析为 `SceneArgs`。
2. `scene_state_and_camera_from_scene` 解析与资源无关的场景状态和 `Camera`。
3. `Renderer::new` 创建无窗口 `wgpu` device、queue、渲染管线，并加载两套 OBJ 模型。
4. `Renderer::load_body` 读取 PNG Skin，必要时转换旧版图集，上传后返回绑定当前 Renderer
   上下文的 `Body`。CLI 再构造 Character，并可选加载共享后背装备图集作为披风或鞘翅。
5. `output::write_output` 解析输出路径和意图，向 Renderer 请求一帧或多帧 RGBA 数据，
   然后写入 PNG、WebP、GIF 或 Animated WebP。扩展名调整会保留父目录，并返回最终写入路径。

每个离屏帧由 Renderer 创建 render target 并提交 render pass。私有的 `renderer::readback`
模块随后执行第二次有序提交：将完成的 texture 复制到 mapped buffer、移除行 padding，并返回
RGBA 像素。native 为 `Renderer::render` 同步等待 map，wasm 为 `Renderer::render_async`
异步等待；两个 adapter 共用相同的 layout 校验和去 padding 实现。

设置 `Character::nametag` 后，Renderer 会将单行文本光栅化为 GPU 资源并存入该 Character；
克隆 Character 时通过 `Arc` 共享缓存资源。之后通过不受光照影响的 billboard 管线绘制。锚点先变换玩家碰撞箱并计算场景空间 AABB，再放在 AABB 水平中心的最高点
上方。碰撞箱高度为 1.8 个世界单位，默认间距为 0.5 个单位，二者都会随 `Camera::scale` 缩放。
默认先绘制 25% 透明度且忽略深度的遮挡层，再绘制正常深度测试层；
`nametag_see_through = false` 会跳过第一层。

`preview` 使用同一套场景数据和渲染管线，但 `Renderer::new_windowed` 会创建窗口 surface，
`render_frame` 每帧提交到 swapchain。

CLI 使用 `--posture` 作为统一动作选择器。静态渲染采用对应 `AnimationPreset` 的代表帧；
preview 初始暂停在同一帧，开始播放后通过 `AnimationTimeline` 继续该 preset。
`render --animate` 使用 `AnimationSchedule` 在自然周期内均匀采样，在 frame producer 中应用
显式 Posture 覆盖，逐帧渲染并交给 `output::write_output`。GIF 延迟和 WebP 时间戳仍属于
output adapter，并使用累计取整避免漂移；Renderer 仍然只负责单帧渲染。

## 模型与皮肤约定

内置 OBJ 文件需要包含以下对象名：

- `Head`、`Hat Layer`
- `Body`、`Body Layer`
- `Right Arm`、`Right Arm Layer`
- `Left Arm`、`Left Arm Layer`
- `Right Leg`、`Right Leg Layer`
- `Left Leg`、`Left Leg Layer`

`SkinType::Classic` 使用 `resources/classic.obj`；`SkinType::Slim` 使用 `resources/slim.obj`。
纹理使用 nearest 采样，以保持 Minecraft 皮肤像素边缘清晰。

## 坐标与角度说明

- 相机 yaw 和 pitch 使用角度制。
- `Camera::scale` 是正数缩放值。数值越大，相机越靠近；当前 uniform 路径中也会缩放模型。
- 角色旋转使用 Euler 角，顺序为 X、Y、Z。
- 姿势字段使用角度制。部分字段按中立 Minecraft 姿势解释，例如头部和腿部 pitch 在渲染器中会
  以 `90` 度为基准偏移。
- `body_pitch` 会让整个角色绕模型中心旋转。两片鞘翅共享中央上背连接点 `(0, 1.5, 0.125)`；两翼共用 pitch，yaw 和 roll 描述左翼，右翼取镜像值。局部 XYZ 欧拉角按上游的 `Rz × Ry × Rx` 矩阵顺序组合。`FLYING` 使用 `20°` pitch 和 `60°` 开合角。

## 当前限制

- Skin 与后背装备图集可从 PNG 路径或 PNG 字节加载。
- 披风与鞘翅共享一个 2:1 后背装备图集（64×32 或等比例高清图集），由
  `BackEquipmentKind` 选择呈现形式；两者都可以脱离 Body 独立渲染。
- Character 的 GPU 资源绑定创建它们的 Renderer。克隆通过 `Arc` 共享不可变 GPU 资源；
  另一个 Renderer 必须重新加载资源。
- nametag 仅支持单行，最多渲染 64 个 Unicode 字符。
- 渲染器需要可用的 `wgpu` 后端。无窗口渲染同样需要创建 GPU adapter。
