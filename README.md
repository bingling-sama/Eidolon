# Minecraft Skin Viewer

一个用Rust编写的Minecraft皮肤查看器，可以渲染3D模型并保存为图片。

## 功能特性

- 加载和渲染Minecraft玩家模型
- 支持纹理贴图
- 3D渲染效果
- 可以保存渲染结果为PNG图片
- 交互式窗口模式

## 使用方法

直接渲染并保存图片：
```bash
cargo run -- <文件名> [--width <宽度>] [--height <高度>]
```

参数说明：
- <文件名>（可选）：输出图片文件名，默认 output.png
- --width（可选）：输出图片宽度，默认 800
- --height（可选）：输出图片高度，默认 600

示例：
```bash
# 保存为默认尺寸 (800x600)
cargo run -- output.png

# 保存为自定义尺寸
cargo run -- my_skin.png --width 1024 --height 768
```

## 文件结构

```