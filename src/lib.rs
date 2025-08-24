/*!
Minecraft 皮肤渲染器库

这个库提供了一个完整的 Minecraft 皮肤渲染解决方案，支持：
- 加载和渲染 Minecraft 皮肤
- 自定义姿势和视角
- 多种输出格式
- 离屏渲染

# 示例

```rust
use skinviewer::{Renderer, Character, Camera};

let renderer = Renderer::new();
let mut character = Character::new();
let camera = Camera::new();
// character.load_skin...
renderer.render_to_image(&character, &camera, "output.png", (800, 600));
```
*/

pub mod utils;
pub mod camera;
pub mod character;
pub mod constants;
pub mod model;
pub mod renderer;
pub mod texture;
