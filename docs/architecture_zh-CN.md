# 架构说明

Eidolon 由一个命令行程序和一个可复用渲染库组成。命令行负责解析场景参数并调用库；库负责
GPU 初始化、皮肤加载、模型加载与图片输出。

## Crate 结构

- `src/main.rs` 定义 `eidolon` 可执行程序，以及 `render`、`preview`、`convert` 子命令。
- `src/lib.rs` 暴露库模块。
- `src/camera.rs` 为轨道相机计算 view 与 projection 矩阵。
- `src/character.rs` 定义皮肤几何类型、姿势预设和角色变换。
- `src/model.rs` 从 `resources/` 加载 classic 与 slim OBJ 模型。
- `src/texture.rs` 加载 PNG 皮肤，并在需要时自动展开旧版单层皮肤。
- `src/renderer/` 包含共享的 `wgpu` 渲染器、输出编码、读回、管线和 uniform 逻辑。
- `src/utils/converter.rs` 实现单层皮肤图集到双层图集的转换。
- `benches/performance_benchmark.rs` 使用 Criterion 批量渲染图片做基准测试。

## 渲染流程

执行 `cargo run -- render ...` 时，流程如下：

1. `src/main.rs` 将命令行参数解析为 `SceneArgs`。
2. `character_and_camera_from_scene` 构造 `Character` 和 `Camera`。
3. `Renderer::new` 创建无窗口 `wgpu` device、queue、渲染管线，并加载两套 OBJ 模型。
4. `Renderer::load_texture` 读取 PNG 皮肤。如果图片满足 `width == height * 2`，会先转换为
   正方形双层图集再上传到 GPU。
5. `Renderer::render_to_image` 渲染到离屏 `Rgba8Unorm` 纹理，将带 padding 的 GPU 行数据复制
   到 CPU buffer，去除 padding 后保存为 PNG 或 WebP。

`preview` 使用同一套场景数据和渲染管线，但 `Renderer::new_windowed` 会创建窗口 surface，
`render_frame` 每帧提交到 swapchain。

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

## 当前限制

- 皮肤纹理仅支持从 PNG 文件加载。
- `Character::cape` 和 `Character::nametag` 是预留字段，当前不会渲染。
- 渲染器需要可用的 `wgpu` 后端。无窗口渲染同样需要创建 GPU adapter。
