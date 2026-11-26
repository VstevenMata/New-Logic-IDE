//! LogicSM: el IDE (editor, lienzo del circuito y paneles), de escritorio o en el navegador.
// Sin ventana de consola extra al abrirlo en Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ayuda;
#[cfg(not(target_arch = "wasm32"))]
mod comandos;
mod interfaz;
mod plataforma;
mod proyecto;

#[cfg(not(target_arch = "wasm32"))]
use std::{path::PathBuf, process::ExitCode};

/// Logo de la app: ícono de la ventana y barra superior
pub const LOGO: &[u8] = include_bytes!("../../../assets/logo.png");

/// Ícono de los proyectos `.SA` (el mismo que les pone el instalador en el sistema)
pub const ICONO_SA: &[u8] = include_bytes!("../../../assets/archivo.png");

/// Certificado de Rust del autor (Ayuda → Acerca de), uno por tema
pub const CERTIFICADO_CLARO: &[u8] = include_bytes!("../../../assets/certificado-rust-claro.png");
pub const CERTIFICADO_OSCURO: &[u8] = include_bytes!("../../../assets/certificado-rust-oscuro.png");

// logicsm (o logicsm open) abre la ventana y logicsm Proyecto.SA abre ese proyecto (doble click
// en el explorador); logicsm update | delete | help son comandos de terminal
#[cfg(not(target_arch = "wasm32"))]
fn main() -> ExitCode {
    let resultado = match std::env::args().nth(1).as_deref() {
        None | Some("open") => ventana(None),
        Some(ruta) if proyecto::es_sa(ruta.as_ref()) => ventana(Some(ruta.into())),
        Some(orden) => return comandos::ejecutar(orden),
    };
    match resultado {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            comandos::consola();
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ventana(proyecto: Option<PathBuf>) -> eframe::Result {
    let opciones = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("LogicSM")
            // Igual al nombre de logicsm.desktop: así Linux (Wayland) le pone el ícono correcto
            .with_app_id("logicsm")
            .with_icon(
                eframe::icon_data::from_png_bytes(LOGO).expect("assets/logo.png es un PNG válido"),
            )
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LogicSM",
        opciones,
        Box::new(|cc| {
            interfaz::estilo(&cc.egui_ctx);
            Ok(Box::new(interfaz::Interfaz::nueva(cc, proyecto)))
        }),
    )
}

/// En el navegador: dibuja en el `<canvas id="logicsm">` de index.html
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast;
    use eframe::web_sys;
    let lienzo = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("logicsm"))
        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .expect("index.html tiene un <canvas id=\"logicsm\">");
    wasm_bindgen_futures::spawn_local(async move {
        let inicio = eframe::WebRunner::new()
            .start(
                lienzo,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    interfaz::estilo(&cc.egui_ctx);
                    Ok(Box::new(interfaz::Interfaz::nueva(cc, None)))
                }),
            )
            .await;
        if let Err(e) = inicio {
            web_sys::console::error_1(&e);
        }
    });
}
