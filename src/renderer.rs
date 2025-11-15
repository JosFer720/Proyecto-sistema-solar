use sdl2::pixels::Color;
use sdl2::rect::Point;
use glam::{Vec3, Mat4};
use crate::framebuffer::ZBuffer;
use crate::shader_type::ShaderType;
use crate::shaders::apply_shader;
use crate::utils::create_model_matrix;

pub const SCREEN_WIDTH: u32 = 800;
pub const SCREEN_HEIGHT: u32 = 600;

pub fn fill_triangle_zbuffer(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    zbuffer: &mut ZBuffer,
    mut p0: Point, mut p1: Point, mut p2: Point,
    mut z0: f32, mut z1: f32, mut z2: f32,
    color: Color
) {
    let in_bounds = |p: &Point| {
        p.x >= 0 && p.x < SCREEN_WIDTH as i32 && p.y >= 0 && p.y < SCREEN_HEIGHT as i32
    };
    
    if !in_bounds(&p0) && !in_bounds(&p1) && !in_bounds(&p2) {
        return;
    }
    
    if p0.y > p1.y { 
        std::mem::swap(&mut p0, &mut p1);
        std::mem::swap(&mut z0, &mut z1);
    }
    if p0.y > p2.y { 
        std::mem::swap(&mut p0, &mut p2);
        std::mem::swap(&mut z0, &mut z2);
    }
    if p1.y > p2.y { 
        std::mem::swap(&mut p1, &mut p2);
        std::mem::swap(&mut z1, &mut z2);
    }
    
    canvas.set_draw_color(color);
    
    let total_height = p2.y - p0.y;
    if total_height == 0 { return; }
    
    let y_start = p0.y.max(0);
    let y_end = p2.y.min(SCREEN_HEIGHT as i32 - 1);
    
    for y in y_start..=y_end {
        let is_upper_half = y <= p1.y;
        let segment_height = if is_upper_half { p1.y - p0.y } else { p2.y - p1.y };
        
        if segment_height == 0 { continue; }
        
        let alpha = (y - p0.y) as f32 / total_height as f32;
        let beta = if is_upper_half {
            if p1.y == p0.y { 0.0 } else { (y - p0.y) as f32 / (p1.y - p0.y) as f32 }
        } else {
            if p2.y == p1.y { 0.0 } else { (y - p1.y) as f32 / (p2.y - p1.y) as f32 }
        };
        
        let ax = p0.x as f32 + (p2.x - p0.x) as f32 * alpha;
        let az = z0 + (z2 - z0) * alpha;
        
        let (bx, bz) = if is_upper_half {
            (p0.x as f32 + (p1.x - p0.x) as f32 * beta, z0 + (z1 - z0) * beta)
        } else {
            (p1.x as f32 + (p2.x - p1.x) as f32 * beta, z1 + (z2 - z1) * beta)
        };
        
        let (x_start, x_end, z_start, z_end) = if ax < bx {
            (ax as i32, bx as i32, az, bz)
        } else {
            (bx as i32, ax as i32, bz, az)
        };
        
        let x_start = x_start.max(0);
        let x_end = x_end.min(SCREEN_WIDTH as i32 - 1);
        
        if x_start <= x_end {
            for x in x_start..=x_end {
                let t = if x_end == x_start { 
                    0.0 
                } else { 
                    (x - x_start) as f32 / (x_end - x_start) as f32 
                };
                let z = z_start + (z_end - z_start) * t;
                
                if zbuffer.test_and_set(x, y, z) {
                    let _ = canvas.draw_point(Point::new(x, y));
                }
            }
        }
    }
}

