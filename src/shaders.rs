use glam::Vec3;
use crate::shader_type::ShaderType;
use crate::utils::noise;

pub fn apply_shader(
    shader_type: ShaderType,
    vertex_position: Vec3,
    normal: Vec3,
    intensity: f32,
    time: f32,
) -> (u8, u8, u8) {
    match shader_type {
        ShaderType::Sun => {
            let position = vertex_position;
            let radius = (position.x * position.x + position.y * position.y + position.z * position.z).sqrt();
            let theta = position.y.atan2((position.x * position.x + position.z * position.z).sqrt());
            let phi = position.z.atan2(position.x);
            
            let core_distance = radius.max(0.01);
            let core_gradient = (1.0 - (core_distance / 2.0).min(1.0)).max(0.0);
            
            let base_r = 255.0;
            let base_g = 180.0 + core_gradient * 50.0;
            let base_b = 20.0 + core_gradient * 30.0;
            
            let spot_freq = 3.0;
            let spot_noise = noise(phi * spot_freq + time * 0.1, theta * spot_freq);
            let spot_noise2 = noise(phi * spot_freq * 2.0 - time * 0.15, theta * spot_freq * 2.0);
            let combined_spots = (spot_noise + spot_noise2 * 0.5) / 1.5;
            
            let spot_factor = if combined_spots > 0.65 { 0.6 } else { 1.0 };
            
            let flare_noise = noise(phi * 2.0 + time * 0.3, theta * 2.0 + time * 0.2);
            let flare_factor = if flare_noise > 0.7 { 1.0 + (flare_noise - 0.7) * 2.0 } else { 1.0 };
            
            let pulse = ((time * 2.0).sin() * 0.5 + 0.5) * 0.15 + 0.85;
            
            let turb_noise = noise(phi * 8.0 + time * 0.5, theta * 8.0 - time * 0.3);
            let turb_factor = 0.9 + turb_noise * 0.2;
            
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let corona = edge_factor * edge_factor * 0.3;
            
            let r = (base_r * spot_factor * flare_factor * pulse * turb_factor * (0.7 + 0.3 * intensity) + corona * 100.0).min(255.0);
            let g = (base_g * spot_factor * flare_factor * pulse * turb_factor * (0.7 + 0.3 * intensity) + corona * 80.0).min(255.0);
            let b = (base_b * spot_factor * pulse * turb_factor * (0.7 + 0.3 * intensity) + corona * 20.0).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::RockyPlanet => {
            let position = vertex_position;
            let theta = position.y.atan2((position.x * position.x + position.z * position.z).sqrt());
            let phi = position.z.atan2(position.x);
            let lat = (theta / std::f32::consts::PI) + 0.5;
            
            let land_noise = noise(phi * 5.0 + time * 0.05, theta * 5.0);
            let land_noise2 = noise(phi * 10.0 - time * 0.03, theta * 10.0 + 100.0);
            let is_land = (land_noise * 0.6 + land_noise2 * 0.4) > 0.48;
            
            let ocean_color = (10.0, 50.0, 120.0);
            let shallow_ocean = (30.0, 80.0, 150.0);
            let land_color = (34.0, 139.0, 34.0);
            let desert_color = (210.0, 180.0, 140.0);
            let mountain_color = (139.0, 137.0, 137.0);
            
            let (mut base_r, mut base_g, mut base_b) = if is_land {
                let terrain_variation = noise(phi * 3.0, theta * 3.0 + 50.0);
                
                if lat > 0.75 || lat < 0.25 {
                    (240.0, 240.0, 255.0)
                } else if terrain_variation > 0.65 {
                    mountain_color
                } else if (lat > 0.35 && lat < 0.42) || (lat > 0.58 && lat < 0.65) {
                    desert_color
                } else {
                    let green_variation = terrain_variation * 20.0;
                    (land_color.0 + green_variation, land_color.1 + green_variation, land_color.2 + green_variation * 0.5)
                }
            } else {
                let depth_noise = noise(phi * 8.0, theta * 8.0 + 200.0);
                if depth_noise > 0.6 {
                    shallow_ocean
                } else {
                    let depth_factor = 0.8 + depth_noise * 0.2;
                    (ocean_color.0 * depth_factor, ocean_color.1 * depth_factor, ocean_color.2 * depth_factor)
                }
            };
            
            let polar_threshold = 0.80;
            let polar_factor = if lat > polar_threshold {
                ((lat - polar_threshold) / (1.0 - polar_threshold)).powf(0.4)
            } else if lat < (1.0 - polar_threshold) {
                ((1.0 - polar_threshold - lat) / (1.0 - polar_threshold)).powf(0.4)
            } else {
                0.0
            };
            
            if polar_factor > 0.0 {
                let snow_white = 255.0;
                let ice_blue_tint = 0.95;
                base_r = base_r * (1.0 - polar_factor) + snow_white * polar_factor;
                base_g = base_g * (1.0 - polar_factor) + snow_white * polar_factor;
                base_b = base_b * (1.0 - polar_factor) + (snow_white * ice_blue_tint) * polar_factor;
            }
            
            let cloud_noise1 = noise(phi * 6.0 + time * 0.3, theta * 6.0);
            let cloud_noise2 = noise(phi * 12.0 - time * 0.2, theta * 12.0 + 300.0);
            let cloud_combined = cloud_noise1 * 0.6 + cloud_noise2 * 0.4;
            
            let cloud_factor = if cloud_combined > 0.6 {
                ((cloud_combined - 0.6) / 0.4).min(1.0) * 0.7
            } else {
                0.0
            };
            
            if cloud_factor > 0.0 {
                let cloud_white = 240.0;
                base_r = base_r * (1.0 - cloud_factor) + cloud_white * cloud_factor;
                base_g = base_g * (1.0 - cloud_factor) + cloud_white * cloud_factor;
                base_b = base_b * (1.0 - cloud_factor) + cloud_white * cloud_factor;
            }
            
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.25;
            
            base_r += atmosphere * 50.0;
            base_g += atmosphere * 100.0;
            base_b += atmosphere * 200.0;
            
            let enhanced_intensity = intensity * 0.4 + 0.6;
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Venus => {
            let position = vertex_position;
            let phi = position.x.atan2(position.z) + time * 0.15;
            let theta = (position.y / position.length()).acos();
            
            let base_noise1 = noise(phi * 3.0, theta * 3.0);
            let base_noise2 = noise(phi * 6.0 + 50.0, theta * 6.0 + 50.0);
            let base_combined = base_noise1 * 0.6 + base_noise2 * 0.4;
            
            let mut base_r;
            let mut base_g;
            let mut base_b;
            
            if base_combined < 0.35 {
                base_r = 200.0; base_g = 140.0; base_b = 50.0;
            } else if base_combined < 0.7 {
                base_r = 230.0; base_g = 190.0; base_b = 80.0;
            } else {
                base_r = 250.0; base_g = 220.0; base_b = 120.0;
            }
            
            let band_pattern = (theta * 8.0 + phi * 2.0 + time * 0.3).sin();
            let band_noise = noise(phi * 4.0 - time * 0.2, theta * 4.0);
            let band_factor = (band_pattern * 0.5 + 0.5) * (band_noise * 0.5 + 0.5);
            
            let band_darken = band_factor * 0.3;
            base_r -= band_darken * 80.0;
            base_g -= band_darken * 60.0;
            base_b -= band_darken * 30.0;
            
            let cloud_noise1 = noise(phi * 5.0 + time * 0.4, theta * 5.0);
            let cloud_noise2 = noise(phi * 10.0 - time * 0.3, theta * 10.0 + 100.0);
            let cloud_combined = cloud_noise1 * 0.7 + cloud_noise2 * 0.3;
            
            let cloud_factor = if cloud_combined > 0.65 {
                ((cloud_combined - 0.65) / 0.35).min(1.0) * 0.5
            } else {
                0.0
            };
            
            if cloud_factor > 0.0 {
                base_r = base_r * (1.0 - cloud_factor) + 255.0 * cloud_factor;
                base_g = base_g * (1.0 - cloud_factor) + 235.0 * cloud_factor;
                base_b = base_b * (1.0 - cloud_factor) + 150.0 * cloud_factor;
            }
            
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.4;
            
            base_r += atmosphere * 150.0;
            base_g += atmosphere * 120.0;
            base_b += atmosphere * 50.0;
            
            let enhanced_intensity = intensity * 0.3 + 0.7;
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Mars => {
            let position = vertex_position;
            let phi = position.x.atan2(position.z) + time * 0.05;
            let theta = (position.y / position.length()).acos();
            
            let terrain_noise1 = noise(phi * 4.0, theta * 4.0);
            let terrain_noise2 = noise(phi * 8.0 + 100.0, theta * 8.0 + 100.0);
            let terrain_combined = terrain_noise1 * 0.6 + terrain_noise2 * 0.4;
            
            let mut base_r;
            let mut base_g;
            let mut base_b;
            
            if terrain_combined < 0.3 {
                base_r = 80.0; base_g = 40.0; base_b = 30.0;
            } else if terrain_combined < 0.7 {
                base_r = 193.0; base_g = 68.0; base_b = 14.0;
            } else {
                base_r = 210.0; base_g = 105.0; base_b = 30.0;
            }
            
            let polar_threshold = 0.85;
            let polar_distance = theta.min(std::f32::consts::PI - theta) / std::f32::consts::PI;
            
            if polar_distance > polar_threshold {
                let polar_factor = ((polar_distance - polar_threshold) / (1.0 - polar_threshold)).min(1.0);
                let ice_white = 240.0;
                let ice_cream = 230.0;
                base_r = base_r * (1.0 - polar_factor) + ice_white * polar_factor;
                base_g = base_g * (1.0 - polar_factor) + ice_cream * polar_factor;
                base_b = base_b * (1.0 - polar_factor) + (ice_cream * 0.9) * polar_factor;
            }
            
            let dust_noise1 = noise(phi * 3.0 + time * 0.1, theta * 3.0);
            let dust_noise2 = noise(phi * 6.0 - time * 0.05, theta * 6.0 + 200.0);
            let dust_combined = dust_noise1 * 0.5 + dust_noise2 * 0.5;
            
            let dust_factor = if dust_combined > 0.75 {
                ((dust_combined - 0.75) / 0.25).min(1.0) * 0.4
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
            
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.15;
            
            base_r += atmosphere * 100.0;
            base_g += atmosphere * 30.0;
            base_b += atmosphere * 10.0;
            
            let enhanced_intensity = intensity * 0.5 + 0.5;
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Moon => {
            let position = vertex_position;
            let phi = position.x.atan2(position.z);
            let theta = (position.y / position.length()).acos();
            
            let terrain_noise1 = noise(phi * 6.0, theta * 6.0);
            let terrain_noise2 = noise(phi * 12.0 + 100.0, theta * 12.0 + 100.0);
            let terrain_combined = terrain_noise1 * 0.6 + terrain_noise2 * 0.4;
            
            let mut base_r;
            let mut base_g;
            let mut base_b;
            
            if terrain_combined < 0.3 {
                base_r = 80.0; base_g = 80.0; base_b = 85.0;
            } else if terrain_combined < 0.7 {
                base_r = 140.0; base_g = 140.0; base_b = 145.0;
            } else {
                base_r = 180.0; base_g = 180.0; base_b = 185.0;
            }
            
            let crater_noise1 = noise(phi * 15.0, theta * 15.0);
            let crater_noise2 = noise(phi * 30.0 + 200.0, theta * 30.0 + 200.0);
            
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
            
            let ray_noise = noise(phi * 8.0 + theta * 8.0, theta * 4.0);
            if ray_noise > 0.7 {
                let ray_brightness = ((ray_noise - 0.7) / 0.3) * 0.2;
                base_r += ray_brightness * 100.0;
                base_g += ray_brightness * 100.0;
                base_b += ray_brightness * 105.0;
            }
            
            let harsh_intensity = if intensity > 0.5 { intensity * 0.8 + 0.2 } else { intensity * 0.3 };
            
            let r = (base_r * harsh_intensity).min(255.0);
            let g = (base_g * harsh_intensity).min(255.0);
            let b = (base_b * harsh_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Jupiter => {
            let position = vertex_position;
            let phi = position.x.atan2(position.z) + time * 0.1;
            let theta = (position.y / position.length()).acos();
            let lat = theta / std::f32::consts::PI;
            
            let band_freq = 12.0;
            let band_pattern = (lat * band_freq).sin();
            let band_noise = noise(phi * 2.0, lat * 15.0 + time * 0.05);
            
            let mut base_r;
            let mut base_g;
            let mut base_b;
            
            if (band_pattern + band_noise * 0.3) > 0.0 {
                base_r = 220.0 + band_noise * 20.0;
                base_g = 190.0 + band_noise * 20.0;
                base_b = 140.0 + band_noise * 15.0;
            } else {
                base_r = 180.0 + band_noise * 15.0;
                base_g = 130.0 + band_noise * 15.0;
                base_b = 80.0 + band_noise * 10.0;
            }
            
            let turb_noise1 = noise(phi * 8.0 + time * 0.2, lat * 20.0);
            let turb_noise2 = noise(phi * 15.0 - time * 0.15, lat * 30.0 + 100.0);
            let turbulence = turb_noise1 * 0.6 + turb_noise2 * 0.4;
            
            base_r += turbulence * 30.0 - 15.0;
            base_g += turbulence * 25.0 - 12.0;
            base_b += turbulence * 20.0 - 10.0;
            
            let spot_lat_center = 0.6;
            let spot_lon_center = std::f32::consts::PI * 0.5 + time * 0.02;
            
            let lat_diff = (lat - spot_lat_center).abs();
            let lon_diff = (phi - spot_lon_center).abs().min(std::f32::consts::TAU - (phi - spot_lon_center).abs());
            
            let spot_distance = (lat_diff * lat_diff * 400.0 + lon_diff * lon_diff * 100.0).sqrt();
            
            if spot_distance < 1.5 {
                let spot_factor = (1.0 - spot_distance / 1.5).max(0.0);
                let spot_noise = noise(phi * 10.0 + time * 0.1, lat * 10.0);
                
                base_r = base_r * (1.0 - spot_factor * 0.8) + (200.0 + spot_noise * 20.0) * spot_factor * 0.8;
                base_g = base_g * (1.0 - spot_factor * 0.8) + (100.0 + spot_noise * 10.0) * spot_factor * 0.8;
                base_b = base_b * (1.0 - spot_factor * 0.8) + (80.0 + spot_noise * 10.0) * spot_factor * 0.8;
            }
            
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
            
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.2;
            
            base_r += atmosphere * 80.0;
            base_g += atmosphere * 70.0;
            base_b += atmosphere * 50.0;
            
            let enhanced_intensity = intensity * 0.4 + 0.6;
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Uranus => {
            let position = vertex_position;
            let phi = position.x.atan2(position.z) + time * 0.08;
            let theta = (position.y / position.length()).acos();
            
            let base_noise = noise(phi * 3.0, theta * 3.0 + time * 0.05);
            
            let mut base_r = 140.0 + base_noise * 30.0;
            let mut base_g = 220.0 + base_noise * 20.0;
            let mut base_b = 220.0 + base_noise * 25.0;
            
            let lat = theta / std::f32::consts::PI;
            let band_pattern = (lat * 6.0).sin();
            let band_noise = noise(phi * 2.0, lat * 10.0);
            
            let band_factor = (band_pattern * 0.5 + 0.5) * (band_noise * 0.5 + 0.5) * 0.15;
            base_r -= band_factor * 20.0;
            base_g -= band_factor * 15.0;
            base_b -= band_factor * 15.0;
            
            let atmosphere_noise = noise(phi * 5.0 + time * 0.1, theta * 5.0);
            base_r += atmosphere_noise * 15.0 - 7.0;
            base_g += atmosphere_noise * 15.0 - 7.0;
            base_b += atmosphere_noise * 15.0 - 7.0;
            
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.3;
            
            base_r += atmosphere * 60.0;
            base_g += atmosphere * 80.0;
            base_b += atmosphere * 80.0;
            
            let enhanced_intensity = intensity * 0.3 + 0.7;
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Neptune => {
            let position = vertex_position;
            let phi = position.x.atan2(position.z) + time * 0.12;
            let theta = (position.y / position.length()).acos();
            
            let base_noise = noise(phi * 4.0, theta * 4.0 + time * 0.06);
            
            let mut base_r = 40.0 + base_noise * 25.0;
            let mut base_g = 90.0 + base_noise * 30.0;
            let mut base_b = 200.0 + base_noise * 35.0;
            
            let lat = theta / std::f32::consts::PI;
            let band_pattern = (lat * 8.0 + time * 0.1).sin();
            let band_noise = noise(phi * 3.0, lat * 12.0);
            
            let band_factor = (band_pattern * 0.5 + 0.5) * (band_noise * 0.5 + 0.5) * 0.2;
            base_r += band_factor * 30.0 - 15.0;
            base_g += band_factor * 25.0 - 12.0;
            base_b += band_factor * 20.0 - 10.0;
            
            let spot_lat_center = 0.4;
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
            
            let cloud_noise = noise(phi * 10.0 + time * 0.2, theta * 10.0);
            if cloud_noise > 0.8 {
                let cloud_factor = (cloud_noise - 0.8) / 0.2 * 0.3;
                base_r += cloud_factor * 80.0;
                base_g += cloud_factor * 100.0;
                base_b += cloud_factor * 120.0;
            }
            
            let edge_factor = 1.0 - normal.dot(Vec3::new(0.0, 0.0, 1.0)).abs();
            let atmosphere = edge_factor * edge_factor * 0.25;
            
            base_r += atmosphere * 40.0;
            base_g += atmosphere * 70.0;
            base_b += atmosphere * 100.0;
            
            let enhanced_intensity = intensity * 0.3 + 0.7;
            
            let r = (base_r * enhanced_intensity).min(255.0);
            let g = (base_g * enhanced_intensity).min(255.0);
            let b = (base_b * enhanced_intensity).min(255.0);
            
            (r as u8, g as u8, b as u8)
        },
        
        ShaderType::Spaceship => {
            let avg_y = vertex_position.y;
            let avg_x = vertex_position.x;
            let avg_z = vertex_position.z;
            
            let (base_r, base_g, base_b) = if avg_y > 0.1 && avg_x > -2.0 {
                (160.0, 180.0, 120.0)
            } else if avg_x > -3.0 {
                (140.0, 160.0, 100.0)
            } else if avg_x < -5.5 {
                (90.0, 110.0, 70.0)
            } else if avg_z.abs() > 4.0 {
                (115.0, 135.0, 90.0)
            } else if avg_y > 2.5 {
                (150.0, 155.0, 135.0)
            } else if avg_z < -5.0 {
                (130.0, 110.0, 75.0)
            } else if avg_x.abs() > 4.0 {
                (125.0, 130.0, 105.0)
            } else {
                (105.0, 125.0, 85.0)
            };
            
            let r = (base_r * (0.7 + 0.3 * intensity)).min(255.0);
            let g = (base_g * (0.7 + 0.3 * intensity)).min(255.0);
            let b = (base_b * (0.7 + 0.3 * intensity)).min(255.0);
            
            (r as u8, g as u8, b as u8)
        }
    }
}
