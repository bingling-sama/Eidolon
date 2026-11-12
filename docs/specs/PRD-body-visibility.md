# Issue #16：Body 部件与双层显隐控制规格

- 状态：Draft / 待评审，可据此拆分实现任务
- 来源：<https://github.com/bingling-sama/Eidolon/issues/16>
- 编写日期：2026-11-13
- 代码基线：`e8431ed3930a5fbb59791d07c8782bfd370fcaaa`
- 范围：`eidolon-core`、`eidolon-cli`、`eidolon-wasm`、场景配置、测试与使用文档
- 本文中的“必须”为验收要求；“建议”为实现选择。接口代码均为待实现契约，不表示当前已经存在。

## 1. 摘要

为每个 Character 提供独立的 Body 显隐状态，控制头、躯干、左右手臂、左右腿各自的内外两层，共 12 个布尔值。Rust、CLI 和 WASM 使用相同语义；默认全部显示，保持现有渲染结果。

开关通过跳过对应网格的 draw 实现，不修改 Skin，不重建网格，不重新上传 GPU 资源。显隐不等于卸载 Body，也不控制 Back equipment 或名字牌。

## 2. 责任与评审

| 责任 | 需要确认的内容 | 负责人 |
| --- | --- | --- |
| 功能维护者 | 范围、命名、批量操作语义、CLI 配置规则 | 待指定 |
| Core 实现者 | 状态模型、绘制入口、资源与索引不变量 | 待指定 |
| 适配层实现者 | CLI、场景配置、WASM 生命周期一致性 | 待指定 |
| 测试评审者 | 像素测试有效性、浏览器覆盖、性能对比 | 待指定 |

本文对 issue 未定义的边界给出明确的建议决策，评审可修改；不将建议视为已经获得维护者批准的历史决策。

## 3. 背景与现状

### 3.1 用户问题

- 生成头像时，只需要头，不需要躯干和四肢。
- 检查皮肤底图时，需要关闭帽子、夹克、袖子和裤腿外层。
- 检查外层设计时，需要仅显示外层。
- 多角色场景中，需要对不同角色使用不同显隐组合。
- 调整显隐时，应保留皮肤、动作、装备等已有状态，不应重新加载资源。

目标是补齐 skinview3d 对身体六部件与内外层的显隐能力，不承诺兼容其对象结构或 API 命名。

### 3.2 当前代码事实

| 位置 | 当前职责及与本功能的关系 |
| --- | --- |
| `crates/core/src/body.rs` | Body 持有 Skin、SkinType、Renderer 归属；克隆共享不可变 GPU 资源 |
| `crates/core/src/character.rs` | Character 持有可选 Body、Back equipment、Posture 和场景状态；禁止移除最后一项视觉资源 |
| `crates/core/src/renderer/uniforms.rs` | 私有 `PartId` 和 `PART_CONFIGS` 定义六部件映射与固定 uniform 槽位 |
| `crates/core/src/renderer/mod.rs` | `encode_render_pass()` 内无条件绘制每个 Body 部件的 main 和 layer；单角色、场景、离屏与窗口输出共用此入口 |
| `crates/core/src/scene.rs` | 取景使用角色级近似包围盒，不是逐网格的可见包围盒 |
| `crates/cli/src/scene.rs` | 将 CLI 参数或 JSON/TOML 配置转换为 Character；`SceneState` 也用于 preview 初始化 |
| `crates/wasm/src/renderer.rs` | 同时提供单角色 API 和基于 ID 的场景 API；替换皮肤会创建并 attach 新 Body |
| `schemas/scene.json` | 提供 JSON 场景配置的编辑器提示 |

issue 中引用的旧版 `Renderer::render()` 行号不再是准确修改位置；必须修改当前共用绘制入口，不能只改某个输出封装。

仓库当前工作树没有 `.trellis/`，因此本文存入 `docs/specs/`，不新建或猜测 Trellis 任务结构。

## 4. 目标、范围与成功标准

### 4.1 必须交付

