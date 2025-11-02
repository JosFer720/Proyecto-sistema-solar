extern crate sdl2;
extern crate tobj;
extern crate glam;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Point;
use glam::{Vec3, Mat4};
use std::path::Path;
use std::time::Instant;
use std::fs::File;
use std::io::BufReader;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

/// Renderiza un modelo 3D en el canvas.
/// Dibuja el modelo en modo wireframe (solo las aristas de los triángulos).
fn render(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, model: &tobj::Model, rotation_angle: f32) {
    let positions = &model.mesh.positions;
    let indices = &model.mesh.indices;

    // Matrices de transformación para pasar de coordenadas 3D a 2D
    let projection = Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at_rh(
        Vec3::new(0.0, 0.0, 5.0), // Posición de la cámara
        Vec3::ZERO,              // Hacia dónde mira la cámara
        Vec3::Y,                 // Vector "arriba"
    );
    
    // Matriz del modelo para rotar el objeto
    let model_matrix = Mat4::from_rotation_y(rotation_angle) * Mat4::from_rotation_x(rotation_angle * 0.5);

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

        // Dibuja las líneas del triángulo
        canvas.draw_line(screen_p0, screen_p1).unwrap();
        canvas.draw_line(screen_p1, screen_p2).unwrap();
        canvas.draw_line(screen_p2, screen_p0).unwrap();
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
    // Usamos un reader personalizado para ignorar completamente los materiales
    let obj_file = File::open("Spaceship/Spaceship.obj")
        .expect("No se pudo abrir el archivo OBJ");
    let mut obj_reader = BufReader::new(obj_file);
    
    let load_options = tobj::LoadOptions {
        triangulate: true,
        ignore_points: true,
        ignore_lines: true,
        ..Default::default()
    };
    
    // Cargamos usando load_obj_buf con un closure que ignora los errores de materiales
    let result = tobj::load_obj_buf(
        &mut obj_reader,
        &load_options,
        |_p| {
            // Retornamos un resultado vacío para ignorar los materiales completamente  
            Ok((Vec::new(), Default::default()))
        }
    );
    
    let (models, _materials) = result.expect("Fallo al cargar el archivo OBJ");

    println!("Modelo cargado con {} mallas", models.len());

    // Usaremos solo el primer modelo del archivo
    let first_model = &models[0];

    let mut event_pump = sdl_context.event_pump()?;
    let start_time = Instant::now();

    'running: loop {
        // Manejo de eventos (como cerrar la ventana)
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

        // Limpia la pantalla con color negro
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        // Establece el color para dibujar el modelo (amarillo)
        canvas.set_draw_color(Color::RGB(255, 255, 0));
        
        // Calcula el ángulo de rotación basado en el tiempo para animar el cubo
        let elapsed = start_time.elapsed().as_secs_f32();
        
        // Llama a la función de renderizado
        render(&mut canvas, first_model, elapsed);

        // Muestra el contenido del buffer en la pantalla
        canvas.present();
    }

    Ok(())
}
