//! LogicSM: el IDE de escritorio (editor, lienzo del circuito y paneles).
// Sin ventana de consola extra al abrirlo en Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod comandos;
mod interfaz;

use eframe::egui;
use std::process::ExitCode;

// logicsm (o logicsm open) abre la ventana; logicsm update | delete | help son comandos de terminal
fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None | Some("open") => match ventana() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                comandos::consola();
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Some(orden) => comandos::ejecutar(orden),
    }
}

fn ventana() -> eframe::Result {
    let opciones = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("LogicSM")
            // Igual al nombre de logicsm.desktop: así Linux (Wayland) le pone el ícono correcto
            .with_app_id("logicsm")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/logo.png"))
                    .expect("assets/logo.png es un PNG válido"),
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
            Ok(Box::new(interfaz::Interfaz::nueva()))
        }),
    )
}