1. 六部件 × 两层的独立控制，支持 classic 与 slim。
2. Rust 显隐数据类型、批量方法与 Character 状态。
3. 共用渲染路径的条件绘制。
4. WASM 单角色与场景实体 API。
5. CLI `render`、`preview` 的三个隐藏参数。
6. JSON/TOML 场景配置中的逐角色显隐字段及 JSON Schema。
7. 单元、渲染集成、CLI、浏览器测试以及性能对比记录。
8. 中英文使用文档和示例。

### 4.2 非目标

- Cape、Elytra、名字牌的新增显隐 API；沿用其现有接口。
- 身体部件层级树、父子继承、透明度滑块、淡入淡出、显隐关键帧。
- 逐面、逐像素、骨骼或材质级显隐。
- 裁剪 Skin、修改 atlas 内容、改变旧版皮肤转换规则。
- 根据可见部件自动裁切图片或重新取景；“只显示头”不等于“自动生成紧凑头像”。
- 重构透明混合、背面剔除、OBJ 资源、uniform 分配或场景分块方案。
- WASM Studio 的完整显隐编辑面板；本期浏览器验收可使用专用测试页面。

### 4.3 成功标准

- 下文验收矩阵中的必测项全部通过。
- 未使用新功能时，在同一渲染环境下 RGBA 输出与基线一致。
- setter 只修改 CPU 值，不引入 GPU 资源操作；全隐藏的 Body 发出零次身体 draw。
- 提交前提供相同硬件、后端、构建条件下的前后性能记录；不得仅以“代码很轻”声称零回归。

## 5. 用户与使用场景

| 用户 | 要完成的工作 | 主要入口 |
| --- | --- | --- |
| Rust 集成开发者 | 为每个角色设置局部外观、批量生成图片 | Core |
| CLI 使用者 | 无外层输出、仅头输出、预览结果 | render / preview |
| Web 集成开发者 | 实时切换图层，分别控制场景角色 | CanvasRenderer |
| 皮肤作者 | 对照检查底层和外层，不丢失正在使用的皮肤 | 任一入口 |

共同约束：开关可重复调用；切换不重置姿势、动画进度、位置、旋转、相机、SkinType 或装备。

## 6. 价值与交互原则

- 最小控制单位是一个部件的一层；不必为常见需求改贴图或创建专用 OBJ。
- 批量操作与局部操作写入同一组状态，不设置第二套隐式遮罩。
- 同样的状态在离屏图片、实时预览、浏览器画布和浏览器导出中产生一致的身体绘制选择。
- 保持既有行为：没有新字段、参数或调用时，所有部件的两层都显示。

## 7. 解决方案与详细契约

### 7.1 术语和部件映射

`inner` 对应现有 `BodyPart.main`；`outer` 对应现有 `BodyPart.layer`。

| 部件 | Rust `BodyPartId` | Rust/配置字段 | WASM 字符串 | CLI 值 |
| --- | --- | --- | --- | --- |
| 头 | `Head` | `head` | `head` | `head` |
| 躯干 | `Body` | `body` | `body` | `body` |
| 右臂 | `RightArm` | `right_arm` | `rightArm` | `right-arm` |
| 左臂 | `LeftArm` | `left_arm` | `leftArm` | `left-arm` |
| 右腿 | `RightLeg` | `right_leg` | `rightLeg` | `right-leg` |
| 左腿 | `LeftLeg` | `left_leg` | `leftLeg` | `left-leg` |

左右始终是 Character 自身的左右，不随相机方向变化。躯干对外统一叫 `body`，本期不添加 `torso` 别名。`body_pitch` 仍表示整个 Character 的倾转，与躯干部件开关无关。

### 7.2 唯一状态与操作语义

每个 Character 保存一个 `BodyVisibility`，其中恰好有六个 `PartVisibility`，每个只有 `inner`、`outer` 两个值。不存在额外的 `part.visible` 或持久化的 `global.inner_layer` / `global.outer_layer`。

