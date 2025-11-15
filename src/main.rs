extern crate sdl2;
extern crate tobj;
extern crate glam;

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


/// Matriz 4x4 de transformación del modelo
fn create_model_matrix(world_translation: Vec3, model_center: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let center_matrix = Mat4::from_translation(model_center);
    
    let scale_matrix = Mat4::from_scale(Vec3::splat(scale));
    

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

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum ShaderType {
    Sun,
    RockyPlanet,  // Planeta rocoso tipo Tierra
    Venus,        // Planeta Venus - amarillo/naranja con atmósfera densa
    Mars,         // Planeta Marte - rojo/oxidado
    Moon,         // Luna de la Tierra - gris rocoso con cráteres
    Jupiter,      // Júpiter - gigante gaseoso con bandas
    Uranus,       // Urano - gigante de hielo azul-verde
    Neptune,      // Neptuno - gigante de hielo azul oscuro
    Spaceship,    // Para mantener la nave comentada pero funcional
}

struct ZBuffer {
    buffer: Vec<f32>,
    width: usize,
    height: usize,
}

#[allow(dead_code)]
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

fn hash(x: f32, y: f32) -> f32 {
    let n = (x * 12.9898 + y * 78.233).sin() * 43758.5453;
    n - n.floor()
}

fn noise(x: f32, y: f32) -> f32 {
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

fn apply_shader(
    shader_type: ShaderType,
    vertex_position: Vec3,  // Posición del vértice en espacio del modelo
    normal: Vec3,           // Normal del triángulo
    intensity: f32,         // Intensidad de luz base
    time: f32,              // Tiempo para animaciones
) -> (u8, u8, u8) {
    match shader_type {
        ShaderType::Sun => {
            // ===== SHADER DEL SOL =====
            // Sistema de 5 capas para máxima complejidad
            
            let position = vertex_position;
            
            // Convertir a coordenadas esféricas para patterns
            let radius = (position.x * position.x + position.y * position.y + position.z * position.z).sqrt();
            let theta = position.y.atan2((position.x * position.x + position.z * position.z).sqrt()); // latitud
            let phi = position.z.atan2(position.x); // longitud
            
            // CAPA 1: Color base - Gradiente de núcleo (amarillo-naranja-rojo)
            let core_distance = radius.max(0.01);
            let core_gradient = (1.0 - (core_distance / 2.0).min(1.0)).max(0.0);
            
            let base_r = 255.0;
            let base_g = 180.0 + core_gradient * 50.0;
            let base_b = 20.0 + core_gradient * 30.0;
            
            // CAPA 2: Manchas solares (patrones oscuros)
            let spot_freq = 3.0;
            let spot_noise = noise(phi * spot_freq + time * 0.1, theta * spot_freq);
            let spot_noise2 = noise(phi * spot_freq * 2.0 - time * 0.15, theta * spot_freq * 2.0);
            let combined_spots = (spot_noise + spot_noise2 * 0.5) / 1.5;
            
            // Umbral para manchas oscuras
            let spot_factor = if combined_spots > 0.65 { 
                0.6 // Mancha oscura
            } else { 
                1.0 
            };
            
            // CAPA 3: Efecto de llamaradas (áreas más brillantes que se mueven)
            let flare_noise = noise(phi * 2.0 + time * 0.3, theta * 2.0 + time * 0.2);
            let flare_factor = if flare_noise > 0.7 {
                1.0 + (flare_noise - 0.7) * 2.0 // Brightening
            } else {
                1.0
            };
            
            // CAPA 4: Pulsación temporal (respiración del sol)
            let pulse = ((time * 2.0).sin() * 0.5 + 0.5) * 0.15 + 0.85; // Oscila entre 0.85 y 1.0
            
            // CAPA 5: Turbulencia en la superficie
            let turb_noise = noise(phi * 8.0 + time * 0.5, theta * 8.0 - time * 0.3);
            let turb_factor = 0.9 + turb_noise * 0.2; // Pequeñas variaciones
            
            // CAPA 6: Efecto de corona/borde brillante
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let corona = edge_factor * edge_factor * 0.3;
            
            // Combinar todas las capas
            let r = (base_r * spot_factor * flare_factor * pulse * turb_factor * (0.7 + 0.3 * intensity) + corona * 100.0).min(255.0);
            let g = (base_g * spot_factor * flare_factor * pulse * turb_factor * (0.7 + 0.3 * intensity) + corona * 80.0).min(255.0);
            let b = (base_b * spot_factor * pulse * turb_factor * (0.7 + 0.3 * intensity) + corona * 20.0).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::RockyPlanet => {
            // ===== SHADER DE PLANETA ROCOSO =====
            // Sistema de 5 capas multicolor tipo Tierra
            
            let position = vertex_position;
            
            // Convertir a coordenadas esféricas
            let theta = position.y.atan2((position.x * position.x + position.z * position.z).sqrt()); // latitud
            let phi = position.z.atan2(position.x); // longitud
            
            // Normalizar latitud a rango [0, 1]
            let lat = (theta / std::f32::consts::PI) + 0.5; // 0 = polo sur, 1 = polo norte, 0.5 = ecuador
            
            // CAPA 1: Base de terreno/océano
            // Usar noise para determinar tierra vs agua
            let land_noise = noise(phi * 5.0 + time * 0.05, theta * 5.0);
            let land_noise2 = noise(phi * 10.0 - time * 0.03, theta * 10.0 + 100.0);
            let is_land = (land_noise * 0.6 + land_noise2 * 0.4) > 0.48; // Más océano, menos tierra (como la Tierra real)
            
            // Colores realistas de la Tierra
            let ocean_color = (10.0, 50.0, 120.0);      // Azul océano profundo
            let shallow_ocean = (30.0, 80.0, 150.0);    // Azul océano poco profundo
            let land_color = (34.0, 139.0, 34.0);       // Verde bosque (forest green)
            let desert_color = (210.0, 180.0, 140.0);   // Beige/tan desierto
            let mountain_color = (139.0, 137.0, 137.0); // Gris montaña/roca
            
            let (mut base_r, mut base_g, mut base_b) = if is_land {
                // Variar el tipo de tierra según latitud y noise
                let terrain_variation = noise(phi * 3.0, theta * 3.0 + 50.0);
                
                if lat > 0.75 || lat < 0.25 {
                    // Zonas polares - hielo/nieve (se renderizará más adelante)
                    (240.0, 240.0, 255.0)
                } else if terrain_variation > 0.65 {
                    // Montañas y zonas rocosas
                    mountain_color
                } else if (lat > 0.35 && lat < 0.42) || (lat > 0.58 && lat < 0.65) {
                    // Desiertos subtropicales (Sahara, etc.)
                    desert_color
                } else {
                    // Tierra fértil - bosques y vegetación
                    let green_variation = terrain_variation * 20.0;
                    (
                        land_color.0 + green_variation,
                        land_color.1 + green_variation,
                        land_color.2 + green_variation * 0.5
                    )
                }
            } else {
                // Océano con variación de profundidad
                let depth_noise = noise(phi * 8.0, theta * 8.0 + 200.0);
                if depth_noise > 0.6 {
                    // Océano poco profundo (cerca de costas)
                    shallow_ocean
                } else {
                    // Océano profundo
                    let depth_factor = 0.8 + depth_noise * 0.2;
                    (
                        ocean_color.0 * depth_factor,
                        ocean_color.1 * depth_factor,
                        ocean_color.2 * depth_factor
                    )
                }
            };
            
            // CAPA 2: Casquetes polares (blanco brillante) - MÁS GRANDES Y REALISTAS
            let polar_threshold = 0.80; // Más grandes que antes (era 0.85)
            let polar_factor = if lat > polar_threshold {
                // Polo norte
                ((lat - polar_threshold) / (1.0 - polar_threshold)).powf(0.4) // Más suave
            } else if lat < (1.0 - polar_threshold) {
                // Polo sur
                ((1.0 - polar_threshold - lat) / (1.0 - polar_threshold)).powf(0.4)
            } else {
                0.0
            };
            
            if polar_factor > 0.0 {
                let snow_white = 255.0;
                let ice_blue_tint = 0.95; // Ligero tinte azul del hielo
                base_r = base_r * (1.0 - polar_factor) + snow_white * polar_factor;
                base_g = base_g * (1.0 - polar_factor) + snow_white * polar_factor;
                base_b = base_b * (1.0 - polar_factor) + (snow_white * ice_blue_tint) * polar_factor;
            }
            
            // CAPA 3: Nubes (blancas semi-transparentes)
            let cloud_noise1 = noise(phi * 6.0 + time * 0.3, theta * 6.0);
            let cloud_noise2 = noise(phi * 12.0 - time * 0.2, theta * 12.0 + 300.0);
            let cloud_combined = cloud_noise1 * 0.6 + cloud_noise2 * 0.4;
            
            let cloud_factor = if cloud_combined > 0.6 {
                ((cloud_combined - 0.6) / 0.4).min(1.0) * 0.7 // Opacidad máxima 70%
            } else {
                0.0
            };
            
            if cloud_factor > 0.0 {
                let cloud_white = 240.0;
                base_r = base_r * (1.0 - cloud_factor) + cloud_white * cloud_factor;
                base_g = base_g * (1.0 - cloud_factor) + cloud_white * cloud_factor;
                base_b = base_b * (1.0 - cloud_factor) + cloud_white * cloud_factor;
            }
            
            // CAPA 4: Atmósfera (brillo azul en los bordes)
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.25;
            
            base_r += atmosphere * 50.0;
            base_g += atmosphere * 100.0;
            base_b += atmosphere * 200.0;
            
            // CAPA 5: Variación de iluminación mejorada (día/noche más pronunciado)
            // La intensidad ya viene calculada desde el render
            let enhanced_intensity = intensity * 0.4 + 0.6; // Mínimo 60%, máximo 100%
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Venus => {
            // ===== SHADER DE VENUS - PLANETA CON ATMÓSFERA DENSA Y 4 CAPAS =====
            // Planeta amarillo/naranja con nubes densas y efecto invernadero
            
            let position = vertex_position;
            
            // Coordenadas esféricas para texturas procedurales
            let phi = position.x.atan2(position.z) + time * 0.15; // Rotación rápida de atmósfera
            let theta = (position.y / position.length()).acos();
            
            // CAPA 1: Color base - Atmósfera amarillo/naranja (ácido sulfúrico)
            let base_noise1 = noise(phi * 3.0, theta * 3.0);
            let base_noise2 = noise(phi * 6.0 + 50.0, theta * 6.0 + 50.0);
            let base_combined = base_noise1 * 0.6 + base_noise2 * 0.4;
            
            let mut base_r;
            let mut base_g;
            let mut base_b;
            
            if base_combined < 0.35 {
                // Áreas más oscuras - naranja oscuro
                base_r = 200.0;
                base_g = 140.0;
                base_b = 50.0;
            } else if base_combined < 0.7 {
                // Áreas principales - amarillo cremoso
                base_r = 230.0;
                base_g = 190.0;
                base_b = 80.0;
            } else {
                // Áreas brillantes - amarillo pálido
                base_r = 250.0;
                base_g = 220.0;
                base_b = 120.0;
            }
            
            // CAPA 2: Bandas atmosféricas horizontales (vientos super-rotación)
            let band_pattern = (theta * 8.0 + phi * 2.0 + time * 0.3).sin();
            let band_noise = noise(phi * 4.0 - time * 0.2, theta * 4.0);
            let band_factor = (band_pattern * 0.5 + 0.5) * (band_noise * 0.5 + 0.5);
            
            // Aplicar bandas más oscuras
            let band_darken = band_factor * 0.3;
            base_r -= band_darken * 80.0;
            base_g -= band_darken * 60.0;
            base_b -= band_darken * 30.0;
            
            // CAPA 3: Patrones de nubes arremolinadas (ácido sulfúrico)
            let cloud_noise1 = noise(phi * 5.0 + time * 0.4, theta * 5.0);
            let cloud_noise2 = noise(phi * 10.0 - time * 0.3, theta * 10.0 + 100.0);
            let cloud_combined = cloud_noise1 * 0.7 + cloud_noise2 * 0.3;
            
            let cloud_factor = if cloud_combined > 0.65 {
                ((cloud_combined - 0.65) / 0.35).min(1.0) * 0.5
            } else {
                0.0
            };
            
            if cloud_factor > 0.0 {
                // Nubes más brillantes y amarillentas
                base_r = base_r * (1.0 - cloud_factor) + 255.0 * cloud_factor;
                base_g = base_g * (1.0 - cloud_factor) + 235.0 * cloud_factor;
                base_b = base_b * (1.0 - cloud_factor) + 150.0 * cloud_factor;
            }
            
            // CAPA 4: Atmósfera densa y brillante (efecto invernadero)
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.4; // Muy brillante
            
            base_r += atmosphere * 150.0;
            base_g += atmosphere * 120.0;
            base_b += atmosphere * 50.0;
            
            // Variación de iluminación (Venus es muy reflectante)
            let enhanced_intensity = intensity * 0.3 + 0.7; // Mínimo 70%, muy brillante
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Mars => {
            // ===== SHADER DE MARTE - PLANETA ROJO CON 4 CAPAS =====
            // Planeta desértico rojizo/oxidado con variaciones de terreno
            
            let position = vertex_position;
            
            // Coordenadas esféricas para texturas procedurales
            let phi = position.x.atan2(position.z) + time * 0.05; // Rotación lenta de textura
            let theta = (position.y / position.length()).acos();
            
            // CAPA 1: Color base rojo/oxidado con variaciones de terreno
            let terrain_noise1 = noise(phi * 4.0, theta * 4.0);
            let terrain_noise2 = noise(phi * 8.0 + 100.0, theta * 8.0 + 100.0);
            let terrain_combined = terrain_noise1 * 0.6 + terrain_noise2 * 0.4;
            
            let mut base_r;
            let mut base_g;
            let mut base_b;
            
            if terrain_combined < 0.3 {
                // Regiones oscuras - roca volcánica/basalto
                base_r = 80.0;
                base_g = 40.0;
                base_b = 30.0;
            } else if terrain_combined < 0.7 {
                // Regiones principales - óxido de hierro (rojo Marte clásico)
                base_r = 193.0;
                base_g = 68.0;
                base_b = 14.0;
            } else {
                // Regiones claras - polvo/arena oxidada más clara
                base_r = 210.0;
                base_g = 105.0;
                base_b = 30.0;
            }
            
            // CAPA 2: Casquetes polares (hielo de CO2 y agua)
            let polar_threshold = 0.85;
            let polar_distance = theta.min(std::f32::consts::PI - theta) / std::f32::consts::PI;
            
            if polar_distance > polar_threshold {
                let polar_factor = ((polar_distance - polar_threshold) / (1.0 - polar_threshold)).min(1.0);
                let ice_white = 240.0;
                let ice_cream = 230.0; // Tinte amarillento del hielo marciano
                base_r = base_r * (1.0 - polar_factor) + ice_white * polar_factor;
                base_g = base_g * (1.0 - polar_factor) + ice_cream * polar_factor;
                base_b = base_b * (1.0 - polar_factor) + (ice_cream * 0.9) * polar_factor;
            }
            
            // CAPA 3: Tormentas de polvo (áreas blanquecinas/amarillentas)
            let dust_noise1 = noise(phi * 3.0 + time * 0.1, theta * 3.0);
            let dust_noise2 = noise(phi * 6.0 - time * 0.05, theta * 6.0 + 200.0);
            let dust_combined = dust_noise1 * 0.5 + dust_noise2 * 0.5;
            
            let dust_factor = if dust_combined > 0.75 {
                ((dust_combined - 0.75) / 0.25).min(1.0) * 0.4 // Opacidad máxima 40%
            } else {
                0.0
            };
            
            if dust_factor > 0.0 {
                let dust_yellow_r = 220.0;
                let dust_yellow_g = 180.0;
                let dust_yellow_b = 120.0;
                base_r = base_r * (1.0 - dust_factor) + dust_yellow_r * dust_factor;
                base_g = base_g * (1.0 - dust_factor) + dust_yellow_g * dust_factor;
                base_b = base_b * (1.0 - dust_factor) + dust_yellow_b * dust_factor;
            }
            
            // CAPA 4: Atmósfera delgada (brillo rojizo tenue en los bordes)
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.15; // Más sutil que la Tierra
            
            base_r += atmosphere * 100.0;
            base_g += atmosphere * 30.0;
            base_b += atmosphere * 10.0;
            
            // Variación de iluminación
            let enhanced_intensity = intensity * 0.5 + 0.5; // Mínimo 50%, máximo 100%
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Moon => {
            // ===== SHADER DE LA LUNA - SATÉLITE ROCOSO CON 4 CAPAS =====
            // Luna gris con cráteres, sin atmósfera
            
            let position = vertex_position;
            
            // Coordenadas esféricas para texturas procedurales
            let phi = position.x.atan2(position.z);
            let theta = (position.y / position.length()).acos();
            
            // CAPA 1: Color base gris con variaciones de terreno
            let terrain_noise1 = noise(phi * 6.0, theta * 6.0);
            let terrain_noise2 = noise(phi * 12.0 + 100.0, theta * 12.0 + 100.0);
            let terrain_combined = terrain_noise1 * 0.6 + terrain_noise2 * 0.4;
            
            let mut base_r;
            let mut base_g;
            let mut base_b;
            
            if terrain_combined < 0.3 {
                // Áreas oscuras - maria (mares lunares)
                base_r = 80.0;
                base_g = 80.0;
                base_b = 85.0;
            } else if terrain_combined < 0.7 {
                // Regiones principales - regolito gris
                base_r = 140.0;
                base_g = 140.0;
                base_b = 145.0;
            } else {
                // Regiones claras - tierras altas
                base_r = 180.0;
                base_g = 180.0;
                base_b = 185.0;
            }
            
            // CAPA 2: Cráteres (círculos oscuros con bordes)
            let crater_noise1 = noise(phi * 15.0, theta * 15.0);
            let crater_noise2 = noise(phi * 30.0 + 200.0, theta * 30.0 + 200.0);
            
            // Crear patrón de cráteres
            if crater_noise1 > 0.75 {
                let crater_depth = (crater_noise1 - 0.75) / 0.25;
                let crater_darken = crater_depth * 0.4;
                base_r -= crater_darken * 80.0;
                base_g -= crater_darken * 80.0;
                base_b -= crater_darken * 85.0;
            }
            
            if crater_noise2 > 0.8 {
                let crater_depth = (crater_noise2 - 0.8) / 0.2;
                let crater_darken = crater_depth * 0.3;
                base_r -= crater_darken * 60.0;
                base_g -= crater_darken * 60.0;
                base_b -= crater_darken * 65.0;
            }
            
            // CAPA 3: Variaciones de brillo (rayos de impacto)
            let ray_noise = noise(phi * 8.0 + theta * 8.0, theta * 4.0);
            if ray_noise > 0.7 {
                let ray_brightness = ((ray_noise - 0.7) / 0.3) * 0.2;
                base_r += ray_brightness * 100.0;
                base_g += ray_brightness * 100.0;
                base_b += ray_brightness * 105.0;
            }
            
            // CAPA 4: Sin atmósfera - contraste fuerte luz/sombra
            // La luna no tiene atmósfera, así que el contraste es muy marcado
            let harsh_intensity = if intensity > 0.5 {
                intensity * 0.8 + 0.2 // Lado iluminado
            } else {
                intensity * 0.3 // Lado oscuro muy oscuro
            };
            
            let r = (base_r * harsh_intensity).min(255.0);
            let g = (base_g * harsh_intensity).min(255.0);
            let b = (base_b * harsh_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Jupiter => {
            // ===== SHADER DE JÚPITER - GIGANTE GASEOSO CON BANDAS =====
            // Planeta gigante con bandas horizontales y la Gran Mancha Roja
            
            let position = vertex_position;
            
            // Coordenadas esféricas para texturas procedurales
            let phi = position.x.atan2(position.z) + time * 0.1; // Rotación rápida
            let theta = (position.y / position.length()).acos();
            
            // Normalizar latitud a rango [0, 1]
            let lat = theta / std::f32::consts::PI; // 0 = polo norte, 1 = polo sur
            
            // CAPA 1: Bandas horizontales base (colores naranja/beige/marrón)
            // Crear patrón de bandas con diferentes anchos
            let band_freq = 12.0; // Número de bandas
            let band_pattern = (lat * band_freq).sin();
            let band_noise = noise(phi * 2.0, lat * 15.0 + time * 0.05);
            
            let mut base_r;
            let mut base_g;
            let mut base_b;
            
            // Alternar entre bandas claras y oscuras
            if (band_pattern + band_noise * 0.3) > 0.0 {
                // Zonas ecuatoriales claras (beige/crema)
                base_r = 220.0 + band_noise * 20.0;
                base_g = 190.0 + band_noise * 20.0;
                base_b = 140.0 + band_noise * 15.0;
            } else {
                // Bandas oscuras (marrón/naranja)
                base_r = 180.0 + band_noise * 15.0;
                base_g = 130.0 + band_noise * 15.0;
                base_b = 80.0 + band_noise * 10.0;
            }
            
            // CAPA 2: Turbulencia en las bandas (remolinos)
            let turb_noise1 = noise(phi * 8.0 + time * 0.2, lat * 20.0);
            let turb_noise2 = noise(phi * 15.0 - time * 0.15, lat * 30.0 + 100.0);
            let turbulence = turb_noise1 * 0.6 + turb_noise2 * 0.4;
            
            // Agregar variación de turbulencia
            base_r += turbulence * 30.0 - 15.0;
            base_g += turbulence * 25.0 - 12.0;
            base_b += turbulence * 20.0 - 10.0;
            
            // CAPA 3: Gran Mancha Roja (óvalo rojizo en latitud media)
            let spot_lat_center = 0.6; // Latitud de la mancha (hemisferio sur)
            let spot_lon_center = std::f32::consts::PI * 0.5 + time * 0.02; // Rota lentamente
            
            // Calcular distancia a la mancha
            let lat_diff = (lat - spot_lat_center).abs();
            let lon_diff = (phi - spot_lon_center).abs().min(std::f32::consts::TAU - (phi - spot_lon_center).abs());
            
            // Mancha elíptica (más ancha que alta)
            let spot_distance = (lat_diff * lat_diff * 400.0 + lon_diff * lon_diff * 100.0).sqrt();
            
            if spot_distance < 1.5 {
                let spot_factor = (1.0 - spot_distance / 1.5).max(0.0);
                let spot_noise = noise(phi * 10.0 + time * 0.1, lat * 10.0);
                
                // Color rojizo/marrón de la mancha
                base_r = base_r * (1.0 - spot_factor * 0.8) + (200.0 + spot_noise * 20.0) * spot_factor * 0.8;
                base_g = base_g * (1.0 - spot_factor * 0.8) + (100.0 + spot_noise * 10.0) * spot_factor * 0.8;
                base_b = base_b * (1.0 - spot_factor * 0.8) + (80.0 + spot_noise * 10.0) * spot_factor * 0.8;
            }
            
            // CAPA 4: Zonas polares (más claras y azuladas)
            let polar_factor = if lat < 0.15 || lat > 0.85 {
                let dist = lat.min(1.0 - lat);
                (0.15 - dist) / 0.15
            } else {
                0.0
            };
            
            if polar_factor > 0.0 {
                base_r += polar_factor * 30.0;
                base_g += polar_factor * 40.0;
                base_b += polar_factor * 60.0;
            }
            
            // CAPA 5: Atmósfera brillante en los bordes
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.2;
            
            base_r += atmosphere * 80.0;
            base_g += atmosphere * 70.0;
            base_b += atmosphere * 50.0;
            
            // Variación de iluminación
            let enhanced_intensity = intensity * 0.4 + 0.6; // Mínimo 60%, máximo 100%
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Uranus => {
            // ===== SHADER DE URANO - GIGANTE DE HIELO AZUL-VERDE =====
            // Planeta con atmósfera uniforme de metano (azul-verde claro)
            
            let position = vertex_position;
            
            // Coordenadas esféricas para texturas procedurales
            let phi = position.x.atan2(position.z) + time * 0.08; // Rotación media
            let theta = (position.y / position.length()).acos();
            
            // CAPA 1: Color base azul-verde (metano)
            let base_noise = noise(phi * 3.0, theta * 3.0 + time * 0.05);
            
            let mut base_r = 140.0 + base_noise * 30.0;  // Cyan/verde-azulado
            let mut base_g = 220.0 + base_noise * 20.0;  // Verde-azul claro
            let mut base_b = 220.0 + base_noise * 25.0;  // Azul claro
            
            // CAPA 2: Bandas sutiles (muy poco visibles en Urano)
            let lat = theta / std::f32::consts::PI;
            let band_pattern = (lat * 6.0).sin();
            let band_noise = noise(phi * 2.0, lat * 10.0);
            
            let band_factor = (band_pattern * 0.5 + 0.5) * (band_noise * 0.5 + 0.5) * 0.15;
            base_r -= band_factor * 20.0;
            base_g -= band_factor * 15.0;
            base_b -= band_factor * 15.0;
            
            // CAPA 3: Atmósfera uniforme y suave
            let atmosphere_noise = noise(phi * 5.0 + time * 0.1, theta * 5.0);
            base_r += atmosphere_noise * 15.0 - 7.0;
            base_g += atmosphere_noise * 15.0 - 7.0;
            base_b += atmosphere_noise * 15.0 - 7.0;
            
            // CAPA 4: Brillo atmosférico en los bordes
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.3;
            
            base_r += atmosphere * 60.0;
            base_g += atmosphere * 80.0;
            base_b += atmosphere * 80.0;
            
            // Variación de iluminación
            let enhanced_intensity = intensity * 0.3 + 0.7; // Mínimo 70% (muy reflectante)
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Neptune => {
            // ===== SHADER DE NEPTUNO - GIGANTE DE HIELO AZUL OSCURO =====
            // Planeta con atmósfera de metano (azul intenso) y Gran Mancha Oscura
            
            let position = vertex_position;
            
            // Coordenadas esféricas para texturas procedurales
            let phi = position.x.atan2(position.z) + time * 0.12; // Rotación rápida
            let theta = (position.y / position.length()).acos();
            
            // CAPA 1: Color base azul profundo (metano)
            let base_noise = noise(phi * 4.0, theta * 4.0 + time * 0.06);
            
            let mut base_r = 40.0 + base_noise * 25.0;   // Azul muy oscuro
            let mut base_g = 90.0 + base_noise * 30.0;   // Azul medio
            let mut base_b = 200.0 + base_noise * 35.0;  // Azul intenso
            
            // CAPA 2: Bandas sutiles horizontales
            let lat = theta / std::f32::consts::PI;
            let band_pattern = (lat * 8.0 + time * 0.1).sin();
            let band_noise = noise(phi * 3.0, lat * 12.0);
            
            let band_factor = (band_pattern * 0.5 + 0.5) * (band_noise * 0.5 + 0.5) * 0.2;
            base_r += band_factor * 30.0 - 15.0;
            base_g += band_factor * 25.0 - 12.0;
            base_b += band_factor * 20.0 - 10.0;
            
            // CAPA 3: Gran Mancha Oscura (óvalo oscuro en latitud media)
            let spot_lat_center = 0.4; // Hemisferio sur
            let spot_lon_center = std::f32::consts::PI * 0.7 + time * 0.03;
            
            let lat_diff = (lat - spot_lat_center).abs();
            let lon_diff = (phi - spot_lon_center).abs().min(std::f32::consts::TAU - (phi - spot_lon_center).abs());
            
            let spot_distance = (lat_diff * lat_diff * 300.0 + lon_diff * lon_diff * 80.0).sqrt();
            
            if spot_distance < 1.2 {
                let spot_factor = (1.0 - spot_distance / 1.2).max(0.0) * 0.6;
                base_r *= 1.0 - spot_factor * 0.5;
                base_g *= 1.0 - spot_factor * 0.4;
                base_b *= 1.0 - spot_factor * 0.3;
            }
            
            // CAPA 4: Nubes cirros brillantes (raras)
            let cloud_noise = noise(phi * 10.0 + time * 0.2, theta * 10.0);
            if cloud_noise > 0.8 {
                let cloud_factor = (cloud_noise - 0.8) / 0.2 * 0.3;
                base_r += cloud_factor * 80.0;
                base_g += cloud_factor * 100.0;
                base_b += cloud_factor * 120.0;
            }
            
            // CAPA 5: Brillo atmosférico en los bordes
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.25;
            
            base_r += atmosphere * 40.0;
            base_g += atmosphere * 70.0;
            base_b += atmosphere * 100.0;
            
            // Variación de iluminación
            let enhanced_intensity = intensity * 0.3 + 0.7; // Mínimo 70% (muy reflectante)
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Spaceship => {
            // ===== SHADER DE LA NAVE CON COLORES PROCEDURALES =====
            // Esquema de colores militar/táctico basado en posición del vértice
            
            let avg_y = vertex_position.y;
            let avg_x = vertex_position.x;
            let avg_z = vertex_position.z;
            
            // Clasificar región del modelo para determinar color base
            let (base_r, base_g, base_b) = if avg_y > 0.1 && avg_x > -2.0 {
                // Cockpit/vidrio - parte superior central frontal - VERDE OLIVA MÁS CLARO
                (160.0, 180.0, 120.0)
            } else if avg_x > -3.0 {
                // Parte frontal - Verde oliva claro
                (140.0, 160.0, 100.0)
            } else if avg_x < -5.5 {
                // Parte trasera - Verde oscuro
                (90.0, 110.0, 70.0)
            } else if avg_z.abs() > 4.0 {
                // Laterales del cuerpo - Verde militar medio
                (115.0, 135.0, 90.0)
            } else if avg_y > 2.5 {
                // Protuberancias superiores - Gris claro verdoso
                (150.0, 155.0, 135.0)
            } else if avg_z < -5.0 {
                // Protuberancias traseras profundas - Marrón militar
                (130.0, 110.0, 75.0)
            } else if avg_x.abs() > 4.0 {
                // Protuberancias laterales externas - Gris verdoso
                (125.0, 130.0, 105.0)
            } else {
                // Centro del cuerpo - Verde oliva base
                (105.0, 125.0, 85.0)
            };
            
            // Aplicar intensidad de luz - MÁS LUZ AMBIENTAL (70% base + 30% direccional)
            let r = (base_r * (0.7 + 0.3 * intensity)).min(255.0);
            let g = (base_g * (0.7 + 0.3 * intensity)).min(255.0);
            let b = (base_b * (0.7 + 0.3 * intensity)).min(255.0);
            
            (r as u8, g as u8, b as u8)
        }
    }
}

// Estructura para almacenar un triángulo con información 3D completa
#[allow(dead_code)]
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
/// Dibuja el modelo con triángulos rellenos y sombreado mediante shaders.
fn render(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, 
    zbuffer: &mut ZBuffer, 
    model: &tobj::Model, 
    camera_position: Vec3,     // Posición libre de la cámara
    camera_target: Vec3,       // Objetivo de la cámara
    world_position: Vec3,      // Posición en el mundo (posición orbital)
    model_center: Vec3,        // Centrado del modelo (negativo del centroide)
    model_scale: f32,          // Escala del modelo
    rotation_y: f32,           // Rotación en Y
    shader_type: ShaderType,   // Tipo de shader a aplicar
    time: f32,                 // Tiempo para animaciones
) {
    let positions = &model.mesh.positions;
    let indices = &model.mesh.indices;

    // Matrices de transformación para pasar de coordenadas 3D a 2D
    // Ajustar near y far plane para manejar las enormes distancias del sistema solar
    let projection = Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32, 1.0, 50000.0);
    let view = Mat4::look_at_rh(
        camera_position,  // Posición de la cámara (ahora controlable)
        camera_target,    // Hacia dónde mira la cámara
        Vec3::Y,          // Vector "arriba"
    );
    
    // Matriz del modelo usando el orden correcto: T_world * R * S * T_center
    // Rotación solo en Y (0, rotation_y, 0)
    let rotation_vec = Vec3::new(0.0, rotation_y, 0.0);
    let model_matrix = create_model_matrix(
        world_position,     // Posición en el mundo (posición orbital)
        model_center,       // Centrado del modelo
        model_scale,        // Escala
        rotation_vec        // Rotación
    );

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
        
        // Calcular centro del triángulo para pasar al shader
        let avg_position = (v0 + v1 + v2) / 3.0;
        
        // Aplicar el shader correspondiente
        let (r, g, b) = apply_shader(
            shader_type,
            avg_position,
            normal,
            intensity,
            time
        );
        
        // Dibujar el triángulo directamente con z-buffer
        // Usar profundidad normalizada y mapeada a [0, 1] para z-buffer correcto
        // p.z/p.w está en rango [-1, 1], lo mapeamos a [0, 1]
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

/// Versión de render que acepta rotación completa en 3 ejes (para la nave)
fn render_with_full_rotation(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, 
    zbuffer: &mut ZBuffer, 
    model: &tobj::Model, 
    camera_position: Vec3,
    camera_target: Vec3,
    world_position: Vec3,
    model_center: Vec3,
    model_scale: f32,
    rotation: Vec3,           // Rotación completa (x, y, z)
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
    
    // Usar create_model_matrix que acepta Vec3 para rotación completa
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
        
        // Usar profundidad normalizada y mapeada a [0, 1] para z-buffer correcto
        // p.z/p.w está en rango [-1, 1], lo mapeamos a [0, 1]
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

    // Configuración del planeta rocoso (Tierra) - MISMO TAMAÑO QUE ANTES
    let rocky_scale = if max_r > 0.0 { 4.0 / max_r } else { 1.0 }; // Tierra (antes 2.0 -> ahora 4.0)

    let mut event_pump = sdl_context.event_pump()?;
    // Activar modo relativo del ratón para control tipo "mouselook"
    let mouse_subsystem = sdl_context.mouse();
    let _ = mouse_subsystem.set_relative_mouse_mode(true);
    // Track time for smooth movement
    let mut last_instant = Instant::now();
    
    // ===== SISTEMA DE CÁMARA LIBRE =====
    // Cámara posicionada más lejos para evitar colisiones iniciales tras cambios de escala
    let mut camera_position = Vec3::new(0.0, 60.0, 400.0); // Alejada del sol para permitir WASD
    let camera_yaw = 0.0f32; // Mirando hacia adelante (no usado para yaw, solo para dirección inicial)
    let mut camera_pitch = 0.0f32; // Horizonte
    
    // ===== ROTACIÓN DEL SOL =====
    let mut sun_rotation = 0.0_f32;
    
    // ===== ROTACIÓN Y TRASLACIÓN DEL PLANETA ROCOSO (TIERRA) =====
    let mut rocky_rotation = 0.0_f32;      // Rotación sobre su eje
    let mut rocky_orbit_angle = 0.0_f32;   // Ángulo orbital alrededor del sol
    // Aumentado 200% adicional (triplicado) respecto al valor previo
    let rocky_orbit_radius = 45.0_f32 * 3.0;     // antes 45.0 -> ahora 135.0
    let rocky_orbit_speed = 0.006_f32;     // Velocidad orbital ajustada
    let rocky_rotation_speed = 0.01_f32;   // Velocidad de rotación sobre su eje (más visible)
    
    // ===== ROTACIÓN Y TRASLACIÓN DE VENUS =====
    let mut venus_rotation = 0.0_f32;      // Rotación sobre su eje (muy lenta y retrógrada)
    let mut venus_orbit_angle = std::f32::consts::PI * 0.5; // Posición inicial diferente
    let venus_orbit_radius = 33.0_f32 * 3.0;     // antes 33.0 -> ahora 99.0
    let venus_orbit_speed = 0.008_f32;     // Más rápido que la Tierra (más cerca del sol)
    let venus_rotation_speed = -0.002_f32; // Rotación retrógrada (negativa) y muy lenta
    let venus_scale = if max_r > 0.0 { 3.8 / max_r } else { 1.0 }; // Casi del tamaño de la Tierra (doble)
    
    // ===== ROTACIÓN Y TRASLACIÓN DE MARTE =====
    let mut mars_rotation = 0.0_f32;       // Rotación sobre su eje
    let mut mars_orbit_angle = std::f32::consts::PI; // Empezar en lado opuesto
    let mars_orbit_radius = 60.0_f32 * 3.0;      // antes 60.0 -> ahora 180.0
    let mars_orbit_speed = 0.004_f32;      // Más lento que la Tierra (más lejos del sol)
    let mars_rotation_speed = 0.0098_f32;  // Rotación similar a la Tierra
    let mars_scale = if max_r > 0.0 { 3.0 / max_r } else { 1.0 }; // Más pequeño que la Tierra (doble)
    
    // ===== ROTACIÓN Y TRASLACIÓN DE JÚPITER (GIGANTE GASEOSO) =====
    let mut jupiter_rotation = 0.0_f32;    // Rotación sobre su eje (muy rápida)
    let mut jupiter_orbit_angle = std::f32::consts::PI * 1.5; // Posición inicial
    let jupiter_orbit_radius = 82.5_f32 * 3.0;   // antes 82.5 -> ahora 247.5
    let jupiter_orbit_speed = 0.002_f32;   // Muy lento (más lejos del sol)
    let jupiter_rotation_speed = 0.02_f32; // Rotación rápida (Júpiter rota en ~10 horas)
    let jupiter_scale = if max_r > 0.0 { 8.0 / max_r } else { 1.0 }; // Mitad del tamaño del Sol (doble)
    
    // ===== ROTACIÓN Y TRASLACIÓN DE URANO (GIGANTE DE HIELO) =====
    let mut uranus_rotation = 0.0_f32;     // Rotación sobre su eje
    let mut uranus_orbit_angle = std::f32::consts::PI * 0.3; // Posición inicial
    let uranus_orbit_radius = 105.0_f32 * 3.0;   // antes 105.0 -> ahora 315.0
    let uranus_orbit_speed = 0.0015_f32;   // Muy lento
    let uranus_rotation_speed = 0.015_f32; // Rotación media
    let uranus_scale = if max_r > 0.0 { 6.0 / max_r } else { 1.0 }; // Más pequeño que Júpiter (doble)
    
    // ===== ROTACIÓN Y TRASLACIÓN DE NEPTUNO (GIGANTE DE HIELO) =====
    let mut neptune_rotation = 0.0_f32;    // Rotación sobre su eje
    let mut neptune_orbit_angle = std::f32::consts::PI * 0.8; // Posición inicial
    let neptune_orbit_radius = 127.5_f32 * 3.0;  // antes 127.5 -> ahora 382.5
    let neptune_orbit_speed = 0.001_f32;   // Muy muy lento (más lejano)
    let neptune_rotation_speed = 0.016_f32; // Rotación media-rápida
    let neptune_scale = if max_r > 0.0 { 5.6 / max_r } else { 1.0 }; // Similar a Urano (doble)
    
    // ===== ROTACIÓN Y TRASLACIÓN DE LA LUNA (SATÉLITE DE LA TIERRA) =====
    let mut moon_rotation = 0.0_f32;       // Rotación sobre su eje (acoplamiento de marea)
    let mut moon_orbit_angle = 0.0_f32;    // Ángulo orbital alrededor de la Tierra
    let moon_orbit_radius = 5.0_f32 * 3.0;       // antes 5.0 -> ahora 15.0 (mantener proporcionalidad)
    let moon_orbit_speed = 0.05_f32;       // Velocidad orbital (completa órbita en ~2 minutos)
    let moon_rotation_speed = 0.05_f32;    // Misma que orbital (acoplamiento de marea - siempre muestra misma cara)
    let moon_scale = if max_r > 0.0 { 1.4 / max_r } else { 1.0 }; // Tamaño apropiado (doble)
    
    // ===== TIEMPO PARA ANIMACIONES =====
    let mut time = 0.0f32;

    'running: loop {
        // movement_delta se calculará después del bucle de eventos usando el estado del teclado

        // Manejo de eventos (como cerrar la ventana)
        // Acumulador de desplazamiento horizontal del ratón (pixels) por frame
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
                    // Solo usar movimiento vertical del ratón para ajustar pitch (mouselook Y)
                    // Sensibilidad aumentada respecto a la anterior pero menor que el valor original
                    let look_sensitivity = 0.0020_f32; // ajustar según petición
                    camera_pitch += -(yrel as f32) * look_sensitivity; // invertir Y para control natural
                    // Limitar pitch para evitar invertir la cámara
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

        // Movimiento continuo con WASD + espacio/shift (mouse controla la orientación)
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

        // Añadir movimiento lateral controlado por el mouse (derecha/izquierda)
        let mouse_sensitivity = 0.08_f32; // unidades por pixel (aumentada para mover más rápido al desplazarse)
        if mouse_dx.abs() > 0.0 {
            mv += right * (mouse_dx * mouse_sensitivity);
        }

        // Asignar movement_delta calculado
        let movement_delta = mv;
        
        // Rotación automática del sol
        sun_rotation += 0.005;
        
        // Actualizar rotación del planeta rocoso (Tierra)
        rocky_rotation += rocky_rotation_speed;
        
        // Actualizar órbita del planeta rocoso
        rocky_orbit_angle += rocky_orbit_speed;
        
        // Calcular posición orbital del planeta rocoso (Tierra) centrada en el origen (sol)
        let rocky_position = Vec3::new(
            rocky_orbit_radius * rocky_orbit_angle.cos(),
            0.0,
            rocky_orbit_radius * rocky_orbit_angle.sin()
        );
        
        // Actualizar rotación de Venus (retrógrada)
        venus_rotation += venus_rotation_speed;
        
        // Actualizar órbita de Venus
        venus_orbit_angle += venus_orbit_speed;
        
        // Calcular posición orbital de Venus
        let venus_position = Vec3::new(
            venus_orbit_radius * venus_orbit_angle.cos(),
            0.0,
            venus_orbit_radius * venus_orbit_angle.sin()
        );
        
        // Actualizar rotación de Marte
        mars_rotation += mars_rotation_speed;
        
        // Actualizar órbita de Marte
        mars_orbit_angle += mars_orbit_speed;
        
        // Calcular posición orbital de Marte
        let mars_position = Vec3::new(
            mars_orbit_radius * mars_orbit_angle.cos(),
            0.0,
            mars_orbit_radius * mars_orbit_angle.sin()
        );
        
        // Actualizar rotación de Júpiter (muy rápida)
        jupiter_rotation += jupiter_rotation_speed;
        
        // Actualizar órbita de Júpiter
        jupiter_orbit_angle += jupiter_orbit_speed;
        
        // Calcular posición orbital de Júpiter
        let jupiter_position = Vec3::new(
            jupiter_orbit_radius * jupiter_orbit_angle.cos(),
            0.0,
            jupiter_orbit_radius * jupiter_orbit_angle.sin()
        );
        
        // Actualizar rotación de Urano
        uranus_rotation += uranus_rotation_speed;
        
        // Actualizar órbita de Urano
        uranus_orbit_angle += uranus_orbit_speed;
        
        // Calcular posición orbital de Urano
        let uranus_position = Vec3::new(
            uranus_orbit_radius * uranus_orbit_angle.cos(),
            0.0,
            uranus_orbit_radius * uranus_orbit_angle.sin()
        );
        
        // Actualizar rotación de Neptuno
        neptune_rotation += neptune_rotation_speed;
        
        // Actualizar órbita de Neptuno
        neptune_orbit_angle += neptune_orbit_speed;
        
        // Calcular posición orbital de Neptuno
        let neptune_position = Vec3::new(
            neptune_orbit_radius * neptune_orbit_angle.cos(),
            0.0,
            neptune_orbit_radius * neptune_orbit_angle.sin()
        );
        
        // Actualizar rotación de la Luna (acoplamiento de marea)
        moon_rotation += moon_rotation_speed;
        
        // Actualizar órbita de la Luna alrededor de la Tierra
        moon_orbit_angle += moon_orbit_speed;
        
        // Calcular posición de la Luna RELATIVA A LA TIERRA (órbita circular)
        let moon_relative_position = Vec3::new(
            moon_orbit_radius * moon_orbit_angle.cos(),
            0.0,
            moon_orbit_radius * moon_orbit_angle.sin()
        );

        // Aplicar movimiento acumulado `movement_delta` con comprobación de colisiones
        // Primero calculamos posiciones relevantes (las órbitas ya fueron calculadas arriba)

        // Posiciones en el mundo de cada cuerpo
        let sun_center = Vec3::ZERO;
        let earth_center = rocky_position; // Tierra
        let venus_center = venus_position;
        let mars_center = mars_position;
        let jupiter_center = jupiter_position;
        let uranus_center = uranus_position;
        let neptune_center = neptune_position;
        let moon_center = earth_center + moon_relative_position;

        // Cámara: radio de colisión (tolerancia)
        let camera_radius = 1.0_f32;

        // Función helper inline para probar colisión con una esfera
        let collides = |pos: Vec3, radius: f32, target: Vec3| -> bool {
            (pos - target).length() < (radius + camera_radius)
        };

        // Proyecto la nueva posición
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

        // Calcular el objetivo de la cámara basado en yaw y pitch (después de moverla)
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

        // Dibujar órbitas proyectadas en pantalla
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

            // Órbitas de los planetas alrededor del Sol
            draw_orbit(rocky_orbit_radius, Vec3::ZERO, Color::RGB(90, 90, 90));
            draw_orbit(venus_orbit_radius, Vec3::ZERO, Color::RGB(90, 80, 70));
            draw_orbit(mars_orbit_radius, Vec3::ZERO, Color::RGB(100, 60, 60));
            draw_orbit(jupiter_orbit_radius, Vec3::ZERO, Color::RGB(80, 80, 100));
            draw_orbit(uranus_orbit_radius, Vec3::ZERO, Color::RGB(70, 90, 100));
            draw_orbit(neptune_orbit_radius, Vec3::ZERO, Color::RGB(60, 80, 120));

            // Órbita de la Luna alrededor de la Tierra
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
                Vec3::ZERO,      // El sol está en el origen del mundo
                sun_translation, // Centrado del modelo del sol
                sun_scale,
                Vec3::new(0.0, sun_rotation, 0.0), // Rotación como Vec3
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
                rocky_position,  // Posición orbital de la Tierra
                sun_translation, // Usar el mismo centrado que el sol (misma geometría)
                rocky_scale, 
                rocky_rotation,  // Rotación sobre su eje
                ShaderType::RockyPlanet,
                time
            );
        }
        
        // ===== RENDERIZAR LA LUNA (SATÉLITE DE LA TIERRA) =====
        for model in rocky_models.iter() {
            // La Luna orbita la Tierra. La Tierra está en rocky_position
            // La Luna está a moon_relative_position de la Tierra
            let moon_world_position = rocky_position + moon_relative_position;
            
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                moon_world_position, // Posición de la Luna en el mundo
                sun_translation,     // Usar el mismo centrado (misma geometría)
                moon_scale, 
                moon_rotation,  // Rotación (acoplamiento de marea)
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
                venus_position,  // Posición orbital de Venus
                sun_translation, // Usar el mismo centrado (misma geometría)
                venus_scale, 
                venus_rotation,  // Rotación sobre su eje (retrógrada)
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
                mars_position,   // Posición orbital de Marte
                sun_translation, // Usar el mismo centrado (misma geometría)
                mars_scale, 
                mars_rotation,   // Rotación sobre su eje
                ShaderType::Mars,
                time
            );
        }
        
        // ===== RENDERIZAR JÚPITER (GIGANTE GASEOSO) =====
        for model in rocky_models.iter() {
            render(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                jupiter_position, // Posición orbital de Júpiter
                sun_translation,  // Usar el mismo centrado (misma geometría)
                jupiter_scale, 
                jupiter_rotation, // Rotación sobre su eje (muy rápida)
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
                uranus_position, // Posición orbital de Urano
                sun_translation, // Usar el mismo centrado (misma geometría)
                uranus_scale, 
                uranus_rotation, // Rotación sobre su eje
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
                neptune_position, // Posición orbital de Neptuno
                sun_translation,  // Usar el mismo centrado (misma geometría)
                neptune_scale, 
                neptune_rotation, // Rotación sobre su eje
                ShaderType::Neptune,
                time
            );
        }
        
        // ===== RENDERIZAR LA NAVE (SIEMPRE ENFRENTE DE LA CÁMARA) =====
        // Calcular posición de la nave: adelante de la cámara
        let ship_forward_distance = 8.0; // Distancia delante de la cámara
        let ship_down_offset = -1.5;     // Offset hacia abajo para que no tape la vista
        let ship_right_offset = 0.0;     // Sin offset lateral por defecto
        
        let up = Vec3::Y;
        let ship_position = camera_position 
            + forward * ship_forward_distance 
            + up * ship_down_offset
            + right * ship_right_offset;
        
        // Rotación completa de la nave para que apunte exactamente donde mira la cámara
        // Pitch (X): positivo para que la nariz suba/baje correctamente con la cámara
        // Yaw (Y): rotación horizontal
        // Roll (Z): mantener en 0 para no inclinar lateralmente
        let ship_rotation = Vec3::new(
            camera_pitch,   // Pitch - apunta arriba/abajo (sin invertir)
            camera_yaw,     // Yaw - apunta izquierda/derecha
            0.0             // Roll - sin inclinación lateral
        );
        
        for model in spaceship_models.iter() {
            render_with_full_rotation(
                &mut canvas, 
                &mut zbuffer, 
                model, 
                camera_position,
                camera_target,
                ship_position,      // Posición relativa a la cámara
                ship_translation,   // Centrado del modelo de la nave
                ship_scale,         // Escala de la nave
                ship_rotation,      // Rotación completa en 3 ejes
                ShaderType::Spaceship,
                time
            );
        }

        // Muestra el contenido del buffer en la pantalla
        canvas.present();
    }

    Ok(())
}