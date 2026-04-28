# 贡献指南

欢迎提交贡献。

## 参与方式

- 通过 Issue 讨论问题、想法或提案
- 提交 PR，并清晰说明变更内容
- 变更保持聚焦，必要时提供使用说明或示例

## 开发说明

- 使用 `cargo build` 构建
- 使用 `cargo fmt --check` 检查格式
- 使用 `cargo clippy --all-targets --all-features` 检查 lint
- 使用 `cargo test` 运行测试
- 参考 `docs/cli.md` 中的命令行示例
- 修改性能敏感渲染路径时运行 `cargo bench`

## 文档变更

修改 CLI 参数、公开库字段、资源名称或输出行为时，请同步更新英文和中文文档。文档索引位于
`docs/README.md` 和 `docs/README_zh-CN.md`。

## PR 检查清单

- 变更聚焦在一个行为或一个文档主题上
- 视觉行为变化时附带截图或渲染输出
- 说明平台相关的 GPU 或窗口环境要求
- 标明变更后仍然存在的已知限制
