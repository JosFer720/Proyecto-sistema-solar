extern crate sdl2;
extern crate tobj;
extern crate glam;

mod framebuffer;
mod shader_type;
mod utils;
mod shaders;
mod renderer;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::rect::Point;
use glam::{Vec3, Mat4};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::time::Instant;
use std::fs;
use std::io::Cursor;

use framebuffer::ZBuffer;
use shader_type::ShaderType;
use renderer::{SCREEN_WIDTH, SCREEN_HEIGHT, render, render_with_full_rotation};

fn main() -> Result<(), String> {
    // Inicializa SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // Crea la ventana
    let window = video_subsystem.window("Sistema Solar - Lab de Shaders", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    // Crea un canvas para dibujar
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    // Generar campo de estrellas (skybox procedural) una sola vez
    const STAR_COUNT: usize = 800;
    let mut rng = StdRng::seed_from_u64(0xC0FFEE);
    let mut stars: Vec<(i32, i32, u8)> = Vec::with_capacity(STAR_COUNT);
    for _ in 0..STAR_COUNT {
        let x = rng.gen_range(0..SCREEN_WIDTH as i32);
        let y = rng.gen_range(0..SCREEN_HEIGHT as i32);
        let b = rng.gen_range(120..256) as u8; // brillo
        stars.push((x, y, b));
    }

    // ===== CARGA DEL SOL (sphere.obj) =====
    let sun_content = fs::read_to_string("sphere.obj")
        .expect("No se pudo leer sphere.obj");
    
    let filtered_sun: String = sun_content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("mtllib") && !trimmed.starts_with("usemtl")
        })
        .collect::<Vec<&str>>()
        .join("\n");
    
    let mut sun_reader = Cursor::new(filtered_sun.as_bytes());
    
    let load_options = tobj::LoadOptions {
        triangulate: true,
        ignore_points: true,
        ignore_lines: true,
        ..Default::default()
    };
    
    let result = tobj::load_obj_buf(
        &mut sun_reader,
        &load_options,
        |_p| Ok((Vec::new(), Default::default()))
    );
    
    let (sun_models, _) = result.expect("Fallo al cargar sphere.obj");
    println!("Sol cargado con {} mallas", sun_models.len());

    // ===== CARGA DEL PLANETA ROCOSO (sphere.obj reutilizado) =====
    let rocky_content = fs::read_to_string("sphere.obj")
        .expect("No se pudo leer sphere.obj para el planeta rocoso");
    
    let filtered_rocky: String = rocky_content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("mtllib") && !trimmed.starts_with("usemtl")
        })
        .collect::<Vec<&str>>()
        .join("\n");
    
    let mut rocky_reader = Cursor::new(filtered_rocky.as_bytes());
    
    let result = tobj::load_obj_buf(
        &mut rocky_reader,
        &load_options,
        |_p| Ok((Vec::new(), Default::default()))
    );
    
    let (rocky_models, _) = result.expect("Fallo al cargar sphere.obj para planeta rocoso");
    println!("Planeta rocoso cargado con {} mallas", rocky_models.len());

    // ===== CARGA DE LA NAVE =====
    let spaceship_content = fs::read_to_string("Spaceship/Spaceship.obj")
        .expect("No se pudo leer Spaceship.obj");
    
    let filtered_spaceship: String = spaceship_content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("mtllib") && !trimmed.starts_with("usemtl")
        })
        .collect::<Vec<&str>>()
        .join("\n");
    
    let mut spaceship_reader = Cursor::new(filtered_spaceship.as_bytes());
    
    let result = tobj::load_obj_buf(
        &mut spaceship_reader,
        &load_options,
        |_p| Ok((Vec::new(), Default::default()))
    );
    
    let (spaceship_models, _) = result.expect("Fallo al cargar Spaceship.obj");
    println!("Nave cargada con {} mallas", spaceship_models.len());
    
    // Calcular centro y escala de la nave
    let mut ship_sum = Vec3::ZERO;
    let mut ship_vcount: usize = 0;
    for model in &spaceship_models {
        let pos = &model.mesh.positions;
        for i in (0..pos.len()).step_by(3) {
            ship_sum += Vec3::new(pos[i], pos[i + 1], pos[i + 2]);
            ship_vcount += 1;
        }
    }
    let ship_centroid = if ship_vcount > 0 { ship_sum / ship_vcount as f32 } else { Vec3::ZERO };

    let mut ship_max_r = 0.0f32;
    for model in &spaceship_models {
        let pos = &model.mesh.positions;
        for i in (0..pos.len()).step_by(3) {
            let v = Vec3::new(pos[i], pos[i + 1], pos[i + 2]);
            let d = (v - ship_centroid).length();
            if d > ship_max_r { ship_max_r = d; }
        }
    }
    let ship_scale = if ship_max_r > 0.0 { 2.5 / ship_max_r } else { 1.0 };
    let ship_translation = -ship_centroid;

    // Calcular centro y escala del SOL
    let mut sum = Vec3::ZERO;
    let mut vcount: usize = 0;
    for model in &sun_models {
        let pos = &model.mesh.positions;
        for i in (0..pos.len()).step_by(3) {
            sum += Vec3::new(pos[i], pos[i + 1], pos[i + 2]);
            vcount += 1;
        }
    }
    let sun_centroid = if vcount > 0 { sum / vcount as f32 } else { Vec3::ZERO };

    let mut max_r = 0.0f32;
    for model in &sun_models {
        let pos = &model.mesh.positions;
        for i in (0..pos.len()).step_by(3) {
            let v = Vec3::new(pos[i], pos[i + 1], pos[i + 2]);
            let d = (v - sun_centroid).length();
            if d > max_r { max_r = d; }
        }
    }
    // Doblar escalas: multiplicar por 2 los factores para todos los cuerpos
    let sun_scale = if max_r > 0.0 { 16.0 / max_r } else { 1.0 }; // Sol (antes 8.0, ahora 16.0)
    let sun_translation = -sun_centroid;

    // ConfiguraciÃ³n del planeta rocoso (Tierra) - MISMO TAMAÃ‘O QUE ANTES
    let rocky_scale = if max_r > 0.0 { 4.0 / max_r } else { 1.0 }; // Tierra (antes 2.0 -> ahora 4.0)

    let mut event_pump = sdl_context.event_pump()?;
    // Activar modo relativo del ratÃ³n para control tipo "mouselook"
    let mouse_subsystem = sdl_context.mouse();
    let _ = mouse_subsystem.set_relative_mouse_mode(true);
    // Track time for smooth movement
    let mut last_instant = Instant::now();
    
    // ===== SISTEMA DE CÃMARA LIBRE =====
    // CÃ¡mara posicionada mÃ¡s lejos para evitar colisiones iniciales tras cambios de escala
    let mut camera_position = Vec3::new(0.0, 60.0, 400.0); // Alejada del sol para permitir WASD
    let camera_yaw = 0.0f32; // Mirando hacia adelante (no usado para yaw, solo para direcciÃ³n inicial)
    let mut camera_pitch = 0.0f32; // Horizonte
    
    // ===== ROTACIÃ“N DEL SOL =====
    let mut sun_rotation = 0.0_f32;
    
    // ===== ROTACIÃ“N Y TRASLACIÃ“N DEL PLANETA ROCOSO (TIERRA) =====
    let mut rocky_rotation = 0.0_f32;      // RotaciÃ³n sobre su eje
    let mut rocky_orbit_angle = 0.0_f32;   // Ãngulo orbital alrededor del sol
    // Aumentado 200% adicional (triplicado) respecto al valor previo
    let rocky_orbit_radius = 45.0_f32 * 3.0;     // antes 45.0 -> ahora 135.0
    let rocky_orbit_speed = 0.006_f32;     // Velocidad orbital ajustada
    let rocky_rotation_speed = 0.01_f32;   // Velocidad de rotaciÃ³n sobre su eje (mÃ¡s visible)
    
    // ===== ROTACIÃ“N Y TRASLACIÃ“N DE VENUS =====
    let mut venus_rotation = 0.0_f32;      // RotaciÃ³n sobre su eje (muy lenta y retrÃ³grada)
    let mut venus_orbit_angle = std::f32::consts::PI * 0.5; // PosiciÃ³n inicial diferente
    let venus_orbit_radius = 33.0_f32 * 3.0;     // antes 33.0 -> ahora 99.0
    let venus_orbit_speed = 0.008_f32;     // MÃ¡s rÃ¡pido que la Tierra (mÃ¡s cerca del sol)
    let venus_rotation_speed = -0.002_f32; // RotaciÃ³n retrÃ³grada (negativa) y muy lenta
    let venus_scale = if max_r > 0.0 { 3.8 / max_r } else { 1.0 }; // Casi del tamaÃ±o de la Tierra (doble)
    
    // ===== ROTACIÃ“N Y TRASLACIÃ“N DE MARTE =====
    let mut mars_rotation = 0.0_f32;       // RotaciÃ³n sobre su eje
    let mut mars_orbit_angle = std::f32::consts::PI; // Empezar en lado opuesto
    let mars_orbit_radius = 60.0_f32 * 3.0;      // antes 60.0 -> ahora 180.0
    let mars_orbit_speed = 0.004_f32;      // MÃ¡s lento que la Tierra (mÃ¡s lejos del sol)
    let mars_rotation_speed = 0.0098_f32;  // RotaciÃ³n similar a la Tierra
    let mars_scale = if max_r > 0.0 { 3.0 / max_r } else { 1.0 }; // MÃ¡s pequeÃ±o que la Tierra (doble)
    
    // ===== ROTACIÃ“N Y TRASLACIÃ“N DE JÃšPITER (GIGANTE GASEOSO) =====
    let mut jupiter_rotation = 0.0_f32;    // RotaciÃ³n sobre su eje (muy rÃ¡pida)
    let mut jupiter_orbit_angle = std::f32::consts::PI * 1.5; // PosiciÃ³n inicial
    let jupiter_orbit_radius = 82.5_f32 * 3.0;   // antes 82.5 -> ahora 247.5
    let jupiter_orbit_speed = 0.002_f32;   // Muy lento (mÃ¡s lejos del sol)
    let jupiter_rotation_speed = 0.02_f32; // RotaciÃ³n rÃ¡pida (JÃºpiter rota en ~10 horas)
    let jupiter_scale = if max_r > 0.0 { 8.0 / max_r } else { 1.0 }; // Mitad del tamaÃ±o del Sol (doble)
    
    // ===== ROTACIÃ“N Y TRASLACIÃ“N DE URANO (GIGANTE DE HIELO) =====
    let mut uranus_rotation = 0.0_f32;     // RotaciÃ³n sobre su eje
    let mut uranus_orbit_angle = std::f32::consts::PI * 0.3; // PosiciÃ³n inicial
    let uranus_orbit_radius = 105.0_f32 * 3.0;   // antes 105.0 -> ahora 315.0
    let uranus_orbit_speed = 0.0015_f32;   // Muy lento
    let uranus_rotation_speed = 0.015_f32; // RotaciÃ³n media
    let uranus_scale = if max_r > 0.0 { 6.0 / max_r } else { 1.0 }; // MÃ¡s pequeÃ±o que JÃºpiter (doble)
    
    // ===== ROTACIÃ“N Y TRASLACIÃ“N DE NEPTUNO (GIGANTE DE HIELO) =====
    let mut neptune_rotation = 0.0_f32;    // RotaciÃ³n sobre su eje
    let mut neptune_orbit_angle = std::f32::consts::PI * 0.8; // PosiciÃ³n inicial
    let neptune_orbit_radius = 127.5_f32 * 3.0;  // antes 127.5 -> ahora 382.5
    let neptune_orbit_speed = 0.001_f32;   // Muy muy lento (mÃ¡s lejano)
    let neptune_rotation_speed = 0.016_f32; // RotaciÃ³n media-rÃ¡pida
    let neptune_scale = if max_r > 0.0 { 5.6 / max_r } else { 1.0 }; // Similar a Urano (doble)
    
    // ===== ROTACIÃ“N Y TRASLACIÃ“N DE LA LUNA (SATÃ‰LITE DE LA TIERRA) =====
    let mut moon_rotation = 0.0_f32;       // RotaciÃ³n sobre su eje (acoplamiento de marea)
    let mut moon_orbit_angle = 0.0_f32;    // Ãngulo orbital alrededor de la Tierra
    let moon_orbit_radius = 5.0_f32 * 3.0;       // antes 5.0 -> ahora 15.0 (mantener proporcionalidad)
    let moon_orbit_speed = 0.05_f32;       // Velocidad orbital (completa Ã³rbita en ~2 minutos)
    let moon_rotation_speed = 0.05_f32;    // Misma que orbital (acoplamiento de marea - siempre muestra misma cara)
    let moon_scale = if max_r > 0.0 { 1.4 / max_r } else { 1.0 }; // TamaÃ±o apropiado (doble)
    
    // ===== TIEMPO PARA ANIMACIONES =====
    let mut time = 0.0f32;

    // ===== FPS COUNTER =====
    let mut frame_count = 0u32;
    let mut fps_timer = Instant::now();
    let mut current_fps = 0.0f32;

    'running: loop {
        // movement_delta se calcularÃ¡ despuÃ©s del bucle de eventos usando el estado del teclado

        // Manejo de eventos (como cerrar la ventana)
        // Acumulador de desplazamiento horizontal del ratÃ³n (pixels) por frame
        let mut mouse_dx = 0.0_f32;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::MouseMotion { xrel, yrel, .. } => {
                    // Acumular desplazamiento horizontal para movimiento lateral
                    mouse_dx += xrel as f32;
                    // Solo usar movimiento vertical del ratÃ³n para ajustar pitch (mouselook Y)
                    // Sensibilidad aumentada respecto a la anterior pero menor que el valor original
                    let look_sensitivity = 0.0020_f32; // ajustar segÃºn peticiÃ³n
                    camera_pitch += -(yrel as f32) * look_sensitivity; // invertir Y para control natural
                    // Limitar pitch para evitar invertir la cÃ¡mara
                    let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
                    if camera_pitch > max_pitch { camera_pitch = max_pitch; }
                    if camera_pitch < -max_pitch { camera_pitch = -max_pitch; }
                },
                _ => {}
            }
        }

        // Calcular dt desde el frame anterior y actualizar tiempo para animaciones
        let now = Instant::now();
        let dt = (now - last_instant).as_secs_f32();
        last_instant = now;
        // Evitar dt demasiado grande
        let dt = dt.min(0.05);
        time += dt;

        // Actualizar FPS counter
        frame_count += 1;
        let fps_elapsed = fps_timer.elapsed().as_secs_f32();
        if fps_elapsed >= 0.5 { // Actualizar cada 0.5 segundos
            current_fps = frame_count as f32 / fps_elapsed;
            frame_count = 0;
            fps_timer = Instant::now();
        }

        // Movimiento continuo con WASD + espacio/shift (mouse controla la orientaciÃ³n)
        let keystate = event_pump.keyboard_state();
        let speed = 20.0_f32; // unidades por segundo
        let vertical_speed = 10.0_f32;

        let forward = Vec3::new(
            camera_yaw.sin() * camera_pitch.cos(),
            camera_pitch.sin(),
            camera_yaw.cos() * camera_pitch.cos(),
        );
        let right = Vec3::new(camera_yaw.cos(), 0.0, -camera_yaw.sin());

        let mut mv = Vec3::ZERO;
        // Swap W and S per user request: W moves backward, S moves forward (inverse mapping)
        if keystate.is_scancode_pressed(Scancode::W) {
            mv -= forward * speed * dt; // W -> backward
        }
        if keystate.is_scancode_pressed(Scancode::S) {
            mv += forward * speed * dt; // S -> forward
        }
        if keystate.is_scancode_pressed(Scancode::A) {
            mv -= right * speed * dt;
        }
        if keystate.is_scancode_pressed(Scancode::D) {
            mv += right * speed * dt;
        }
        if keystate.is_scancode_pressed(Scancode::Space) {
            mv.y += vertical_speed * dt;
        }
        if keystate.is_scancode_pressed(Scancode::LShift) {
            mv.y -= vertical_speed * dt;
        }

        // AÃ±adir movimiento lateral controlado por el mouse (derecha/izquierda)
        let mouse_sensitivity = 0.08_f32; // unidades por pixel (aumentada para mover mÃ¡s rÃ¡pido al desplazarse)
        if mouse_dx.abs() > 0.0 {
            mv += right * (mouse_dx * mouse_sensitivity);
        }

        // Asignar movement_delta calculado
        let movement_delta = mv;
        
        // RotaciÃ³n automÃ¡tica del sol
        sun_rotation += 0.005;
        
        // Actualizar rotaciÃ³n del planeta rocoso (Tierra)
        rocky_rotation += rocky_rotation_speed;
        
        // Actualizar Ã³rbita del planeta rocoso
        rocky_orbit_angle += rocky_orbit_speed;
        
        // Calcular posiciÃ³n orbital del planeta rocoso (Tierra) centrada en el origen (sol)
        let rocky_position = Vec3::new(
            rocky_orbit_radius * rocky_orbit_angle.cos(),
            0.0,
            rocky_orbit_radius * rocky_orbit_angle.sin()
        );
        
        // Actualizar rotaciÃ³n de Venus (retrÃ³grada)
        venus_rotation += venus_rotation_speed;
        
        // Actualizar Ã³rbita de Venus
        venus_orbit_angle += venus_orbit_speed;
        
        // Calcular posiciÃ³n orbital de Venus
        let venus_position = Vec3::new(
            venus_orbit_radius * venus_orbit_angle.cos(),
            0.0,
            venus_orbit_radius * venus_orbit_angle.sin()
        );
        
        // Actualizar rotaciÃ³n de Marte
        mars_rotation += mars_rotation_speed;
        
        // Actualizar Ã³rbita de Marte
        mars_orbit_angle += mars_orbit_speed;
        
        // Calcular posiciÃ³n orbital de Marte
        let mars_position = Vec3::new(
            mars_orbit_radius * mars_orbit_angle.cos(),
            0.0,
            mars_orbit_radius * mars_orbit_angle.sin()
        );
        
        // Actualizar rotaciÃ³n de JÃºpiter (muy rÃ¡pida)
        jupiter_rotation += jupiter_rotation_speed;
        
        // Actualizar Ã³rbita de JÃºpiter
        jupiter_orbit_angle += jupiter_orbit_speed;
        
        // Calcular posiciÃ³n orbital de JÃºpiter
        let jupiter_position = Vec3::new(
            jupiter_orbit_radius * jupiter_orbit_angle.cos(),
            0.0,
            jupiter_orbit_radius * jupiter_orbit_angle.sin()
        );
        
        // Actualizar rotaciÃ³n de Urano
        uranus_rotation += uranus_rotation_speed;
        
        // Actualizar Ã³rbita de Urano
        uranus_orbit_angle += uranus_orbit_speed;
        
        // Calcular posiciÃ³n orbital de Urano
        let uranus_position = Vec3::new(
            uranus_orbit_radius * uranus_orbit_angle.cos(),
            0.0,
            uranus_orbit_radius * uranus_orbit_angle.sin()
        );
        
        // Actualizar rotaciÃ³n de Neptuno
        neptune_rotation += neptune_rotation_speed;
        
        // Actualizar Ã³rbita de Neptuno
        neptune_orbit_angle += neptune_orbit_speed;
        
        // Calcular posiciÃ³n orbital de Neptuno
        let neptune_position = Vec3::new(
            neptune_orbit_radius * neptune_orbit_angle.cos(),
            0.0,
            neptune_orbit_radius * neptune_orbit_angle.sin()
        );
        
        // Actualizar rotaciÃ³n de la Luna (acoplamiento de marea)
        moon_rotation += moon_rotation_speed;
        
        // Actualizar Ã³rbita de la Luna alrededor de la Tierra
        moon_orbit_angle += moon_orbit_speed;
        
        // Calcular posiciÃ³n de la Luna RELATIVA A LA TIERRA (Ã³rbita circular)
        let moon_relative_position = Vec3::new(
            moon_orbit_radius * moon_orbit_angle.cos(),
            0.0,
            moon_orbit_radius * moon_orbit_angle.sin()
        );

        // Aplicar movimiento acumulado `movement_delta` con comprobaciÃ³n de colisiones
        // Primero calculamos posiciones relevantes (las Ã³rbitas ya fueron calculadas arriba)

        // Posiciones en el mundo de cada cuerpo
        let sun_center = Vec3::ZERO;
        let earth_center = rocky_position; // Tierra
        let venus_center = venus_position;
        let mars_center = mars_position;
        let jupiter_center = jupiter_position;
        let uranus_center = uranus_position;
        let neptune_center = neptune_position;
        let moon_center = earth_center + moon_relative_position;

        // CÃ¡mara: radio de colisiÃ³n (tolerancia)
        let camera_radius = 1.0_f32;

        // FunciÃ³n helper inline para probar colisiÃ³n con una esfera
        let collides = |pos: Vec3, radius: f32, target: Vec3| -> bool {
            (pos - target).length() < (radius + camera_radius)
        };

        // Proyecto la nueva posiciÃ³n
        let proposed = camera_position + movement_delta;

        // Lista de cuerpos con su radio aproximado (scale * max_r)
        let mut collision = false;

        // Nota: `max_r` es el radio del modelo original calculado al inicio de `main`.
        let bodies = [
            (sun_center, sun_scale),
            (earth_center, rocky_scale),
            (venus_center, venus_scale),
            (mars_center, mars_scale),
            (jupiter_center, jupiter_scale),
            (uranus_center, uranus_scale),
            (neptune_center, neptune_scale),
            (moon_center, moon_scale),
        ];

        for (center, scale) in bodies.iter() {
            let radius_world = *scale * max_r;
            if collides(*center, radius_world, proposed) {
                collision = true;
                break;
            }
        }

        if !collision {
            camera_position = proposed;
        }

        // Calcular el objetivo de la cÃ¡mara basado en yaw y pitch (despuÃ©s de moverla)
        let camera_target = camera_position + Vec3::new(
            camera_yaw.sin() * camera_pitch.cos(),
            camera_pitch.sin(),
            camera_yaw.cos() * camera_pitch.cos()
        );

        // Limpia la pantalla con color negro (espacio)
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        // Dibujar skybox procedural: estrellas
        for (x, y, b) in stars.iter() {
            canvas.set_draw_color(Color::RGB(*b, *b, *b));
            let _ = canvas.draw_point(Point::new(*x, *y));
        }

        // Dibujar Ã³rbitas proyectadas en pantalla
        {
            let projection = Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32, 1.0, 50000.0);
            let view = Mat4::look_at_rh(camera_position, camera_target, Vec3::Y);

            let mut draw_orbit = |radius: f32, center: Vec3, col: Color| {
                let segments = 128usize;
                let mut prev: Option<(i32,i32)> = None;
                for i in 0..=segments {
                    let theta = i as f32 / segments as f32 * std::f32::consts::TAU;
                    let world_point = center + Vec3::new(radius * theta.cos(), 0.0, radius * theta.sin());
                    let p = projection * view * world_point.extend(1.0);
                    // Skip extreme / behind-camera projections to avoid giant lines when very close
                    if p.w.abs() < 1e-4 { prev = None; continue; }
                    let nx = p.x / p.w;
                    let ny = p.y / p.w;
                    // If normalized coords are absurdly large, skip to avoid artifacts
                    if nx.abs() > 100.0 || ny.abs() > 100.0 { prev = None; continue; }
                    // Map to screen
                    let sx = ((nx + 1.0) * 0.5 * SCREEN_WIDTH as f32) as i32;
                    let sy = ((1.0 - (ny + 1.0) * 0.5) * SCREEN_HEIGHT as f32) as i32;
                    if let Some((px, py)) = prev {
                        // Only draw if both points are reasonably on/near screen bounds
                        if (px >= -200 && px <= SCREEN_WIDTH as i32 + 200) && (py >= -200 && py <= SCREEN_HEIGHT as i32 + 200) &&
                           (sx >= -200 && sx <= SCREEN_WIDTH as i32 + 200) && (sy >= -200 && sy <= SCREEN_HEIGHT as i32 + 200) {
                            let _ = canvas.set_draw_color(col);
                            let _ = canvas.draw_line(Point::new(px, py), Point::new(sx, sy));
                        }
                    }
                    prev = Some((sx, sy));
                }
            };

            // Ã“rbitas de los planetas alrededor del Sol
            draw_orbit(rocky_orbit_radius, Vec3::ZERO, Color::RGB(90, 90, 90));
            draw_orbit(venus_orbit_radius, Vec3::ZERO, Color::RGB(90, 80, 70));
            draw_orbit(mars_orbit_radius, Vec3::ZERO, Color::RGB(100, 60, 60));
            draw_orbit(jupiter_orbit_radius, Vec3::ZERO, Color::RGB(80, 80, 100));
            draw_orbit(uranus_orbit_radius, Vec3::ZERO, Color::RGB(70, 90, 100));
            draw_orbit(neptune_orbit_radius, Vec3::ZERO, Color::RGB(60, 80, 120));

            // Ã“rbita de la Luna alrededor de la Tierra
            draw_orbit(moon_orbit_radius, earth_center, Color::RGB(120, 120, 120));
        }

        // Crear un z-buffer compartido para todos los objetos
        let mut zbuffer = ZBuffer::new(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);
        
        // ===== RENDERIZAR EL SOL =====
        for model in sun_models.iter() {
            render_with_full_rotation(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                Vec3::ZERO,      // El sol estÃ¡ en el origen del mundo
                sun_translation, // Centrado del modelo del sol
                sun_scale,
                Vec3::new(0.0, sun_rotation, 0.0), // RotaciÃ³n como Vec3
                ShaderType::Sun,
                time
            );
        }
        
        // ===== RENDERIZAR PLANETA ROCOSO (TIERRA) =====
        for model in rocky_models.iter() {
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                rocky_position,  // PosiciÃ³n orbital de la Tierra
                sun_translation, // Usar el mismo centrado que el sol (misma geometrÃ­a)
                rocky_scale, 
                rocky_rotation,  // RotaciÃ³n sobre su eje
                ShaderType::RockyPlanet,
                time
            );
        }
        
        // ===== RENDERIZAR LA LUNA (SATÃ‰LITE DE LA TIERRA) =====
        for model in rocky_models.iter() {
            // La Luna orbita la Tierra. La Tierra estÃ¡ en rocky_position
            // La Luna estÃ¡ a moon_relative_position de la Tierra
            let moon_world_position = rocky_position + moon_relative_position;
            
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                moon_world_position, // PosiciÃ³n de la Luna en el mundo
                sun_translation,     // Usar el mismo centrado (misma geometrÃ­a)
                moon_scale, 
                moon_rotation,  // RotaciÃ³n (acoplamiento de marea)
                ShaderType::Moon,
                time
            );
        }
        
        // ===== RENDERIZAR VENUS =====
        for model in rocky_models.iter() {
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                venus_position,  // PosiciÃ³n orbital de Venus
                sun_translation, // Usar el mismo centrado (misma geometrÃ­a)
                venus_scale, 
                venus_rotation,  // RotaciÃ³n sobre su eje (retrÃ³grada)
                ShaderType::Venus,
                time
            );
        }
        
        // ===== RENDERIZAR MARTE =====
        for model in rocky_models.iter() {
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                mars_position,   // PosiciÃ³n orbital de Marte
                sun_translation, // Usar el mismo centrado (misma geometrÃ­a)
                mars_scale, 
                mars_rotation,   // RotaciÃ³n sobre su eje
                ShaderType::Mars,
                time
            );
        }
        
        // ===== RENDERIZAR JÃšPITER (GIGANTE GASEOSO) =====
        for model in rocky_models.iter() {
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                jupiter_position, // PosiciÃ³n orbital de JÃºpiter
                sun_translation,  // Usar el mismo centrado (misma geometrÃ­a)
                jupiter_scale, 
                jupiter_rotation, // RotaciÃ³n sobre su eje (muy rÃ¡pida)
                ShaderType::Jupiter,
                time
            );
        }
        
        // ===== RENDERIZAR URANO (GIGANTE DE HIELO) =====
        for model in rocky_models.iter() {
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                uranus_position, // PosiciÃ³n orbital de Urano
                sun_translation, // Usar el mismo centrado (misma geometrÃ­a)
                uranus_scale, 
                uranus_rotation, // RotaciÃ³n sobre su eje
                ShaderType::Uranus,
                time
            );
        }
        
        // ===== RENDERIZAR NEPTUNO (GIGANTE DE HIELO) =====
        for model in rocky_models.iter() {
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                neptune_position, // PosiciÃ³n orbital de Neptuno
                sun_translation,  // Usar el mismo centrado (misma geometrÃ­a)
                neptune_scale, 
                neptune_rotation, // RotaciÃ³n sobre su eje
                ShaderType::Neptune,
                time
            );
        }
        
        // ===== RENDERIZAR LA NAVE (SIEMPRE ENFRENTE DE LA CÃMARA) =====
        // Calcular posiciÃ³n de la nave: adelante de la cÃ¡mara
        let ship_forward_distance = 8.0; // Distancia delante de la cÃ¡mara
        let ship_down_offset = -1.5;     // Offset hacia abajo para que no tape la vista
        let ship_right_offset = 0.0;     // Sin offset lateral por defecto
        
        let up = Vec3::Y;
        let ship_position = camera_position 
            + forward * ship_forward_distance 
            + up * ship_down_offset
            + right * ship_right_offset;
        
        // RotaciÃ³n completa de la nave para que apunte exactamente donde mira la cÃ¡mara
        // Pitch (X): positivo para que la nariz suba/baje correctamente con la cÃ¡mara
        // Yaw (Y): rotaciÃ³n horizontal
        // Roll (Z): mantener en 0 para no inclinar lateralmente
        let ship_rotation = Vec3::new(
            camera_pitch,   // Pitch - apunta arriba/abajo (sin invertir)
            camera_yaw,     // Yaw - apunta izquierda/derecha
            0.0             // Roll - sin inclinaciÃ³n lateral
        );
        
        for model in spaceship_models.iter() {
            render_with_full_rotation(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                ship_position,      // PosiciÃ³n relativa a la cÃ¡mara
                ship_translation,   // Centrado del modelo de la nave
                ship_scale,         // Escala de la nave
                ship_rotation,      // RotaciÃ³n completa en 3 ejes
                ShaderType::Spaceship,
                time
            );
        }

        // ===== RENDERIZAR FPS COUNTER =====
        // Dibujar FPS en esquina superior derecha con píxeles (fuente bitmap simple)
        canvas.set_draw_color(Color::RGB(0, 255, 0));
        let fps_value = current_fps as u32;
        let fps_text = format!("FPS:{}", fps_value);
        
        // Posición inicial (esquina superior derecha)
        let mut x_pos = SCREEN_WIDTH as i32 - 10;
        
        // Dibujar cada carácter de derecha a izquierda
        for ch in fps_text.chars().rev() {
            x_pos -= 6; // Ancho de carácter + espacio
            let y_pos = 10;
            
            // Dibujar dígitos y letras de forma simple (5x7 pixels)
            match ch {
                '0' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if (dy == 0 || dy == 6) && dx > 0 && dx < 4 ||
                               (dx == 0 || dx == 4) && dy > 0 && dy < 6 {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                '1' => {
                    for dy in 0..7 {
                        let _ = canvas.draw_point(Point::new(x_pos + 2, y_pos + dy));
                        if dy == 1 { let _ = canvas.draw_point(Point::new(x_pos + 1, y_pos + dy)); }
                        if dy == 6 { 
                            let _ = canvas.draw_point(Point::new(x_pos + 1, y_pos + dy));
                            let _ = canvas.draw_point(Point::new(x_pos + 3, y_pos + dy));
                        }
                    }
                },
                '2' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dy == 0 || dy == 3 || dy == 6 ||
                               (dx == 4 && dy < 3) || (dx == 0 && dy > 3) {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                '3' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dy == 0 || dy == 3 || dy == 6 || dx == 4 {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                '4' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dx == 0 && dy < 4 || dy == 3 || dx == 4 {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                '5' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dy == 0 || dy == 3 || dy == 6 ||
                               (dx == 0 && dy < 3) || (dx == 4 && dy > 3) {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                '6' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dy == 0 || dy == 3 || dy == 6 ||
                               dx == 0 || (dx == 4 && dy > 3) {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                '7' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dy == 0 || (dx == 4 && dy > 0) {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                '8' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dy == 0 || dy == 3 || dy == 6 ||
                               dx == 0 || dx == 4 {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                '9' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dy == 0 || dy == 3 || dy == 6 ||
                               dx == 4 || (dx == 0 && dy < 4) {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                'F' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dx == 0 || dy == 0 || (dy == 3 && dx < 4) {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                'P' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dx == 0 || dy == 0 || (dy == 3 && dx < 4) || (dx == 4 && dy < 4) {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                'S' => {
                    for dy in 0..7 {
                        for dx in 0..5 {
                            if dy == 0 || dy == 3 || dy == 6 ||
                               (dx == 0 && (dy < 3 || dy == 6)) || (dx == 4 && (dy > 3 || dy == 0)) {
                                let _ = canvas.draw_point(Point::new(x_pos + dx, y_pos + dy));
                            }
                        }
                    }
                },
                ':' => {
                    let _ = canvas.draw_point(Point::new(x_pos + 2, y_pos + 2));
                    let _ = canvas.draw_point(Point::new(x_pos + 2, y_pos + 4));
                },
                _ => {} // Ignorar otros caracteres
            }
        }

        // Muestra el contenido del buffer en la pantalla
        canvas.present();
    }

    Ok(())
}
