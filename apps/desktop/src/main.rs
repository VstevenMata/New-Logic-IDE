//! LogicSM: el IDE de escritorio (editor, lienzo del circuito y paneles).
// Sin ventana de consola extra al abrirlo en Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod comandos;

use eframe::egui;
use logicsm_sim::Compuerta;
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
            .with_inner_size([520.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LogicSM",
        opciones,
        Box::new(|_| Ok(Box::new(Demo::default()))),
    )
}

// ponytail: demo de una sola compuerta para probar el instalador; el IDE real llega en la fase 2
struct Demo {
    compuerta: Compuerta,
    entradas: Vec<bool>,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            compuerta: Compuerta::And,
            entradas: vec![false; 2],
        }
    }
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("LogicSM");
            ui.label(format!(
                "Versión {} · demo de compuertas",
                env!("CARGO_PKG_VERSION")
            ));
            ui.separator();

            egui::ComboBox::from_label("Compuerta")
                .selected_text(self.compuerta.nombre().to_uppercase())
                .show_ui(ui, |ui| {
                    for c in Compuerta::TODAS {
                        ui.selectable_value(&mut self.compuerta, c, c.nombre().to_uppercase());
                    }
                });

            let mut n = self.entradas.len();
            if self.compuerta.entradas_validas(1) {
                n = 1;
            } else {
                n = n.max(2);
                ui.add(egui::Slider::new(&mut n, 2..=8).text("entradas"));
            }
            self.entradas.resize(n, false);

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for (letra, valor) in ('A'..).zip(self.entradas.iter_mut()) {
                    ui.toggle_value(valor, format!("{letra} = {}", *valor as u8));
                }
            });

            ui.add_space(12.0);
            let salida = self.compuerta.evaluar(&self.entradas);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                let color = if salida {
                    egui::Color32::from_rgb(40, 200, 90)
                } else {
                    egui::Color32::from_gray(90)
                };
                ui.painter().circle_filled(rect.center(), 12.0, color);
                ui.label(format!("Y = {}", salida as u8));
            });
        });
    }
}
