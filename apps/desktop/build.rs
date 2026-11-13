// En Windows, el logo queda como ícono del .exe (Explorador, barra de tareas y acceso del menú Inicio)
fn main() {
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("../../assets/logo.ico")
        .compile()
        .expect("no se pudo agregar el ícono al .exe");
}
