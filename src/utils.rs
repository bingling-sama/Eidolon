//! 工具函数模块
//!
//! 这个模块包含了一些通用的数学工具函数，
//! 主要用于 3D 图形渲染中的矩阵计算。

/// 创建视图矩阵
///
/// 根据相机位置、观察方向和上方向创建视图矩阵。
/// 这个函数实现了标准的 LookAt 算法。
///
/// # 参数
///
/// * `position` - 相机位置坐标 [x, y, z]
/// * `direction` - 观察方向向量 [x, y, z]
/// * `up` - 上方向向量 [x, y, z]
///
/// # 返回
///
/// 返回 4x4 的视图矩阵
///
/// # 算法
///
/// 1. 计算前向量 (f)：观察方向的单位向量
/// 2. 计算右向量 (s)：上向量和前向量的叉积
/// 3. 计算上向量 (u)：前向量和右向量的叉积
/// 4. 计算平移分量 (p)：相机位置的负值
/// 5. 组合成视图矩阵
///
/// # 示例
///
/// ```rust
/// use skinviewer::utils::view_matrix;
///
/// let view = view_matrix(
///     &[0.0, 1.0, 4.0],    // 相机位置
///     &[0.0, -0.2, -1.0],  // 观察方向
///     &[0.0, 1.0, 0.0]     // 上方向
/// );
/// ```
pub fn view_matrix(position: &[f32; 3], direction: &[f32; 3], up: &[f32; 3]) -> [[f32; 4]; 4] {
    let f = {
        let f = direction;
        let len = f[0] * f[0] + f[1] * f[1] + f[2] * f[2];
        let len = len.sqrt();
        [f[0] / len, f[1] / len, f[2] / len]
    };

    let s = [
        up[1] * f[2] - up[2] * f[1],
        up[2] * f[0] - up[0] * f[2],
        up[0] * f[1] - up[1] * f[0],
    ];

    let s_norm = {
        let len = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
        let len = len.sqrt();
        [s[0] / len, s[1] / len, s[2] / len]
    };

    let u = [
        f[1] * s_norm[2] - f[2] * s_norm[1],
        f[2] * s_norm[0] - f[0] * s_norm[2],
        f[0] * s_norm[1] - f[1] * s_norm[0],
    ];

    let p = [
        -position[0] * s_norm[0] - position[1] * s_norm[1] - position[2] * s_norm[2],
        -position[0] * u[0] - position[1] * u[1] - position[2] * u[2],
        -position[0] * f[0] - position[1] * f[1] - position[2] * f[2],
    ];

    [
        [s_norm[0], u[0], f[0], 0.0],
        [s_norm[1], u[1], f[1], 0.0],
        [s_norm[2], u[2], f[2], 0.0],
        [p[0], p[1], p[2], 1.0],
    ]
}
