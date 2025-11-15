#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    Sun,
    RockyPlanet,  // Planeta rocoso tipo Tierra
    Venus,        // Planeta Venus - amarillo/naranja con atmósfera densa
    Mars,         // Planeta Marte - rojo/oxidado
    Moon,         // Luna de la Tierra - gris rocoso con cráteres
    Jupiter,      // Júpiter - gigante gaseoso con bandas
    Uranus,       // Urano - gigante de hielo azul-verde
    Neptune,      // Neptuno - gigante de hielo azul oscuro
    Spaceship,    // Para la nave espacial
}
