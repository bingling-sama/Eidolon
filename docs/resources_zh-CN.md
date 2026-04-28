# 资源

本项目在 `resources/` 中提供了示例模型与皮肤，便于测试与演示。

## 内置资源

- `resources/classic.obj` Steve 模型（经典手臂）
- `resources/slim.obj` Alex 模型（细手臂）
- `resources/bingling_sama.png` 双层 Alex 皮肤
- `resources/undefinedR2.png` 双层 Steve 皮肤
- `resources/SSSSSteven.png` 单层 Steve 皮肤

## 说明

- 单层皮肤在加载时会自动转换为双层，前提是 `width == height * 2`。
- 双层皮肤应为正方形，例如 `64x64`。
- 自定义皮肤请使用 PNG 格式。
- 根据皮肤手臂宽度匹配 `--skin-type classic` 或 `--skin-type slim`。
- 替换 OBJ 时必须保留 `architecture_zh-CN.md` 中列出的对象名。

## 资源使用示例

```bash
cargo run -- render --skin-type slim --texture resources/bingling_sama.png
cargo run -- render --skin-type classic --texture resources/undefinedR2.png
cargo run -- convert resources/SSSSSteven.png converted.png
```

## 致谢

- `undefinedR2` 提供双层 Steve 皮肤
- `SSSSSteven` 提供单层 Steve 皮肤
- `bingling-sama` 提供双层 Alex 皮肤