pub fn render(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, 
    zbuffer: &mut ZBuffer, 
    model: &tobj::Model, 
    camera_position: Vec3,
    camera_target: Vec3,
    world_position: Vec3,
    model_center: Vec3,
    model_scale: f32,
    rotation_y: f32,
    shader_type: ShaderType,
    time: f32,
) {
    let positions = &model.mesh.positions;
    let indices = &model.mesh.indices;

    let projection = Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32, 1.0, 50000.0);
    let view = Mat4::look_at_rh(
        camera_position,
        camera_target,
        Vec3::Y,
    );
    
    let rotation_vec = Vec3::new(0.0, rotation_y, 0.0);
    let model_matrix = create_model_matrix(
        world_position,
        model_center,
        model_scale,
        rotation_vec
    );

    let mvp = projection * view * model_matrix;

    for i in (0..indices.len()).step_by(3) {
        let i0 = indices[i] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        let v0 = Vec3::new(positions[3 * i0], positions[3 * i0 + 1], positions[3 * i0 + 2]);
        let v1 = Vec3::new(positions[3 * i1], positions[3 * i1 + 1], positions[3 * i1 + 2]);
        let v2 = Vec3::new(positions[3 * i2], positions[3 * i2 + 1], positions[3 * i2 + 2]);

        let p0 = mvp * v0.extend(1.0);
        let p1 = mvp * v1.extend(1.0);
        let p2 = mvp * v2.extend(1.0);

        let screen_p0 = Point::new(
            ((p0.x / p0.w + 1.0) * 0.5 * SCREEN_WIDTH as f32) as i32,
            ((1.0 - (p0.y / p0.w + 1.0) * 0.5) * SCREEN_HEIGHT as f32) as i32,
        );
        let screen_p1 = Point::new(
            ((p1.x / p1.w + 1.0) * 0.5 * SCREEN_WIDTH as f32) as i32,
            ((1.0 - (p1.y / p1.w + 1.0) * 0.5) * SCREEN_HEIGHT as f32) as i32,
        );
        let screen_p2 = Point::new(
            ((p2.x / p2.w + 1.0) * 0.5 * SCREEN_WIDTH as f32) as i32,
            ((1.0 - (p2.y / p2.w + 1.0) * 0.5) * SCREEN_HEIGHT as f32) as i32,
        );

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize_or_zero();
        
        let light_dir = Vec3::new(0.5, 0.7, 1.0).normalize();
        let intensity = normal.dot(light_dir).max(0.0);
        
        let avg_position = (v0 + v1 + v2) / 3.0;
        
        let (r, g, b) = apply_shader(
            shader_type,
            avg_position,
            normal,
            intensity,
            time
        );
        
        let z0 = (p0.z / p0.w + 1.0) * 0.5;
        let z1 = (p1.z / p1.w + 1.0) * 0.5;
        let z2 = (p2.z / p2.w + 1.0) * 0.5;
        
        fill_triangle_zbuffer(
            canvas,
            zbuffer,
            screen_p0, screen_p1, screen_p2,
            z0, z1, z2,
            Color::RGB(r, g, b)
        );
    }
}

pub fn render_with_full_rotation(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, 
    zbuffer: &mut ZBuffer, 
    model: &tobj::Model, 
    camera_position: Vec3,
    camera_target: Vec3,
    world_position: Vec3,
    model_center: Vec3,
    model_scale: f32,
    rotation: Vec3,
    shader_type: ShaderType,
    time: f32,
) {
    let positions = &model.mesh.positions;
    let indices = &model.mesh.indices;

    let projection = Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32, 1.0, 50000.0);
    let view = Mat4::look_at_rh(
        camera_position,
        camera_target,
        Vec3::Y,
    );
    
    let model_matrix = create_model_matrix(
        world_position,
        model_center,
        model_scale,
        rotation
    );

    let mvp = projection * view * model_matrix;

    for i in (0..indices.len()).step_by(3) {
        let i0 = indices[i] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        let v0 = Vec3::new(positions[3 * i0], positions[3 * i0 + 1], positions[3 * i0 + 2]);
        let v1 = Vec3::new(positions[3 * i1], positions[3 * i1 + 1], positions[3 * i1 + 2]);
        let v2 = Vec3::new(positions[3 * i2], positions[3 * i2 + 1], positions[3 * i2 + 2]);

        let p0 = mvp * v0.extend(1.0);
        let p1 = mvp * v1.extend(1.0);
        let p2 = mvp * v2.extend(1.0);

        let screen_p0 = Point::new(
            ((p0.x / p0.w + 1.0) * 0.5 * SCREEN_WIDTH as f32) as i32,
            ((1.0 - (p0.y / p0.w + 1.0) * 0.5) * SCREEN_HEIGHT as f32) as i32,
        );
        let screen_p1 = Point::new(
            ((p1.x / p1.w + 1.0) * 0.5 * SCREEN_WIDTH as f32) as i32,
            ((1.0 - (p1.y / p1.w + 1.0) * 0.5) * SCREEN_HEIGHT as f32) as i32,
        );
        let screen_p2 = Point::new(
            ((p2.x / p2.w + 1.0) * 0.5 * SCREEN_WIDTH as f32) as i32,
            ((1.0 - (p2.y / p2.w + 1.0) * 0.5) * SCREEN_HEIGHT as f32) as i32,
        );

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize_or_zero();
        
        let light_dir = Vec3::new(0.5, 0.7, 1.0).normalize();
        let intensity = normal.dot(light_dir).max(0.0);
        
        let avg_position = (v0 + v1 + v2) / 3.0;
        
        let (r, g, b) = apply_shader(
            shader_type,
            avg_position,
            normal,
            intensity,
            time
        );
        
        let z0 = (p0.z / p0.w + 1.0) * 0.5;
        let z1 = (p1.z / p1.w + 1.0) * 0.5;
        let z2 = (p2.z / p2.w + 1.0) * 0.5;
        
        fill_triangle_zbuffer(
            canvas,
            zbuffer,
            screen_p0, screen_p1, screen_p2,
            z0, z1, z2,
            Color::RGB(r, g, b)
        );
    }
}
