//! 着色器常量模块
//!
//! 这个模块包含了用于渲染 Minecraft 皮肤的 OpenGL 着色器代码。
//! 着色器支持纹理映射、光照计算和透明度处理。

/// 顶点着色器
///
/// 处理顶点变换、法线变换和纹理坐标传递。
/// 使用 GLSL 410 版本，支持现代 OpenGL 特性。
///
/// # 输入变量
///
/// - `position`: 顶点位置 (vec3)
/// - `normal`: 顶点法线 (vec3)
/// - `texture`: 纹理坐标 (vec2)
///
/// # 输出变量
///
/// - `v_normal`: 传递给片段着色器的法线 (vec3)
/// - `v_texture`: 传递给片段着色器的纹理坐标 (vec2)
///
/// # Uniform 变量
///
/// - `perspective`: 透视投影矩阵 (mat4)
/// - `view`: 视图矩阵 (mat4)
/// - `model`: 模型变换矩阵 (mat4)
pub const VERTEX_SHADER: &str = r#"
    #version 410

    in vec3 position;
    in vec3 normal;
    in vec2 texture;
    
    out vec3 v_normal;
    out vec2 v_texture;

    uniform mat4 perspective;
    uniform mat4 view;
    uniform mat4 model;

    void main() {
        mat4 modelview = view * model;
        v_texture = texture;
        v_normal = transpose(inverse(mat3(model))) * normal;
        gl_Position = perspective * modelview * vec4(position, 1.0);
    }
"#;

/// 片段着色器
///
/// 处理纹理采样、光照计算和透明度处理。
/// 使用 GLSL 330 版本，提供 Minecraft 风格的渲染效果。
///
/// # 输入变量
///
/// - `v_texture`: 从顶点着色器传来的纹理坐标 (vec2)
/// - `v_normal`: 从顶点着色器传来的法线 (vec3)
///
/// # 输出变量
///
/// - `FragColor`: 最终输出的颜色 (vec4)
///
/// # Uniform 变量
///
/// - `texture1`: 皮肤纹理采样器 (sampler2D)
///
/// # 特性
///
/// - 支持透明度处理（丢弃完全透明的像素）
/// - 双光源照明系统（主光源 + 辅助光源）
/// - 保留原始纹理颜色
/// - 适合 Minecraft 像素艺术风格
pub const FRAGMENT_SHADER: &str = r#"
    #version 330 core
    in vec2 v_texture;
    in vec3 v_normal;
    out vec4 FragColor;

    uniform sampler2D texture1;

    void main()
    {
        // Minecraft textures are pixel art, so we want nearest neighbor filtering
        vec4 texColor = texture(texture1, v_texture);

        // 只丢弃完全透明的像素，保留半透明像素
        if(texColor.a < 0.01)
            discard;

        // Enhanced lighting for Minecraft models
        vec3 lightDir1 = normalize(vec3(1.0, 1.0, 1.0));
        vec3 lightDir2 = normalize(vec3(-1.0, 0.5, -0.5)); // Secondary light from opposite direction

        float ambient = 0.5; // Higher ambient for Minecraft-style lighting
        float diff1 = max(dot(normalize(v_normal), lightDir1), 0.0);
        float diff2 = max(dot(normalize(v_normal), lightDir2), 0.0) * 0.3; // Secondary light is dimmer

        vec3 diffuse = (ambient + diff1 * 0.5 + diff2) * vec3(1.0, 1.0, 1.0);

        // Apply lighting but preserve original colors
        FragColor = vec4(texColor.rgb * diffuse, texColor.a);
    }
"#;