| 操作 | 精确效果 |
| --- | --- |
| 设置部件两层 | 仅替换该部件的 `inner` 和 `outer` |
| 显示/隐藏整个部件 | 将该部件的两个值同时设为 true/false |
| `set_inner_layers(v)` | 将六部件的 `inner` 改为 v，不改 `outer` |
| `set_outer_layers(v)` | 将六部件的 `outer` 改为 v，不改 `inner` |
| `set_all(v)` | 将全部 12 个值改为 v |
| 恢复默认 | 等价于 `set_all(true)` |

**调用顺序就是覆盖顺序。** 例如先隐藏全部外层，再将头设为 `(true, true)`，头的外层必须显示。再调用 `set_outer_layers(true)` 会打开全部外层，包括此前隐藏的部件外层；批量方法不记忆旧状态、不提供自动恢复。

部件的绘制真值表：

| inner | outer | 结果 |
| --- | --- | --- |
| true | true | 绘制 main，再绘制 layer |
| true | false | 仅 main |
| false | true | 仅 layer；允许壳状外观，不补画 main |
| false | false | 不绘制该部件，不写颜色或深度 |

### 7.3 状态归属和生命周期

**建议决策：放在 Character，字段为 `pub body_visibility: BodyVisibility`，不放在 Body 或 Renderer。**

理由：显隐是角色展示状态；共享 Skin 的角色必须可以不同显隐；皮肤替换不应导致设置丢失。Character 的可选 Body 也允许“先配置、后装载”。

- `Character::from_body()` 和 `from_back_equipment()` 默认全部显示；无 Body 时状态暂存但不产生身体 draw。
- `attach_body()`、`remove_body()`、切换 SkinType 不改变显隐状态。
- `Character::clone()` 复制显隐值；修改克隆不影响原角色，GPU 资源仍按既有方式共享。
- 把一个 Body 从角色 A 转到角色 B 时，使用 B 的显隐状态，不携带 A 的设置。
- Body 全隐藏仍是已挂载资源，不违反 Character 非空约束，不自动移除 Character。
- 隐藏 `body` 躯干不隐藏其余部件；不存在父部件显隐继承。
- Body 隐藏后，装备和名字牌仍按原规则绘制；它们可能因失去身体遮挡而露出，这是预期效果。
- Posture 与 Animation 只控制变换，不读写显隐；重新显示部件使用当前动作变换。
- 切换显隐不更新场景包围盒或相机。维持现有角色级近似取景，包括全隐藏角色。

### 7.4 Rust 公共 API

