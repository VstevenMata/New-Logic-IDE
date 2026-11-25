//! Lo que cambia entre el escritorio y el navegador: diálogos de archivos, escribir
//! archivos y el portapapeles. En el navegador no hay carpetas: guardar o exportar
//! descarga el archivo y abrir lo sube (llega después, por un canal).

use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

/// Corre en el navegador
pub const WEB: bool = cfg!(target_arch = "wasm32");

/// Filtro de los diálogos para los proyectos
pub const SA: (&str, &[&str]) = ("Circuito de LogicSM (.SA)", &["SA", "sa"]);

/// Un `.SA` leído: su ruta (en el navegador, solo el nombre) y su texto
pub type Subido = (String, String);

/// Un `.SA` leído y si se abre (`true`) o se agrega al proyecto abierto (`false`)
pub type Leido = (Subido, bool);

/// Por donde llegan los `.SA` leídos
pub type Llegada = Sender<Leido>;

/// Dónde deja `pegar` lo que leyó del portapapeles
fn id_pegado() -> egui::Id {
    egui::Id::unique("pegado")
}

/// Lo que leyó `pegar` (el texto o por qué no se pudo), cuando ya llegó
pub fn pegado(ctx: &egui::Context) -> Option<Result<String, String>> {
    // Va dentro de un Option: la memoria de egui pide tipos con valor por defecto
    ctx.data_mut(|d| d.remove_temp::<Option<Result<String, String>>>(id_pegado())).flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub use escritorio::*;
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
mod escritorio {
    use super::*;

    /// Pide un `.SA` y devuelve su ruta
    pub fn abrir_sa(_: &egui::Context, titulo: &str, _: &Llegada) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title(titulo)
            .add_filter(SA.0, SA.1)
            .pick_file()
    }

    /// Pide dónde guardar un archivo
    pub fn guardar_como(
        titulo: &str,
        filtro: (&str, &[&str]),
        nombre: &str,
        carpeta: Option<&Path>,
    ) -> Option<PathBuf> {
        let mut d = rfd::FileDialog::new()
            .set_title(titulo)
            .add_filter(filtro.0, filtro.1)
            .set_file_name(nombre);
        if let Some(c) = carpeta {
            d = d.set_directory(c);
        }
        d.save_file()
    }

    /// Pide una carpeta (para exportar varios archivos)
    pub fn elegir_carpeta(titulo: &str, carpeta: Option<&Path>) -> Option<PathBuf> {
        let mut d = rfd::FileDialog::new().set_title(titulo);
        if let Some(c) = carpeta {
            d = d.set_directory(c);
        }
        d.pick_folder()
    }

    pub fn escribir(ruta: &Path, datos: &[u8]) -> std::io::Result<()> {
        std::fs::write(ruta, datos)
    }

    /// Lee un archivo soltado sobre la ventana y lo manda por `llegada`
    pub fn leer_soltado(
        _: &egui::Context,
        f: egui::DroppedFileHandle,
        abrir: bool,
        llegada: &Llegada,
    ) -> Result<(), String> {
        let texto = String::from_utf8_lossy(&f.bytes()?).into_owned();
        let _ = llegada.send(((f.path().to_string_lossy().into_owned(), texto), abrir));
        Ok(())
    }

    /// Lee el portapapeles; se recoge con `pegado`
    pub fn pegar(ctx: &egui::Context) {
        let texto = arboard::Clipboard::new()
            .and_then(|mut p| p.get_text())
            .map_err(|e| format!("No se pudo leer el portapapeles: {e}"));
        ctx.data_mut(|d| d.insert_temp(id_pegado(), Some(texto)));
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use eframe::wasm_bindgen::JsCast;

    /// Pide un `.SA`: el archivo llega después por `llegada` y devuelve `None`
    pub fn abrir_sa(ctx: &egui::Context, titulo: &str, llegada: &Llegada) -> Option<PathBuf> {
        let dialogo = rfd::AsyncFileDialog::new()
            .set_title(titulo)
            .add_filter(SA.0, SA.1);
        let (ctx, llegada) = (ctx.clone(), llegada.clone());
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(archivo) = dialogo.pick_file().await {
                let texto = String::from_utf8_lossy(&archivo.read().await).into_owned();
                let _ = llegada.send(((archivo.file_name(), texto), true));
                ctx.request_repaint();
            }
        });
        None
    }

    /// Lee un archivo soltado sobre la página; llega después por `llegada`
    pub fn leer_soltado(
        ctx: &egui::Context,
        f: egui::DroppedFileHandle,
        abrir: bool,
        llegada: &Llegada,
    ) -> Result<(), String> {
        let (ctx, llegada) = (ctx.clone(), llegada.clone());
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(bytes) = f.bytes_async().await {
                let texto = String::from_utf8_lossy(&bytes).into_owned();
                let _ = llegada.send(((f.path().to_string_lossy().into_owned(), texto), abrir));
                ctx.request_repaint();
            }
        });
        Ok(())
    }

    /// No se pregunta: se descarga con ese nombre
    pub fn guardar_como(
        _: &str,
        _: (&str, &[&str]),
        nombre: &str,
        _: Option<&Path>,
    ) -> Option<PathBuf> {
        Some(PathBuf::from(nombre))
    }

    /// No hay carpetas: cada archivo se descarga con su nombre
    pub fn elegir_carpeta(_: &str, _: Option<&Path>) -> Option<PathBuf> {
        Some(PathBuf::new())
    }

    /// Descarga `datos` con el nombre del archivo de `ruta`
    pub fn escribir(ruta: &Path, datos: &[u8]) -> std::io::Result<()> {
        let fallo = || std::io::Error::other("el navegador no dejó descargar el archivo");
        let nombre = ruta
            .file_name()
            .map_or("archivo".into(), |n| n.to_string_lossy().into_owned());
        let partes = js_sys::Array::of1(&js_sys::Uint8Array::from(datos));
        let blob = web_sys::Blob::new_with_u8_array_sequence(&partes).map_err(|_| fallo())?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| fallo())?;
        let enlace = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.create_element("a").ok())
            .and_then(|e| e.dyn_into::<web_sys::HtmlAnchorElement>().ok())
            .ok_or_else(fallo)?;
        enlace.set_href(&url);
        enlace.set_download(&nombre);
        enlace.click();
        let _ = web_sys::Url::revoke_object_url(&url);
        Ok(())
    }

    /// Lee el portapapeles; llega después y se recoge con `pegado`.
    /// Va dentro del toque en el botón: sin eso el navegador no deja leerlo
    /// (en el iPhone aparece la burbuja "Pegar" para confirmar)
    pub fn pegar(ctx: &egui::Context) {
        let guardar = |ctx: &egui::Context, texto: Result<String, String>| {
            ctx.data_mut(|d| d.insert_temp(id_pegado(), Some(texto)));
            ctx.request_repaint();
        };
        let Some(ventana) = web_sys::window().filter(|w| w.is_secure_context()) else {
            // Sin https el navegador no da el portapapeles
            return guardar(ctx, Err("El navegador no deja leer el portapapeles aquí.".into()));
        };
        let promesa = ventana.navigator().clipboard().read_text();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let texto = wasm_bindgen_futures::JsFuture::from(promesa)
                .await
                .ok()
                .and_then(|t| t.as_string())
                .ok_or_else(|| "No se pudo leer el portapapeles (¿se dio permiso?).".to_string());
            guardar(&ctx, texto);
        });
    }
}
