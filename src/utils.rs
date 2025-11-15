use glam::Mat4;

pub fn hash(x: f32, y: f32) -> f32 {
    let n = (x * 12.9898 + y * 78.233).sin() * 43758.5453;
    n - n.floor()
}

pub fn noise(x: f32, y: f32) -> f32 {
    let i = x.floor();
    let j = y.floor();
    let f_x = x - i;
    let f_y = y - j;
    
    let u = f_x * f_x * (3.0 - 2.0 * f_x);
    let v = f_y * f_y * (3.0 - 2.0 * f_y);
    
    let a = hash(i, j);
    let b = hash(i + 1.0, j);
    let c = hash(i, j + 1.0);
    let d = hash(i + 1.0, j + 1.0);
    
    a * (1.0 - u) * (1.0 - v) + 
    b * u * (1.0 - v) + 
    c * (1.0 - u) * v + 
    d * u * v
}

/// Matriz 4x4 de transformación del modelo
pub fn create_model_matrix(world_translation: glam::Vec3, model_center: glam::Vec3, scale: f32, rotation: glam::Vec3) -> Mat4 {
    let center_matrix = Mat4::from_translation(model_center);
    
    let scale_matrix = Mat4::from_scale(glam::Vec3::splat(scale));
    

    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();
    
    // Matriz de rotación en X
    let rotation_matrix_x = Mat4::from_cols_array(&[
        1.0,  0.0,    0.0,   0.0,
        0.0,  cos_x, -sin_x, 0.0,
        0.0,  sin_x,  cos_x, 0.0,
        0.0,  0.0,    0.0,   1.0,
    ]);
    
    // Matriz de rotación en Y
    let rotation_matrix_y = Mat4::from_cols_array(&[
        cos_y,  0.0,  sin_y, 0.0,
        0.0,    1.0,  0.0,   0.0,
       -sin_y,  0.0,  cos_y, 0.0,
        0.0,    0.0,  0.0,   1.0,
    ]);
    
    // Matriz de rotación en Z
    let rotation_matrix_z = Mat4::from_cols_array(&[
        cos_z, -sin_z, 0.0, 0.0,
        sin_z,  cos_z, 0.0, 0.0,
        0.0,    0.0,   1.0, 0.0,
        0.0,    0.0,   0.0, 1.0,
    ]);
    
    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;
    
    let world_matrix = Mat4::from_translation(world_translation);
    world_matrix * rotation_matrix * scale_matrix * center_matrix
}