建议在 `crates/core/src/visibility.rs` 集中定义；通过 `pub mod visibility` 暴露，不引入 serde 或 bitflags 依赖。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyPartId {
    Head,
    Body,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartVisibility {
    pub inner: bool,
    pub outer: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyVisibility {
    pub head: PartVisibility,
    pub body: PartVisibility,
    pub right_arm: PartVisibility,
    pub left_arm: PartVisibility,
    pub right_leg: PartVisibility,
    pub left_leg: PartVisibility,
}
```

必须手工实现 `PartVisibility::default()` 为两个 true；`BodyVisibility` 的 Default 可组合六个 PartVisibility 的 Default。不能对布尔值直接派生出全 false 的默认状态。

必须提供：

```rust
impl BodyVisibility {
    pub fn get(&self, part: BodyPartId) -> &PartVisibility;
    pub fn get_mut(&mut self, part: BodyPartId) -> &mut PartVisibility;
    pub fn set_part(&mut self, part: BodyPartId, inner: bool, outer: bool);
    pub fn set_inner_layers(&mut self, visible: bool);
    pub fn set_outer_layers(&mut self, visible: bool);
    pub fn set_all(&mut self, visible: bool);
}
```

字段允许直接修改，方法负责常用批量操作。Rust 枚举不实现字符串解析；CLI/WASM 在各自边界转换，避免把特定入口的命名带入 Core。

当前私有 `PartId` 改用公共 `BodyPartId`，不再保留两套同义枚举；保持 `PART_CONFIGS` 顺序不变。

使用示例：

```rust
use eidolon_core::visibility::BodyPartId;

// character 已通过现有资源装载 API 创建。
character.body_visibility.set_outer_layers(false);
character.body_visibility.head.outer = true;

// 只显示头的两层；不影响装备与名字牌。
character.body_visibility.set_all(false);
character.body_visibility.set_part(BodyPartId::Head, true, true);
```

### 7.5 共用渲染入口

在 `encode_render_pass()` 的六部件循环中：

1. 通过 `PART_CONFIGS[i].2` 获取部件 ID，读取当前 Character 的显隐值。
2. 两层都隐藏时，直接跳过该部件的 bind 和 draw。
3. 否则保留原来的 `slot_base + i`，绑定对应 uniform。
4. 按真值表分别绑定并绘制 main、layer；可见层之间的原有顺序不变。
5. Body 之后的 Cape/Elytra 和名字牌流程继续执行。

**不得因为身体全隐藏而跳过整个 Character。** 不得压缩可见部件列表后重新编号 uniform，不得移动 Cape/Elytra/名字牌槽位。必须覆盖超过 `MAX_SCENE_CHARACTERS` 的场景，防止分块后的角色读错显隐或 uniform。

本期可以继续为隐藏部件计算和上传原有 uniform，避免扩大重构。性能要求是“显隐 setter 无 GPU 操作，显隐不新增资源重建”，不是要求现有每帧路径从此零分配；当前帧级资源行为不在本功能中重写。

不更改 shader、贴图 alpha、深度状态或背面剔除。仅外层时出现的空心或背面不可见效果按现有材质规则处理。

### 7.6 WASM API

#### 命名

现有 wasm-bindgen API 使用 snake_case 导出，新增方法必须保持一致。issue 的 camelCase 为能力示意，本期不增加平行的 camelCase 别名，不破坏已有导出。

以下为生成 TypeScript 声明应表达的接口；`part: string` 的运行时合法值仅限上表六个 WASM 字符串。

```typescript
// 单角色：设置可在加载 Skin 之前调用。
set_part_visibility(part: string, inner: boolean, outer: boolean): void;
set_inner_layers_visible(visible: boolean): void;
set_outer_layers_visible(visible: boolean): void;
set_all_parts_visible(visible: boolean): void;

// 场景：只修改指定实体。
set_character_part_visibility(
  id: number, part: string, inner: boolean, outer: boolean
): void;
set_character_inner_layers_visible(id: number, visible: boolean): void;
set_character_outer_layers_visible(id: number, visible: boolean): void;
set_character_all_parts_visible(id: number, visible: boolean): void;
```

带 part 或实体 ID 的 Rust 方法返回 `Result<(), JsValue>`，由 wasm-bindgen 在 JS 侧抛异常；不需要引入自定义 options 对象。其余单角色批量方法不需要返回 Result。

#### 状态与错误

- 单角色无资源时，设置写入待应用状态；不能创建空 Character 来绕过非空约束。
- 建议使用 `pending_body_visibility: BodyVisibility`：存在单角色时，Character 字段为当前状态；不存在时 pending 为当前状态。
- 在销毁最后资源前保存状态；在 `load_skin`、`load_cape`、`load_elytra` 新建 Character 时应用该状态。只在角色创建/销毁时迁移，避免维护两个长期独立可变真源。
- `clear_skin()`、清除最后装备、再次加载资源后设置保留；新 CanvasRenderer 恢复默认。`set_all_parts_visible(true)` 可显式复位。
- `load_skin()` 和 `set_character_skin()` 替换 Body 后保留对应 Character 显隐。
- `add_character()` 的新实体始终全部显示，不继承单角色 pending 或其他实体状态。
- 无 ID 方法只针对既有单角色槽，不修改场景角色，也不切换 scene mode。带 ID 方法不修改单角色 pending。
- 无效/已删除 ID 必须抛异常；无效 part 必须抛异常；大小写敏感，不接受 `right_arm`、`right-arm` 或 `torso`。
- 带 ID 和 part 时，先验证实体 ID，再验证 part；错误中包含非法值和可接受的 part 列表。所有参数验证完成后才写状态，失败无副作用。
- setter 不自动渲染、不自动改变动画时间；在下一次 `render_frame()` 或导出时生效。
- 浏览器离屏导出也必须读取相同 Character 状态，不能恢复默认或忽略场景角色设置。

示例：

```javascript
renderer.set_outer_layers_visible(false);
renderer.set_part_visibility('head', true, true);
renderer.render_frame();

const id = renderer.add_character(skinBytes, 'slim');
renderer.set_character_all_parts_visible(id, false);
renderer.set_character_part_visibility(id, 'head', true, true);
renderer.render_frame();
```

### 7.7 CLI

在共享 `SceneArgs` 中新增：

| 参数 | 类型/默认 | 效果 |
| --- | --- | --- |
| `--hide-inner` | bool / false | 隐藏六部件内层 |
| `--hide-outer` | bool / false | 隐藏六部件外层 |
| `--hide-parts <PARTS>` | 枚举列表 / 空 | 隐藏指定部件两层 |

`--hide-parts` 接受逗号分隔及重复参数，例如 `--hide-parts body,right-arm --hide-parts left-arm`。实现使用 CLI 专用 `ValueEnum`，转换为 Core `BodyPartId`；重复部件幂等，无效值、空值及未知别名由 Clap 拒绝，命令非零退出。

单角色初始化顺序固定为：默认全显示 → hide-inner → hide-outer → hide-parts。三个参数都是 hide 操作，命令行出现顺序不改变结果。共同写入 `SceneState`，经 `apply_to()` 覆盖 render 与 preview，动画逐帧采样不得重置它们。

全部隐藏是合法输入，Body-only 输出为背景；默认透明清屏时所有 alpha 为零。不发警告，不强制显示至少一个部件。

#### 与场景文件的关系

**建议决策：三个新 CLI 隐藏参数与 `--scene-config` 互斥。** 场景文件有自己的逐角色显隐，不定义“命令行覆盖所有角色”的另一套规则。

Clap 必须在创建 Renderer/窗口前拒绝这三类组合，并在帮助中说明使用 `characters[].body_visibility`。不扩展或清理其他已有 SceneArgs 的场景配置行为。

示例：

```bash
eidolon render skin.png base.png --hide-outer
eidolon preview skin.png --hide-inner
eidolon render skin.png head.png --hide-parts body,right-arm,left-arm,right-leg,left-leg
eidolon render skin.png empty.png --hide-inner --hide-outer
```

### 7.8 JSON / TOML 场景配置

`CharacterConfig` 新增可选 `body_visibility` 对象，结构与 Core 六部件一致；每个部件支持可选布尔字段 `inner` 和 `outer`。

```json
{
  "characters": [
    {
      "skin": "skin.png",
      "body_visibility": {
        "head": { "outer": false },
        "right_arm": { "inner": false, "outer": false }
      }
    }
  ]
}
```

```toml
[[characters]]
skin = "skin.png"

[characters.body_visibility.head]
outer = false

[characters.body_visibility.right_arm]
inner = false
outer = false
```

必须满足：

- 省略整个对象、某部件或某一层时，省略的值均为 true；空对象等价于默认。
- 每个角色独立配置；缺省角色不继承前一个角色。
- 不在配置中添加额外的全局层遮罩或 `visible` 字段；批量关闭用六个部件的对应值表示。
- `null`、数字、字符串不能代替对象或布尔值；显隐对象及其部件子对象拒绝未知字段，防止拼错后静默失效。
- 配置根和旧字段的未知字段策略不变；本功能不全局启用 `deny_unknown_fields`。
- 在 CLI 层定义 Deserialize/Serialize DTO 及向 Core 的转换。DTO 的每个布尔值缺省为 true，不能使用 bool 的普通 default；Core 无需增加 serde 依赖。
- JSON 和 TOML 得到相同的 Core 状态，解析后序列化再解析不得丢失显隐值。
- `schemas/scene.json` 同步增加可复用定义：布尔默认 true、六部件对象、不允许显隐对象中的额外字段。Schema default 仅作提示，运行时仍由 DTO 实现默认。
- 配置在 `render` 和 `preview` 使用同一装载与转换路径。

### 7.9 兼容性

- 现有 Rust 构造方法不变；Character 字段默认初始化，包括测试辅助构造。
- 已有 CLI 命令、场景文件、WASM 调用无需修改。
- 不改变 SkinType、皮肤格式识别、HD 和 legacy 转换结果。旧版皮肤可能本来就没有某些外层像素，开启 outer 不会凭空生成内容。
- 资源绑定到 Renderer 的校验继续执行；“隐藏”不能绕过资源归属校验或让外部 Renderer 的 Body 成为合法输入。
- 不新增运行时第三方依赖、网络请求或持久化存储。

### 7.10 测试与可验证的验收

#### 测试素材与比较原则

不能仅用普通皮肤截图的“看起来不同”证明某层没有绘制，也不能把自然皮肤的颜色直接视为外层专属像素。

建议程序生成合法的 64×64 测试 Skin：内层六部件分别使用可识别纯色，外层使用与内层区分的纯色；另生成“外层 atlas 区域全透明”的对照 Skin。根据实际 UV 填充所有相关面，classic/slim 分别验证。至少一个正常视角必须能看到外层，先证明素材和检测器有效。

渲染测试固定尺寸、相机、姿势、背景，关闭名字牌与装备，停止动画。对照图在同一进程、同一后端生成；像素比较基于 RGBA，而非 PNG 文件字节。纯色检测需考虑既有着色与边缘像素，不能硬编码未经验证的单个屏幕坐标。

| ID | 层级 | 场景与必须断言 |
| --- | --- | --- |
| V01 | Core 单元 | Default 的 12 个值全部 true；get/get_mut 六部件映射无串位 |
| V02 | Core 单元 | 每部件四种组合均可设置；set_part 不改其他部件 |
| V03 | Core 单元 | 批量内/外层只改目标层；set_all 覆盖所有值；重复操作幂等；调用顺序符合 7.2 |
| V04 | Core 生命周期 | Character clone 独立；attach/remove Body、SkinType 切换、装备切换不重置状态；无 Body 时配置可保留 |
| V05 | 渲染回归 | 默认与显式 set_all(true) 相同；同环境与改动前默认 RGBA 基线相同 |
| V06 | 渲染集成 | hide-outer 输出与“外层透明对照 Skin、全部显示”的 RGBA 相同；原素材默认输出与该对照不同 |
| V07 | 渲染集成 | 逐一测试六部件 × 四种状态；inner-only/outer-only 有预期层内容，双 false 无该部件贡献；classic/slim 均通过 |
| V08 | 渲染集成 | 仅头时头仍有非透明像素，固定姿势下躯干/四肢专属区域为空；对照完整身体证明区域断言有效 |
| V09 | 渲染集成 | Body 全隐藏且无装备/名字牌时 RGBA 为 clear color；透明 clear 下 alpha 全零 |
| V10 | 渲染集成 | Body 全隐藏时 Cape/Elytra 分别仍可见；独立检查名字牌仍可见；隐藏几何不再遮挡其他角色/装备 |
| V11 | 场景渲染 | 两个共享 Skin 的角色使用不同显隐，不相互影响；17 个以上角色跨分块仍映射正确 |
| V12 | 渲染生命周期 | 隐藏 → 改动作/位置 → 重新显示，使用当前变换；hide/show 后可恢复同状态输出；相机和近似包围盒不变 |
| V13 | CLI 单元 | 三参数默认值、逗号/重复列表、幂等、合法名与错误名；与 scene-config 的冲突均在参数解析时报错 |
| V14 | CLI 集成 | render 输出符合 V06/V08/V09；动画所有采样帧保留显隐；preview 初始化走相同 SceneState，提供窗口冒烟记录 |
| V15 | 配置 | JSON/TOML 缺省、空对象、部分字段、逐角色独立、往返一致；非法类型/未知显隐字段失败；Schema 与运行时一致 |
| V16 | 浏览器 E2E | 单角色四个方法实际改变画布与导出；预加载设置、替换 Skin、clear/reload、classic/slim 后状态保留 |
| V17 | 浏览器 E2E | 场景四个方法只影响目标 ID；新实体默认全显示；已删除/越界 ID 和错误 part 抛异常且无状态变更 |
| V18 | 资源与性能 | setter 路径无纹理/网格上传或 GPU 创建；全隐藏 Body 零身体 draw；提供前后基准结果 |

V07 不应依赖“绘制总数减少”单独证明层选择正确；结合差异素材、对照渲染和映射单元测试。V18 的 draw 验证可用测试专用计数或 GPU capture，不要求新增公共统计 API。

浏览器 E2E 放在 `examples/wasm/tests/visibility.spec.js`。允许用专用 Vite 测试页面直接实例化实际生成的 WASM，避免把生产 CanvasRenderer 暴露到全局对象；不得只 mock setter。等待明确的渲染/导出完成，不用固定 sleep 作为正确性的依据。

#### 性能验收方法

扩展 `crates/core/benches/performance_benchmark.rs`，覆盖全显示、无外层、仅头、全隐藏，以及多角色状态。至少记录 128×128、800×600、1920×1080 的全显示基线。

- 对改动前/后使用相同 release 配置、同设备/驱动/后端、同素材/相机，预热并分别运行至少三轮；记录提交、环境和 Criterion 结果。
- 端到端 render 包含 readback，不能仅以其耗时说明 draw 编码没有开销；另检查 setter 的 CPU 开销和实际 draw 选择。
- **建议回归门槛：全显示耗时稳定增加超过 3% 且超过测量噪声时阻塞合并并调查。** 此数值是评审建议，不是现有实测结果；若环境无法分辨该幅度，标记“测量不充分”，不能声称零回归。
- 隐藏场景不强制固定加速倍数；readback、uniform 和背景处理成本仍存在。
- 切换操作必须零 GPU 资源创建和上传；现有每帧 uniform 分配不算显隐 setter 的新增成本。

## 8. 发布与实施计划

### 8.1 分阶段交付

| 阶段 | 交付物 | 完成条件 |
| --- | --- | --- |
| P0：契约评审 | 确认 Character 归属、批量写入语义、snake_case WASM、配置/CLI 互斥规则 | 本文评审通过 |
| P1：Core | visibility 模块、Character 字段、PartId 统一、条件 draw、核心测试 | V01–V12 对应测试通过 |
| P2：适配层 | SceneArgs/SceneState、JSON/TOML DTO、Schema、WASM 单角色与 ID 方法、pending 生命周期 | V13–V17 对应测试通过 |
| P3：发布验证 | E2E、默认基线对比、基准记录、中英文文档与示例 | V18 与全部回归通过 |

不承诺未经估算的具体日期；P1 完成后 CLI 与 WASM 可独立开发，发布时作为同一功能交付。完整 Studio 控制面板作为后续独立范围，不阻塞 issue #16 的底层能力。

### 8.2 文件改动清单

| 文件/目录 | 预期变更 |
| --- | --- |
| `crates/core/src/visibility.rs` | 新类型、默认值、访问/批量方法、纯 CPU 单测 |
| `crates/core/src/lib.rs` | 暴露 visibility 模块 |
| `crates/core/src/character.rs` | 状态字段、所有构造路径与 Debug 输出 |
| `crates/core/src/renderer/uniforms.rs` | 使用同一公共部件枚举，保留配置与槽位顺序 |
| `crates/core/src/renderer/mod.rs` | 共用 pass 条件绘制 |
| `crates/core/tests/`、`src/test_support.rs` | 测试 Skin、渲染对照、生命周期/多角色测试，按现有测试布局组织 |
| `crates/core/benches/performance_benchmark.rs` | 显隐基准场景 |
| `crates/cli/src/args.rs` | 三参数、部件 ValueEnum、scene-config 冲突规则及帮助 |
| `crates/cli/src/scene.rs` | DTO、转换、SceneState、配置装载与单元测试 |
| CLI 测试及必要调用处 | 更新 SceneArgs/SceneState 构造；验证 preview 和 animation 不丢状态 |
| `schemas/scene.json` | 新配置定义、默认说明和显隐对象未知字段约束 |
| `crates/wasm/src/renderer.rs` | 八个方法、解析、pending、创建/销毁迁移与导出状态一致性 |
| `examples/wasm/tests/visibility.spec.js` 及测试页面 | 实际 WASM E2E |
| `docs/library.md`、`docs/library_zh-CN.md` | Rust 类型、使用方式与批量覆盖语义 |
| `docs/cli.md`、`docs/cli_zh-CN.md` | 新参数、场景 JSON/TOML、互斥与全隐藏行为 |
| `crates/wasm/README.md`、`examples/wasm/README.md` | WASM 接口、生命周期与运行测试说明 |
| `CONTEXT.md`、必要的 ADR | 补充 Body visibility 术语；如需 ADR，评审后记录 Character 状态归属决策 |

不提交手工修改的 WASM 生成文件或本机截图作为源代码替代。文档增加后按仓库约定更新相关索引链接。

### 8.3 验证命令

以下是实现完成后运行的命令，不表示本文编写时已执行这些测试。GPU/浏览器所需依赖与环境变量沿用仓库开发文档。

```bash
rtk cargo fmt --all -- --check
rtk cargo test -p eidolon-core -- --test-threads=1
rtk cargo test -p eidolon-cli
rtk cargo check -p eidolon-wasm --target wasm32-unknown-unknown
rtk pnpm --dir examples/wasm run build:wasm
rtk pnpm --dir examples/wasm run test:e2e
rtk cargo bench -p eidolon-core --bench performance_benchmark
```

基线采集应在实现前保存；若现有测试因环境缺少 GPU/浏览器依赖而无法运行，必须记录具体阻塞，不能把跳过视为通过。

### 8.4 风险与缓解

| 风险 | 缓解 |
| --- | --- |
| Default 错用 bool 默认导致旧场景身体消失 | Core 与 DTO 分层测试全部默认为 true |
| Skin 替换或单角色被清空后设置丢失 | Character 持有状态，WASM pending 在创建/销毁边界迁移 |
| 只修改单角色离屏路径，浏览器/场景无效 | 修改唯一共用 pass，并跨入口验收 |
| 部件筛选后压缩索引导致装备/后续角色变形 | 固定槽位不变，17+ 角色测试 |
| 普通皮肤没外层，测试“假通过” | 明确有外层的合成素材与正向对照 |
| “全局开关”被误解为永久总遮罩 | API 文档与顺序覆盖测试明确批量赋值 |
| 将隐藏理解为卸载，误触 Character 非空校验 | 全隐藏保持资源与角色存在，装备继续渲染 |
| 期待只头时自动放大或重新居中 | 帮助与文档明确本期不改变相机和取景 |

### 8.5 合并检查表

- [ ] 确认 7.2、7.3、7.6、7.7 的建议决策，无未解决的阻塞设计问题。
- [ ] 六部件及双层覆盖完整，所有默认值为 true。
- [ ] 单角色/多角色、classic/slim、离屏/窗口/浏览器选择同一显隐状态。
- [ ] Skin 替换、clear/reload、克隆、动画均保留预期状态。
- [ ] 配置、Schema、CLI 帮助和 WASM 文档一致。
- [ ] V01–V18 有测试结果或要求的人工/性能证据；环境阻塞单独记录。
- [ ] 默认 RGBA 回归对照通过，性能结论附基线与测量环境。
- [ ] 中英文示例可复现；未扩大到 Studio 重设计或渲染资源重构。
