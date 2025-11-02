fn main() {
    // Este script de compilación es para enlazar con la librería SDL2 manualmente.
    // Asume que has descargado la librería de desarrollo de SDL2 para MSVC
    // y has colocado los archivos .lib en una carpeta llamada `sdl2/lib` en la raíz de tu proyecto.
    
    // Obtiene el directorio actual del proyecto.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // Construye la ruta a los archivos de la librería SDL2.
    // Asumimos que estarán en `C:\Github\Proyecto-sistema-solar\sdl2\lib`
    let lib_path = std::path::Path::new(&manifest_dir).join("sdl2").join("lib");
    
    // Le dice a rustc que busque librerías en este directorio.
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    
    // Le dice a rustc que enlace con las librerías SDL2.
    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-lib=SDL2main");
}
