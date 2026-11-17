//! LogicSM: el IDE de escritorio (editor, lienzo del circuito y paneles).
// Sin ventana de consola extra al abrirlo en Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ayuda;
mod comandos;
mod interfaz;
mod proyecto;

use eframe::egui;
use std::path::PathBuf;
use std::process::ExitCode;

/// Logo de la app: ícono de la ventana y barra superior
pub const LOGO: &[u8] = include_bytes!("../../../assets/logo.png");

/// Ícono de los proyectos `.SA` (el mismo que les pone el instalador en el sistema)
pub const ICONO_SA: &[u8] = include_bytes!("../../../assets/archivo.png");

// logicsm (o logicsm open) abre la ventana y logicsm Proyecto.SA abre ese proyecto (doble click
// en el explorador); logicsm update | delete | help son comandos de terminal
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

fn ventana(proyecto: Option<PathBuf>) -> eframe::Result {
    let opciones = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
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
