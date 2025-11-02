extern crate sdl2;
extern crate tobj;
extern crate glam;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Point;
use glam::{Vec3, Mat4};
 
use std::fs;
use std::io::Cursor;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

// Z-buffer para almacenar la profundidad de cada píxel
struct ZBuffer {
    buffer: Vec<f32>,
    width: usize,
    height: usize,
}

impl ZBuffer {
    fn new(width: usize, height: usize) -> Self {
        ZBuffer {
            buffer: vec![f32::INFINITY; width * height],
            width,
            height,
        }
    }
    
    fn clear(&mut self) {
        self.buffer.fill(f32::INFINITY);
    }
    
    fn test_and_set(&mut self, x: i32, y: i32, depth: f32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        let idx = (y as usize) * self.width + (x as usize);
        if depth < self.buffer[idx] {
            self.buffer[idx] = depth;
            true
        } else {
            false
        }
    }
}

// Estructura para almacenar un triángulo con información 3D completa
struct Triangle3D {
    v0: glam::Vec4,
    v1: glam::Vec4,
    v2: glam::Vec4,
    screen_p0: Point,
    screen_p1: Point,
    screen_p2: Point,
    color: Color,
}

/// Función auxiliar para rellenar un triángulo con z-buffer
fn fill_triangle_zbuffer(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    zbuffer: &mut ZBuffer,
    mut p0: Point, mut p1: Point, mut p2: Point,
    mut z0: f32, mut z1: f32, mut z2: f32,
    color: Color
) {
    // Verificar que los puntos estén dentro de los límites de la pantalla
    let in_bounds = |p: &Point| {
        p.x >= 0 && p.x < SCREEN_WIDTH as i32 && p.y >= 0 && p.y < SCREEN_HEIGHT as i32
    };
    
    // Si todos los puntos están muy fuera de pantalla, no dibujar
    if !in_bounds(&p0) && !in_bounds(&p1) && !in_bounds(&p2) {
        return;
    }
    
    // Ordenar los puntos por coordenada Y (p0.y <= p1.y <= p2.y)
    // También intercambiar las profundidades correspondientes
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
    
    // Relleno simple línea por línea con interpolación de profundidad
    let total_height = p2.y - p0.y;
    if total_height == 0 { return; }
    
    // Limitar el rango de Y a los límites de la pantalla
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

/// Renderiza un modelo 3D en el canvas.
/// Dibuja el modelo con triángulos rellenos y sombreado simple.
fn render(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, 
    zbuffer: &mut ZBuffer, 
    model: &tobj::Model, 
    camera_distance: f32, 
    model_translation: glam::Vec3, 
    model_scale: f32, 
    rotation_y: f32, 
    mesh_index: usize
) {
    let positions = &model.mesh.positions;
    let indices = &model.mesh.indices;

    // Matrices de transformación para pasar de coordenadas 3D a 2D
    let projection = Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at_rh(
        Vec3::new(0.0, 0.0, camera_distance), // Posición de la cámara (ajustable)
        Vec3::ZERO,      // Hacia dónde mira la cámara
        Vec3::Y,         // Vector "arriba"
    );
    
    // Matriz del modelo: primero rotar en Y, luego trasladar para centrar y escalar
    let model_matrix = Mat4::from_scale(Vec3::splat(model_scale)) 
        * Mat4::from_translation(model_translation)
        * Mat4::from_rotation_y(rotation_y);

    // Matriz final Modelo-Vista-Proyección
    let mvp = projection * view * model_matrix;

    // Itera sobre todos los triángulos del modelo
    for i in (0..indices.len()).step_by(3) {
        let i0 = indices[i] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        let v0 = Vec3::new(positions[3 * i0], positions[3 * i0 + 1], positions[3 * i0 + 2]);
        let v1 = Vec3::new(positions[3 * i1], positions[3 * i1 + 1], positions[3 * i1 + 2]);
        let v2 = Vec3::new(positions[3 * i2], positions[3 * i2 + 1], positions[3 * i2 + 2]);

        // Transforma los vértices del espacio del modelo al espacio de la pantalla
        let p0 = mvp * v0.extend(1.0);
        let p1 = mvp * v1.extend(1.0);
        let p2 = mvp * v2.extend(1.0);

        // Mapea las coordenadas a la pantalla
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

        // Calcular normal del triángulo para sombreado simple
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize_or_zero(); // Usar normalize_or_zero para evitar NaN
        
        // Dirección de la luz (luz direccional desde arriba-derecha)
        let light_dir = Vec3::new(0.5, 0.7, 1.0).normalize();
        
        // Calcular intensidad de luz (producto punto entre normal y dirección de luz)
        let intensity = normal.dot(light_dir).max(0.0);
        
        // Esquema de colores metálico - Estilo Military/Tactical
        let (base_r, base_g, base_b) = match mesh_index {
            // Mesh 0: Alas - Gris oscuro metálico (NO TOCAR)
            0 => (70, 75, 80),
            
            // Mesh 2: CUERPO PRINCIPAL - VERDE OLIVA CON VARIACIONES
            2 => {
                // USAR COORDENADAS ORIGINALES
                let avg_y = (v0.y + v1.y + v2.y) / 3.0;
                let avg_x = (v0.x + v1.x + v2.x) / 3.0;
                let avg_z = (v0.z + v1.z + v2.z) / 3.0;
                
                // Cockpit/vidrio - parte superior central frontal - VERDE OLIVA MÁS CLARO
                if avg_y > 0.1 && avg_x > -2.0 {
                    (160, 180, 120)  // Verde oliva claro - COCKPIT
                }
                // Parte frontal - Verde oliva claro
                else if avg_x > -3.0 {
                    (140, 160, 100)  // Verde oliva claro
                }
                // Parte trasera - Verde oscuro
                else if avg_x < -5.5 {
                    (90, 110, 70)    // Verde oscuro
                }
                // Laterales del cuerpo - Verde militar medio
                else if avg_z < -4.0 {
                    (115, 135, 90)   // Verde militar medio
                }
                // Centro del cuerpo - Verde oliva base
                else {
                    (105, 125, 85)   // Verde oliva base
                }
            },
            
            // Mesh 3: Protuberancias y detalles - MARRÓN Y GRIS VERDOSO
            3 => {
                // USAR COORDENADAS ORIGINALES
                let avg_x = (v0.x + v1.x + v2.x) / 3.0;
                let avg_z = (v0.z + v1.z + v2.z) / 3.0;
                let avg_y = (v0.y + v1.y + v2.y) / 3.0;
                
                // Protuberancias superiores - Gris claro
                if avg_y > 2.5 {
                    (150, 155, 135)  // Gris claro verdoso
                }
                // Protuberancias traseras profundas - Marrón militar
                else if avg_z < -5.0 {
                    (130, 110, 75)   // Marrón militar/arena
                }
                // Protuberancias laterales externas - Gris verdoso
                else if avg_x.abs() > 4.0 {
                    (125, 130, 105)  // Gris verdoso
                }
                // Detalles centrales - Verde oscuro
                else {
                    (80, 100, 65)    // Verde oscuro
                }
            },
            
            // Mesh 1 u otros: Gris por defecto
            _ => (150, 150, 150),
        };
        
        // Aplicar intensidad de luz al color - MÁS LUZ AMBIENTAL (70% base + 30% direccional)
        let r = (base_r as f32 * (0.7 + 0.3 * intensity)).min(255.0) as u8;
        let g = (base_g as f32 * (0.7 + 0.3 * intensity)).min(255.0) as u8;
        let b = (base_b as f32 * (0.7 + 0.3 * intensity)).min(255.0) as u8;
        
        // Dibujar el triángulo directamente con z-buffer
        let z0 = p0.w;
        let z1 = p1.w;
        let z2 = p2.w;
        
        fill_triangle_zbuffer(
            canvas,
            zbuffer,
            screen_p0, screen_p1, screen_p2,
            z0, z1, z2,
            Color::RGB(r, g, b)
        );
    }
}

fn main() -> Result<(), String> {
    // Inicializa SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // Crea la ventana
    let window = video_subsystem.window("Renderizador de OBJ en Rust", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    // Crea un canvas para dibujar
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    // Carga el archivo OBJ desde la carpeta Spaceship
    // Leemos el contenido y removemos las líneas de materiales para evitar errores de parseo
    let obj_content = fs::read_to_string("Spaceship/Spaceship.obj")
        .expect("No se pudo leer el archivo OBJ");
    
    // Filtramos las líneas que contienen mtllib y usemtl para ignorar materiales
    let filtered_content: String = obj_content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("mtllib") && !trimmed.starts_with("usemtl")
        })
        .collect::<Vec<&str>>()
        .join("\n");
    
    let mut obj_reader = Cursor::new(filtered_content.as_bytes());
    
    let load_options = tobj::LoadOptions {
        triangulate: true,
        ignore_points: true,
        ignore_lines: true,
        ..Default::default()
    };
    
    // Cargamos el OBJ sin referencias a materiales
    let result = tobj::load_obj_buf(
        &mut obj_reader,
        &load_options,
        |_p| Ok((Vec::new(), Default::default()))
    );
    
    let (models, _materials) = result.expect("Fallo al cargar el archivo OBJ");

    println!("Modelo cargado con {} mallas", models.len());

    // Calcular centro y escala del modelo para centrarlo y ajustarlo a la pantalla
    let mut sum = Vec3::ZERO;
    let mut vcount: usize = 0;
    for model in &models {
        let pos = &model.mesh.positions;
        for i in (0..pos.len()).step_by(3) {
            sum += Vec3::new(pos[i], pos[i + 1], pos[i + 2]);
            vcount += 1;
        }
    }
    let centroid = if vcount > 0 { sum / vcount as f32 } else { Vec3::ZERO };

    let mut max_r = 0.0f32;
    for model in &models {
        let pos = &model.mesh.positions;
        for i in (0..pos.len()).step_by(3) {
            let v = Vec3::new(pos[i], pos[i + 1], pos[i + 2]);
            let d = (v - centroid).length();
            if d > max_r { max_r = d; }
        }
    }
    let model_scale = if max_r > 0.0 { 1.5 / max_r } else { 1.0 };
    let model_translation = -centroid;

    let mut event_pump = sdl_context.event_pump()?;
    
    // Distancia inicial de la cámara y ángulo de rotación manual
    let mut camera_distance = 10.0;
    let mut rotation_y = 0.0_f32;

    'running: loop {
        // Manejo de eventos (como cerrar la ventana)
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::W), .. } | 
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                    // Acercarse (W o flecha arriba)
                    camera_distance = (camera_distance - 0.5_f32).max(3.0);
                },
                Event::KeyDown { keycode: Some(Keycode::S), .. } | 
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                    // Alejarse (S o flecha abajo)
                    camera_distance = (camera_distance + 0.5_f32).min(30.0);
                },
                Event::KeyDown { keycode: Some(Keycode::A), .. } | 
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                    // Rotar a la izquierda (A o flecha izquierda)
                    rotation_y -= 0.1;
                },
                Event::KeyDown { keycode: Some(Keycode::D), .. } | 
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                    // Rotar a la derecha (D o flecha derecha)
                    rotation_y += 0.1;
                },
                _ => {}
            }
        }

        // Limpia la pantalla con color negro
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        
        // Crear un z-buffer compartido para todas las mallas
        let mut zbuffer = ZBuffer::new(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);
        
        // Renderizamos cada malla con su índice para determinar el color
        for (idx, model) in models.iter().enumerate() {
            render(&mut canvas, &mut zbuffer, model, camera_distance, model_translation, model_scale, rotation_y, idx);
        }

        // Muestra el contenido del buffer en la pantalla
        canvas.present();
    }

    Ok(())
}
