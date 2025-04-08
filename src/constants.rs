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
