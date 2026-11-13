// En Windows, el logo queda como ícono del .exe (Explorador, barra de tareas y acceso del menú Inicio)
// y "LogicSM" como su nombre (Propiedades, barra de tareas y Administrador de tareas)
fn main() {
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("../../assets/logo.ico")
        .set("FileDescription", "LogicSM")
        .set("ProductName", "LogicSM")
        .compile()
        .expect("no se pudo agregar el ícono al .exe");
}
