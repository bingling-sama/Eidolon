// 创建视图矩阵的函数
pub fn view_matrix(position: &[f32; 3], direction: &[f32; 3], up: &[f32; 3]) -> [[f32; 4]; 4] {
    let f = {
        let f = direction;
        let len = f[0] * f[0] + f[1] * f[1] + f[2] * f[2];
        let len = len.sqrt();
        [f[0] / len, f[1] / len, f[2] / len]
    };

    let s = [up[1] * f[2] - up[2] * f[1],
             up[2] * f[0] - up[0] * f[2],
             up[0] * f[1] - up[1] * f[0]];

    let s_norm = {
        let len = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
        let len = len.sqrt();
        [s[0] / len, s[1] / len, s[2] / len]
    };

    let u = [f[1] * s_norm[2] - f[2] * s_norm[1],
             f[2] * s_norm[0] - f[0] * s_norm[2],
             f[0] * s_norm[1] - f[1] * s_norm[0]];

    let p = [-position[0] * s_norm[0] - position[1] * s_norm[1] - position[2] * s_norm[2],
             -position[0] * u[0] - position[1] * u[1] - position[2] * u[2],
             -position[0] * f[0] - position[1] * f[1] - position[2] * f[2]];

    [
        [s_norm[0], u[0], f[0], 0.0],
        [s_norm[1], u[1], f[1], 0.0],
        [s_norm[2], u[2], f[2], 0.0],
        [p[0], p[1], p[2], 1.0],
    ]
}

// 着色器常量
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
