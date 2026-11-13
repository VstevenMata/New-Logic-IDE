//! Interfaz de LogicSM: barra superior, proyecto, código, lienzo, entradas y salidas, y
//! pestañas inferiores. Diseño y paleta tomados de index.html.
//!
//! El código se compila en cada cambio y el lienzo dibuja el circuito al instante.

use crate::ayuda;
use crate::proyecto::Proyecto;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Shape,
    Stroke, StrokeKind, Theme, Vec2, pos2, vec2,
};
use logicsm_core::Compuerta;
use logicsm_core::netlist::{Netlist, Nodo, Tipo};
use std::collections::HashMap;

/// Colores de un tema. Hay dos: CLARO (pastel de index.html) y OSCURO.
struct Paleta {
    fondo: Color32,
    panel: Color32,
    tinta: Color32,
    tenue: Color32,
    linea: Color32,
    suave: Color32,
    lavanda: Color32,
    lavanda_fondo: Color32,
    salvia_fondo: Color32,
    salvia_borde: Color32,
    salvia_texto: Color32,
    codigo: Color32,
    /// Puntos de la cuadrícula del lienzo (premultiplicado)
    puntos: Color32,
    /// Cables y LEDs en 1 (en 0 usan `tenue`)
    uno: Color32,
}

const CLARO: Paleta = Paleta {
    fondo: Color32::from_rgb(0xf8, 0xf8, 0xf5),
    panel: Color32::WHITE,
    tinta: Color32::from_rgb(0x29, 0x2b, 0x2a),
    tenue: Color32::from_rgb(0x7f, 0x82, 0x7e),
    linea: Color32::from_rgb(0xe5, 0xe6, 0xe1),
    suave: Color32::from_rgb(0xf2, 0xf3, 0xef),
    lavanda: Color32::from_rgb(0xa9, 0x94, 0xd2),
    lavanda_fondo: Color32::from_rgb(0xf0, 0xeb, 0xf8),
    salvia_fondo: Color32::from_rgb(0xe6, 0xee, 0xe7),
    salvia_borde: Color32::from_rgb(0xd7, 0xe4, 0xd9),
    salvia_texto: Color32::from_rgb(0x42, 0x63, 0x4a),
    codigo: Color32::from_rgb(0xfb, 0xfb, 0xf9),
    // rgba(67, 76, 69, 0.075)
    puntos: Color32::from_rgba_premultiplied(5, 6, 5, 19),
    uno: Color32::from_rgb(0x4f, 0xa5, 0x60),
};

// Gris verdoso (no negro puro) para que el pastel siga combinando
const OSCURO: Paleta = Paleta {
    fondo: Color32::from_rgb(0x1d, 0x20, 0x1e),
    panel: Color32::from_rgb(0x25, 0x28, 0x26),
    tinta: Color32::from_rgb(0xe3, 0xe5, 0xe0),
    tenue: Color32::from_rgb(0x92, 0x97, 0x90),
    linea: Color32::from_rgb(0x36, 0x3a, 0x37),
    suave: Color32::from_rgb(0x2e, 0x32, 0x2f),
    lavanda: Color32::from_rgb(0xb9, 0xa6, 0xe2),
    lavanda_fondo: Color32::from_rgb(0x3a, 0x34, 0x4d),
    salvia_fondo: Color32::from_rgb(0x2b, 0x3b, 0x2f),
    salvia_borde: Color32::from_rgb(0x3d, 0x55, 0x43),
    salvia_texto: Color32::from_rgb(0xa9, 0xd2, 0xb2),
    codigo: Color32::from_rgb(0x21, 0x24, 0x22),
    // rgba(255, 255, 255, 0.06)
    puntos: Color32::from_rgba_premultiplied(15, 15, 15, 15),
    uno: Color32::from_rgb(0x7c, 0xc9, 0x8b),
};

/// La paleta del tema activo
fn paleta(ui: &egui::Ui) -> &'static Paleta {
    match ui.ctx().theme() {
        Theme::Light => &CLARO,
        Theme::Dark => &OSCURO,
    }
}

/// Colores y bordes de los widgets de egui para los dos temas.
/// Arranca con el tema del sistema; el botón de la barra lo cambia.
pub fn estilo(ctx: &egui::Context) {
    for (tema, c) in [(Theme::Light, &CLARO), (Theme::Dark, &OSCURO)] {
        ctx.style_mut_of(tema, |s| {
            let v = &mut s.visuals;
            v.panel_fill = c.panel;
            v.window_fill = c.panel;
            v.window_stroke = Stroke::new(1.0, c.linea);
            v.extreme_bg_color = c.codigo;
            v.override_text_color = Some(c.tinta);
            v.selection.bg_fill = c.lavanda_fondo;
            v.selection.stroke = Stroke::new(1.0, c.lavanda);
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.linea);
            for w in [
                &mut v.widgets.inactive,
                &mut v.widgets.hovered,
                &mut v.widgets.active,
            ] {
                w.corner_radius = CornerRadius::same(5);
                w.bg_stroke = Stroke::new(1.0, c.linea);
                w.expansion = 0.0;
            }
            v.widgets.hovered.bg_stroke = Stroke::new(1.0, c.lavanda);
            v.widgets.inactive.weak_bg_fill = c.panel;
            v.widgets.hovered.weak_bg_fill = c.suave;
            v.widgets.active.weak_bg_fill = c.suave;
            s.spacing.button_padding = vec2(9.0, 5.0);
        });
    }
}

/// Lo que se hace después de confirmar que se pueden perder los cambios sin guardar
#[derive(Clone, Copy)]
enum Accion {
    Nuevo,
    Abrir,
    Salir,
}

/// Ventanas del menú Ayuda
#[derive(Clone, Copy)]
enum Ventana {
    Guia,
    Atajos,
    Actualizar,
    Acerca,
}

#[derive(Clone, Copy, PartialEq)]
enum Pestana {
    Problemas,
    Tabla,
    Ondas,
    Salida,
}

/// Aviso para Ejecutar y Paso a paso mientras no exista `clock`
const SIN_RELOJ: &str = "Llega con clock. Por ahora la simulación corre sola al escribir.";

pub struct Interfaz {
    proyecto: Proyecto,
    /// Último circuito que compiló; se queda mientras el código tiene errores.
    red: Option<Netlist>,
    problemas: Vec<String>,
    /// Valor de cada `in`, en orden
    entradas: Vec<bool>,
    /// Valor de cada nodo de `red` (guarda la memoria de los flip-flops)
    valores: Vec<bool>,
    estable: bool,
    /// Mensajes de la pestaña Salida (archivos abiertos, guardados, errores)
    salida: Vec<String>,
    pestana: Pestana,
    ver_proyecto: bool,
    ver_senales: bool,
    ver_inferior: bool,
    ventana: Option<Ventana>,
    /// Acción esperando confirmación porque hay cambios sin guardar
    confirmar: Option<Accion>,
    /// Archivo esperando confirmación para eliminarlo
    borrar: Option<usize>,
    /// Nombre que se escribe para un archivo nuevo (`None`) o para renombrar uno
    nombrando: Option<(Option<usize>, String)>,
    error_nombre: Option<String>,
    /// Ya se confirmó la salida: no volver a preguntar al cerrar
    saliendo: bool,
    /// Evento del menú Editar (deshacer, copiar…) que recibe el editor
    para_editor: Option<egui::Event>,
    logo: egui::TextureHandle,
    /// Piezas del lienzo que el usuario movió, por su clave (ver `clave`), relativas
    /// a la esquina del lienzo
    posiciones: HashMap<String, Pos2>,
}

impl Interfaz {
    pub fn nueva(ctx: &egui::Context) -> Self {
        let icono = eframe::icon_data::from_png_bytes(crate::LOGO)
            .expect("assets/logo.png es un PNG válido");
        let imagen = egui::ColorImage::from_rgba_unmultiplied(
            [icono.width as usize, icono.height as usize],
            &icono.rgba,
        );
        let mut s = Self {
            proyecto: Proyecto::nuevo(),
            red: None,
            problemas: Vec::new(),
            entradas: Vec::new(),
            valores: Vec::new(),
            estable: true,
            salida: Vec::new(),
            pestana: Pestana::Problemas,
            ver_proyecto: true,
            ver_senales: true,
            ver_inferior: true,
            ventana: None,
            confirmar: None,
            borrar: None,
            nombrando: None,
            error_nombre: None,
            saliendo: false,
            para_editor: None,
            logo: ctx.load_texture("logo", imagen, egui::TextureOptions::LINEAR),
            posiciones: HashMap::new(),
        };
        s.compilar();
        s
    }

    fn compilar(&mut self) {
        match self.proyecto.compilar() {
            Ok(red) => {
                self.problemas.clear();
                let n = red
                    .nodos
                    .iter()
                    .filter(|n| matches!(n.tipo, Tipo::Entrada(_)))
                    .count();
                self.entradas.resize(n, false);
                self.valores.clear();
                self.red = Some(red);
            }
            Err(problemas) => self.problemas = problemas,
        }
    }

    /// Se cambió de archivo o de proyecto: se dibuja desde cero
    fn recompilar_desde_cero(&mut self) {
        self.red = None;
        self.entradas.clear();
        self.posiciones.clear();
        self.compilar();
    }

    fn abrir_archivo(&mut self, i: usize) {
        self.proyecto.activo = i;
        self.recompilar_desde_cero();
    }

    /// Mensaje en la pestaña Salida, que se muestra
    fn aviso(&mut self, mensaje: String) {
        self.salida.push(mensaje);
        self.pestana = Pestana::Salida;
        self.ver_inferior = true;
    }

    /// Hace la acción, o primero pregunta si hay cambios sin guardar
    fn pedir(&mut self, ctx: &egui::Context, accion: Accion) {
        if self.proyecto.sin_guardar() {
            self.confirmar = Some(accion);
        } else {
            self.hacer(ctx, accion);
        }
    }

    fn hacer(&mut self, ctx: &egui::Context, accion: Accion) {
        match accion {
            Accion::Nuevo => {
                self.proyecto = Proyecto::nuevo();
                self.recompilar_desde_cero();
            }
            Accion::Abrir => {
                let Some(carpeta) = rfd::FileDialog::new()
                    .set_title("Abrir la carpeta del proyecto")
                    .pick_folder()
                else {
                    return;
                };
                match Proyecto::abrir(carpeta.clone()) {
                    Ok(proyecto) => {
                        self.salida.push(format!("Abierto: {}", carpeta.display()));
                        self.proyecto = proyecto;
                        self.recompilar_desde_cero();
                    }
                    Err(e) => self.aviso(format!("No se pudo abrir {}: {e}", carpeta.display())),
                }
            }
            Accion::Salir => {
                self.saliendo = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Guarda en la carpeta del proyecto, o pregunta cuál (`otra` o si nunca se guardó).
    /// Devuelve `false` si no se guardó.
    fn guardar(&mut self, otra: bool) -> bool {
        let nueva = if otra || self.proyecto.carpeta.is_none() {
            let Some(carpeta) = rfd::FileDialog::new()
                .set_title("Carpeta donde guardar el proyecto")
                .pick_folder()
            else {
                return false;
            };
            Some(carpeta)
        } else {
            None
        };
        match self.proyecto.guardar(nueva) {
            Ok(()) => {
                if let Some(carpeta) = &self.proyecto.carpeta {
                    self.salida
                        .push(format!("Guardado en {}", carpeta.display()));
                }
                true
            }
            Err(e) => {
                self.aviso(e);
                false
            }
        }
    }

    fn agregar_ejemplo(&mut self, archivos: &[(&str, &str)]) {
        for aviso in self.proyecto.agregar(archivos) {
            self.aviso(aviso);
        }
        let nombres: Vec<&str> = archivos.iter().map(|(nombre, _)| *nombre).collect();
        self.salida
            .push(format!("Agregado al proyecto: {}", nombres.join(", ")));
        self.recompilar_desde_cero();
    }

    /// Entradas en 0 y se borra la memoria de los flip-flops
    fn reiniciar(&mut self) {
        self.entradas.fill(false);
        self.valores.clear();
    }

    fn atajos(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let tecla = |m: Modifiers, k: Key| {
            ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)))
        };
        let ctrl = Modifiers::COMMAND;
        // Primero la versión con Shift: Ctrl+S también aceptaría Ctrl+Shift+S
        if tecla(ctrl | Modifiers::SHIFT, Key::S) {
            self.guardar(true);
        } else if tecla(ctrl, Key::S) {
            self.guardar(false);
        }
        if tecla(ctrl, Key::N) {
            self.pedir(ctx, Accion::Nuevo);
        }
        if tecla(ctrl, Key::O) {
            self.pedir(ctx, Accion::Abrir);
        }
        if tecla(ctrl, Key::Q) {
            self.pedir(ctx, Accion::Salir);
        }
        if tecla(ctrl, Key::B) {
            self.ver_proyecto = !self.ver_proyecto;
        }
        if tecla(ctrl, Key::J) {
            self.ver_inferior = !self.ver_inferior;
        }
        if tecla(ctrl | Modifiers::SHIFT, Key::F5) {
            self.reiniciar();
        }
        if tecla(Modifiers::NONE, Key::F1) {
            self.ventana = Some(Ventana::Guia);
        }
        // Cerrar con la X de la ventana también pregunta
        let cerrar = ctx.input(|i| i.viewport().close_requested());
        if cerrar && !self.saliendo && self.proyecto.sin_guardar() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirmar = Some(Accion::Salir);
        }
    }

    /// Diálogos "Hay cambios sin guardar" y "¿Eliminar el archivo?"
    fn confirmacion(&mut self, ctx: &egui::Context) {
        if let Some(i) = self.borrar {
            let nombre = self.proyecto.archivos[i].nombre.clone();
            let mut eliminar = None;
            let r = egui::Modal::new(egui::Id::unique("borrar")).show(ctx, |ui| {
                ui.set_width(340.0);
                ui.label(
                    RichText::new(format!("¿Eliminar {nombre}?"))
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label("También se borra de la carpeta del proyecto.");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Eliminar").clicked() {
                        eliminar = Some(true);
                    }
                    if ui.button("Cancelar").clicked() {
                        eliminar = Some(false);
                    }
                });
            });
            if r.should_close() && eliminar.is_none() {
                eliminar = Some(false);
            }
            if let Some(eliminar) = eliminar {
                self.borrar = None;
                if eliminar {
                    match self.proyecto.borrar(i) {
                        Ok(()) => self.salida.push(format!("Eliminado: {nombre}")),
                        Err(e) => self.aviso(e),
                    }
                    self.recompilar_desde_cero();
                }
            }
        }

        let Some(accion) = self.confirmar else {
            return;
        };
        enum Respuesta {
            Guardar,
            Descartar,
            Cancelar,
        }
        let mut respuesta = None;
        let r = egui::Modal::new(egui::Id::unique("confirmar")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(RichText::new("Hay cambios sin guardar").size(14.0).strong());
            ui.add_space(4.0);
            ui.label("¿Quieres guardarlos antes de continuar?");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Guardar").clicked() {
                    respuesta = Some(Respuesta::Guardar);
                }
                if ui.button("No guardar").clicked() {
                    respuesta = Some(Respuesta::Descartar);
                }
                if ui.button("Cancelar").clicked() {
                    respuesta = Some(Respuesta::Cancelar);
                }
            });
        });
        if r.should_close() && respuesta.is_none() {
            respuesta = Some(Respuesta::Cancelar);
        }
        let Some(respuesta) = respuesta else {
            return;
        };
        self.confirmar = None;
        let seguir = match respuesta {
            Respuesta::Guardar => self.guardar(false),
            Respuesta::Descartar => true,
            Respuesta::Cancelar => false,
        };
        if seguir {
            self.hacer(ctx, accion);
        }
    }

    fn ventanas(&mut self, ctx: &egui::Context) {
        let Some(ventana) = self.ventana else {
            return;
        };
        let titulo = match ventana {
            Ventana::Guia => "Guía del lenguaje",
            Ventana::Atajos => "Atajos de teclado",
            Ventana::Actualizar => "Buscar actualizaciones",
            Ventana::Acerca => "Acerca de LogicSM",
        };
        let mut abierta = true;
        egui::Window::new(titulo)
            .open(&mut abierta)
            .collapsible(false)
            .default_size([560.0, 520.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match ventana {
                    Ventana::Guia => ayuda::guia(ui),
                    Ventana::Atajos => ayuda::atajos(ui),
                    Ventana::Actualizar => ayuda::actualizar(ui),
                    Ventana::Acerca => ayuda::acerca(ui, &self.logo),
                });
            });
        if !abierta {
            self.ventana = None;
        }
    }
}

impl eframe::App for Interfaz {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.atajos(&ctx);
        if let Some(red) = &self.red {
            self.estable = logicsm_sim::simular(red, &mut self.valores, &self.entradas);
        }
        let c = paleta(ui);
        egui::Panel::top("barra")
            .exact_size(48.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(c.panel)
                    .inner_margin(Margin::symmetric(14, 0)),
            )
            .show(ui, |ui| self.barra_superior(ui));
        if self.ver_proyecto {
            egui::Panel::left("proyecto")
                .exact_size(190.0)
                .resizable(false)
                .frame(
                    Frame::new()
                        .fill(c.panel)
                        .inner_margin(Margin::symmetric(14, 15)),
                )
                .show(ui, |ui| self.panel_proyecto(ui));
        }
        if self.ver_senales {
            egui::Panel::right("senales")
                .exact_size(222.0)
                .resizable(false)
                .frame(
                    Frame::new()
                        .fill(c.panel)
                        .inner_margin(Margin::symmetric(14, 15)),
                )
                .show(ui, |ui| self.senales(ui));
        }
        if self.ver_inferior {
            egui::Panel::bottom("pestanas")
                .exact_size(166.0)
                .resizable(false)
                .frame(Frame::new().fill(c.panel))
                .show(ui, |ui| self.pestanas(ui));
        }
        egui::Panel::left("codigo")
            .default_size(420.0)
            .resizable(true)
            .frame(Frame::new().fill(c.codigo))
            .show(ui, |ui| self.editor(ui));
        egui::CentralPanel::default()
            .frame(Frame::new().fill(c.fondo))
            .show(ui, |ui| self.lienzo(ui));
        self.ventanas(&ctx);
        self.confirmacion(&ctx);
    }
}

/// Formatos de exportación (menú Archivo y botón Exportar de Proyecto)
// ponytail: deshabilitados hasta que exista cada exportador
const FORMATOS: [&str; 3] = ["Imagen (PNG)", "Tabla de verdad (CSV)", "Verilog (.v)"];

impl Interfaz {
    /// Logo, nombre y menú a la izquierda, controles de simulación y el tema a la derecha
    fn barra_superior(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.image((self.logo.id(), vec2(22.0, 22.0)));
            ui.label(RichText::new("LogicSM").size(14.0).strong());
            ui.add_space(6.0);
            self.menu(ui);
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            boton(ui, "Ejecutar", Icono::Ejecutar, true).on_hover_text(SIN_RELOJ);
            boton(ui, "Paso a paso", Icono::Paso, false).on_hover_text(SIN_RELOJ);
            if boton(ui, "Reiniciar", Icono::Reiniciar, false)
                .on_hover_text("Pone las entradas en 0 y borra la memoria")
                .clicked()
            {
                self.reiniciar();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let oscuro = ui.ctx().theme() == Theme::Dark;
                let (texto, icono, otro) = if oscuro {
                    ("Claro", Icono::Sol, Theme::Light)
                } else {
                    ("Oscuro", Icono::Luna, Theme::Dark)
                };
                if boton(ui, texto, icono, false).clicked() {
                    ui.ctx().set_theme(otro);
                }
            });
        });
    }

    /// Menú principal al estilo de VS Code
    fn menu(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let opcion = |ui: &mut egui::Ui, texto: &str, atajo: &str| {
            ui.add(egui::Button::new(texto).shortcut_text(atajo))
                .clicked()
        };
        let tecla = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        };

        ui.menu_button("Archivo", |ui| {
            if opcion(ui, "Nuevo proyecto", "Ctrl+N") {
                self.pedir(&ctx, Accion::Nuevo);
            }
            if opcion(ui, "Abrir carpeta…", "Ctrl+O") {
                self.pedir(&ctx, Accion::Abrir);
            }
            if opcion(ui, "Nuevo archivo", "") {
                self.ver_proyecto = true;
                self.nombrando = Some((None, String::new()));
                self.error_nombre = None;
            }
            ui.separator();
            if opcion(ui, "Guardar", "Ctrl+S") {
                self.guardar(false);
            }
            if opcion(ui, "Guardar en otra carpeta…", "Ctrl+Shift+S") {
                self.guardar(true);
            }
            ui.menu_button("Exportar", |ui| {
                for formato in FORMATOS {
                    ui.add_enabled(false, egui::Button::new(formato));
                }
            });
            ui.separator();
            if opcion(ui, "Salir", "Ctrl+Q") {
                self.pedir(&ctx, Accion::Salir);
            }
        });
        ui.menu_button("Editar", |ui| {
            if opcion(ui, "Deshacer", "Ctrl+Z") {
                self.para_editor = Some(tecla(egui::Key::Z));
            }
            if opcion(ui, "Rehacer", "Ctrl+Y") {
                self.para_editor = Some(tecla(egui::Key::Y));
            }
            ui.separator();
            if opcion(ui, "Cortar", "Ctrl+X") {
                self.para_editor = Some(egui::Event::Cut);
            }
            if opcion(ui, "Copiar", "Ctrl+C") {
                self.para_editor = Some(egui::Event::Copy);
            }
            if opcion(ui, "Pegar", "Ctrl+V") {
                match arboard::Clipboard::new().and_then(|mut p| p.get_text()) {
                    Ok(texto) => self.para_editor = Some(egui::Event::Paste(texto)),
                    Err(e) => self.aviso(format!("No se pudo leer el portapapeles: {e}")),
                }
            }
            ui.separator();
            if opcion(ui, "Seleccionar todo", "Ctrl+A") {
                self.para_editor = Some(tecla(egui::Key::A));
            }
        });
        ui.menu_button("Ver", |ui| {
            // Igual que en VS Code, el zoom es de toda la ventana
            if opcion(ui, "Acercar", "Ctrl++") {
                ctx.set_zoom_factor((ctx.zoom_factor() + 0.1).min(3.0));
            }
            if opcion(ui, "Alejar", "Ctrl+-") {
                ctx.set_zoom_factor((ctx.zoom_factor() - 0.1).max(0.5));
            }
            if opcion(ui, "Tamaño normal", "Ctrl+0") {
                ctx.set_zoom_factor(1.0);
            }
            ui.separator();
            ui.checkbox(&mut self.ver_proyecto, "Panel de proyecto  (Ctrl+B)");
            ui.checkbox(&mut self.ver_senales, "Entradas y salidas");
            ui.checkbox(&mut self.ver_inferior, "Panel inferior  (Ctrl+J)");
        });
        ui.menu_button("Simulación", |ui| {
            ui.add_enabled(false, egui::Button::new("Ejecutar").shortcut_text("F5"))
                .on_disabled_hover_text(SIN_RELOJ);
            ui.add_enabled(false, egui::Button::new("Paso a paso").shortcut_text("F10"))
                .on_disabled_hover_text(SIN_RELOJ);
            if opcion(ui, "Reiniciar", "Ctrl+Shift+F5") {
                self.reiniciar();
            }
            ui.separator();
            if opcion(ui, "Tabla de verdad", "") {
                self.pestana = Pestana::Tabla;
                self.ver_inferior = true;
            }
            if opcion(ui, "Ondas", "") {
                self.pestana = Pestana::Ondas;
                self.ver_inferior = true;
            }
        });
        ui.menu_button("Ayuda", |ui| {
            if opcion(ui, "Guía del lenguaje", "F1") {
                self.ventana = Some(Ventana::Guia);
            }
            ui.menu_button("Ejemplos", |ui| self.lista_de_ejemplos(ui));
            if opcion(ui, "Atajos de teclado", "") {
                self.ventana = Some(Ventana::Atajos);
            }
            ui.separator();
            if opcion(ui, "Buscar actualizaciones", "") {
                self.ventana = Some(Ventana::Actualizar);
            }
            if opcion(ui, "Reportar un problema", "") {
                ctx.open_url(egui::OpenUrl::new_tab(concat!(
                    env!("CARGO_PKG_REPOSITORY"),
                    "/issues/new"
                )));
            }
            ui.separator();
            if opcion(ui, "Acerca de LogicSM", "") {
                self.ventana = Some(Ventana::Acerca);
            }
        });
    }

    /// Circuitos clásicos que se agregan al proyecto (menú Ayuda y panel Proyecto)
    fn lista_de_ejemplos(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled(false, egui::Label::new("Se agregan al proyecto:"));
        for (titulo, archivos) in ayuda::EJEMPLOS {
            if ui.button(*titulo).clicked() {
                self.agregar_ejemplo(archivos);
            }
        }
    }

    fn panel_proyecto(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        titulo(ui, "Proyecto");
        let ancho = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
        let accion = |ui: &mut egui::Ui, texto: &str| {
            ui.add(egui::Button::new(RichText::new(texto).size(12.0)).min_size(vec2(ancho, 28.0)))
        };
        ui.horizontal(|ui| {
            if accion(ui, "Nuevo")
                .on_hover_text("Nuevo proyecto (Ctrl+N)")
                .clicked()
            {
                self.pedir(&ctx, Accion::Nuevo);
            }
            if accion(ui, "Abrir")
                .on_hover_text("Abrir la carpeta de un proyecto (Ctrl+O)")
                .clicked()
            {
                self.pedir(&ctx, Accion::Abrir);
            }
        });
        ui.horizontal(|ui| {
            if accion(ui, "Guardar")
                .on_hover_text("Guardar todos los archivos (Ctrl+S)")
                .clicked()
            {
                self.guardar(false);
            }
            let exportar = accion(ui, "Exportar ▾");
            egui::Popup::menu(&exportar).show(|ui| {
                for formato in FORMATOS {
                    ui.add_enabled(false, egui::Button::new(formato));
                }
            });
        });
        ui.add_space(14.0);

        // Carpeta y botón para crear archivos
        let carpeta = match &self.proyecto.carpeta {
            Some(c) => c
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            None => "Sin guardar".into(),
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(carpeta).size(12.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("+")
                    .on_hover_text("Nuevo archivo")
                    .clicked()
                {
                    self.nombrando = Some((None, String::new()));
                    self.error_nombre = None;
                }
            });
        });
        ui.add_space(4.0);
        if matches!(self.nombrando, Some((None, _))) {
            self.campo_nombre(ui);
        }

        // Archivos
        let nombres: Vec<String> = self
            .proyecto
            .archivos
            .iter()
            .map(|a| {
                let marca = if a.sin_guardar() { " ●" } else { "" };
                format!("{}{marca}", a.nombre)
            })
            .collect();
        egui::ScrollArea::vertical()
            .max_height((ui.available_height() - 50.0).max(60.0))
            .show(ui, |ui| {
                for (i, texto) in nombres.into_iter().enumerate() {
                    if matches!(self.nombrando, Some((Some(j), _)) if j == i) {
                        self.campo_nombre(ui);
                        continue;
                    }
                    let boton = egui::Button::selectable(i == self.proyecto.activo, texto)
                        .min_size(vec2(ui.available_width(), 0.0));
                    let r = ui.add(boton);
                    if r.clicked() && i != self.proyecto.activo {
                        self.abrir_archivo(i);
                    }
                    r.context_menu(|ui| {
                        if ui.button("Renombrar…").clicked() {
                            let a = &self.proyecto.archivos[i].nombre;
                            let base = a.strip_suffix(".SA").unwrap_or(a).to_string();
                            self.nombrando = Some((Some(i), base));
                            self.error_nombre = None;
                        }
                        if ui.button("Eliminar…").clicked() {
                            self.borrar = Some(i);
                        }
                    });
                }
            });

        ui.add_space(8.0);
        let ejemplos = ui.add(
            egui::Button::new(RichText::new("Agregar ejemplo ▾").size(12.0))
                .min_size(vec2(ui.available_width(), 28.0)),
        );
        egui::Popup::menu(&ejemplos).show(|ui| self.lista_de_ejemplos(ui));
    }

    /// Campo de texto para el nombre de un archivo nuevo o del que se renombra.
    /// Enter lo confirma; Escape o un click afuera lo cancela.
    fn campo_nombre(&mut self, ui: &mut egui::Ui) {
        let Some((destino, mut nombre)) = self.nombrando.take() else {
            return;
        };
        let r = ui.add(
            egui::TextEdit::singleline(&mut nombre)
                .hint_text("Nombre (ej. Mux4)")
                .desired_width(f32::INFINITY),
        );
        if r.changed() {
            self.error_nombre = None;
        }
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        if r.lost_focus() && enter {
            let hecho = match destino {
                None => self.proyecto.crear(&nombre),
                Some(i) => self.proyecto.renombrar(i, &nombre),
            };
            match hecho {
                Ok(()) => {
                    // Al renombrar sigue el mismo circuito: solo cambian los nombres en Problemas
                    if destino.is_none() {
                        self.recompilar_desde_cero();
                    } else {
                        self.compilar();
                    }
                    return;
                }
                Err(e) => self.error_nombre = Some(e),
            }
        } else if r.lost_focus() {
            self.error_nombre = None;
            return;
        } else if !r.has_focus() {
            r.request_focus();
        }
        if let Some(error) = &self.error_nombre {
            ui.label(RichText::new(error).size(11.0).color(paleta(ui).tenue));
        }
        ui.add_space(4.0);
        self.nombrando = Some((destino, nombre));
    }

    /// Nombre del archivo abierto para la cabecera del editor
    fn nombre_archivo(&self) -> String {
        let a = self.proyecto.actual();
        let marca = if a.sin_guardar() { " ●" } else { "" };
        format!("Código — {}{marca}", a.nombre)
    }
}

/// Título de sección ("Proyecto", "Entradas"…)
fn titulo(ui: &mut egui::Ui, texto: &str) {
    ui.label(RichText::new(texto).size(12.0).strong());
    ui.add_space(8.0);
}

/// Texto tenue para secciones vacías
fn vacio(ui: &mut egui::Ui, texto: &str) {
    ui.label(RichText::new(texto).size(11.5).color(paleta(ui).tenue));
}

impl Interfaz {
    /// Interruptores de las entradas y valor de las salidas
    fn senales(&mut self, ui: &mut egui::Ui) {
        let c = paleta(ui);
        let Some(red) = &self.red else {
            titulo(ui, "Entradas");
            vacio(ui, "Aparecen al escribir un circuito.");
            return;
        };
        titulo(ui, "Entradas");
        let mut k = 0;
        for n in &red.nodos {
            if let Tipo::Entrada(nombre) = &n.tipo {
                let v = &mut self.entradas[k];
                let texto = format!("{nombre} = {}", *v as u8);
                ui.toggle_value(v, texto);
                k += 1;
            }
        }
        ui.add_space(24.0);
        titulo(ui, "Salidas");
        for (n, &v) in red.nodos.iter().zip(&self.valores) {
            if let Tipo::Salida(nombre) = &n.tipo {
                let color = if v { c.uno } else { c.tenue };
                ui.label(
                    RichText::new(format!("{nombre} = {}", v as u8))
                        .monospace()
                        .color(color),
                );
            }
        }
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        cabecera(ui, &self.nombre_archivo());
        // Un id por archivo: cada uno tiene su propio historial de deshacer
        let id = egui::Id::unique(("editor", self.proyecto.actual().nombre.as_str()));
        // Lo que pidió el menú Editar: se le da el foco al editor y se le pasa el evento
        if let Some(evento) = self.para_editor.take() {
            ui.memory_mut(|m| m.request_focus(id));
            ui.input_mut(|i| i.events.push(evento));
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            let r = ui.add(
                egui::TextEdit::multiline(&mut self.proyecto.actual_mut().codigo)
                    .id(id)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(40),
            );
            if r.changed() {
                self.compilar();
            }
        });
    }

    fn lienzo(&mut self, ui: &mut egui::Ui) {
        let c = paleta(ui);
        cabecera(ui, "Circuito");
        let rect = ui.available_rect_before_wrap();
        let p = ui.painter_at(rect);

        let mut y = rect.top() + 9.0;
        while y < rect.bottom() {
            let mut x = rect.left() + 9.0;
            while x < rect.right() {
                p.circle_filled(pos2(x, y), 0.9, c.puntos);
                x += 18.0;
            }
            y += 18.0;
        }
        match &self.red {
            Some(red) => dibujar(
                ui,
                rect,
                red,
                &self.valores,
                &mut self.entradas,
                &mut self.posiciones,
            ),
            None => {
                p.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Escribe un circuito en el editor",
                    FontId::monospace(13.0),
                    c.tenue,
                );
            }
        }

        // Controles del lienzo, abajo a la derecha
        let mut x = rect.right() - 12.0;
        let y = rect.bottom() - 39.0;
        for (texto, ancho) in [
            ("Reordenar", 82.0),
            ("Ajustar", 64.0),
            ("−", 29.0),
            ("+", 29.0),
        ] {
            x -= ancho;
            let r = ui.put(
                Rect::from_min_size(pos2(x, y), vec2(ancho, 27.0)),
                egui::Button::new(RichText::new(texto).size(11.0).color(c.tenue)),
            );
            // ponytail: Ajustar y el zoom siguen sin función
            if texto == "Reordenar"
                && r.on_hover_text("Devuelve las piezas movidas a su lugar")
                    .clicked()
            {
                self.posiciones.clear();
            }
            x -= 4.0;
        }
    }

    fn pestanas(&mut self, ui: &mut egui::Ui) {
        let c = paleta(ui);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 37.0), Sense::hover());
        let p = ui.painter();
        p.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, c.linea),
        );
        let mut problemas = self.problemas.clone();
        if !self.estable {
            problemas.push("El circuito oscila: nunca llega a un valor estable.".into());
        }
        let titulo_problemas = match problemas.len() {
            0 => "Problemas".to_string(),
            n => format!("Problemas ({n})"),
        };
        let mut x = rect.left() + 15.0;
        for (pestana, nombre) in [
            (Pestana::Problemas, titulo_problemas.as_str()),
            (Pestana::Tabla, "Tabla de verdad"),
            (Pestana::Ondas, "Ondas"),
            (Pestana::Salida, "Salida"),
        ] {
            let activa = self.pestana == pestana;
            let r = p.text(
                pos2(x, rect.center().y),
                Align2::LEFT_CENTER,
                nombre,
                FontId::proportional(12.0),
                if activa { c.tinta } else { c.tenue },
            );
            let zona = Rect::from_x_y_ranges(r.x_range().expand(6.0), rect.y_range());
            if ui
                .interact(zona, ui.scope_id().with(nombre), Sense::click())
                .clicked()
            {
                self.pestana = pestana;
            }
            if activa {
                let y = rect.bottom() - 1.0;
                p.line_segment(
                    [pos2(r.left(), y), pos2(r.right(), y)],
                    Stroke::new(2.0, c.lavanda),
                );
            }
            x = r.right() + 21.0;
        }
        ui.add_space(10.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.vertical(|ui| match self.pestana {
                    Pestana::Problemas => {
                        if problemas.is_empty() {
                            vacio(ui, "Sin problemas.");
                        }
                        for problema in &problemas {
                            ui.label(RichText::new(problema).size(12.0).monospace());
                        }
                    }
                    Pestana::Tabla => self.tabla_de_verdad(ui),
                    Pestana::Ondas => vacio(ui, "Las ondas llegan con clock y probe."),
                    Pestana::Salida => {
                        if self.salida.is_empty() {
                            vacio(ui, "Sin mensajes.");
                        }
                        for mensaje in &self.salida {
                            ui.label(RichText::new(mensaje).size(12.0).monospace());
                        }
                    }
                });
            });
        });
    }

    /// Todas las combinaciones de entradas y lo que vale cada salida
    // ponytail: cada fila parte de 0, así que en circuitos con memoria no muestra el estado guardado
    fn tabla_de_verdad(&self, ui: &mut egui::Ui) {
        let c = paleta(ui);
        let Some(red) = &self.red else {
            vacio(ui, "Escribe un circuito para ver su tabla de verdad.");
            return;
        };
        let entradas: Vec<&str> = red
            .nodos
            .iter()
            .filter_map(|n| match &n.tipo {
                Tipo::Entrada(nombre) => Some(nombre.as_str()),
                _ => None,
            })
            .collect();
        let salidas: Vec<(usize, &str)> = red
            .nodos
            .iter()
            .enumerate()
            .filter_map(|(i, n)| match &n.tipo {
                Tipo::Salida(nombre) => Some((i, nombre.as_str())),
                _ => None,
            })
            .collect();
        let n = entradas.len();
        if n > 8 {
            vacio(ui, "Con más de 8 entradas la tabla sería demasiado larga.");
            return;
        }
        let bit = |v: bool| RichText::new(if v { "1" } else { "0" }).monospace();
        egui::Grid::new("tabla_de_verdad")
            .striped(true)
            .spacing(vec2(18.0, 3.0))
            .show(ui, |ui| {
                for nombre in &entradas {
                    ui.label(RichText::new(*nombre).monospace().strong());
                }
                for (_, nombre) in &salidas {
                    ui.label(RichText::new(*nombre).monospace().strong().color(c.lavanda));
                }
                ui.end_row();
                let mut valores = Vec::new();
                for fila in 0..1usize << n {
                    let e: Vec<bool> = (0..n).map(|i| (fila >> (n - 1 - i)) & 1 == 1).collect();
                    valores.clear();
                    logicsm_sim::simular(red, &mut valores, &e);
                    for &v in &e {
                        ui.label(bit(v));
                    }
                    for &(i, _) in &salidas {
                        let v = valores[i];
                        ui.label(bit(v).color(if v { c.uno } else { c.tenue }));
                    }
                    ui.end_row();
                }
            });
    }
}

// ---------- Dibujo del circuito ----------

const ANCHO_COLUMNA: f32 = 130.0;
const ALTO_FILA: f32 = 64.0;

fn tamano(n: &Nodo) -> Vec2 {
    match n.tipo {
        Tipo::Compuerta(_) => vec2(46.0, (n.entradas.len() as f32 * 14.0 + 8.0).max(36.0)),
        Tipo::Entrada(_) => vec2(30.0, 24.0),
        Tipo::Constante(_) => vec2(22.0, 22.0),
        Tipo::Salida(_) => vec2(24.0, 24.0),
    }
}

fn invertida(c: Compuerta) -> bool {
    matches!(
        c,
        Compuerta::Not | Compuerta::Nand | Compuerta::Nor | Compuerta::Xnor
    )
}

/// Columna de cada pieza, al estilo de una red neuronal: lo que no tiene entradas a
/// la izquierda, cada pieza una columna después de la más lejana que la alimenta y
/// las salidas al final. `fuentes[i]` son las piezas que alimentan a `i`.
// ponytail: dentro de cada columna van en orden de aparición, sin minimizar cruces
fn columnas(fuentes: &[Vec<usize>], salida: &[bool]) -> Vec<usize> {
    fn profundidad(
        i: usize,
        fuentes: &[Vec<usize>],
        col: &mut [Option<usize>],
        visitando: &mut [bool],
    ) -> usize {
        if let Some(c) = col[i] {
            return c;
        }
        if visitando[i] {
            return 0; // realimentación: no empuja la columna
        }
        visitando[i] = true;
        let c = fuentes[i]
            .iter()
            .map(|&f| profundidad(f, fuentes, col, visitando) + 1)
            .max()
            .unwrap_or(0);
        visitando[i] = false;
        col[i] = Some(c);
        c
    }

    let total = fuentes.len();
    let mut col = vec![None; total];
    let mut visitando = vec![false; total];
    let mut cols: Vec<usize> = (0..total)
        .map(|i| profundidad(i, fuentes, &mut col, &mut visitando))
        .collect();
    let ultima = cols
        .iter()
        .zip(salida)
        .filter(|&(_, &s)| !s)
        .map(|(&c, _)| c + 1)
        .max()
        .unwrap_or(0);
    for (c, &s) in cols.iter_mut().zip(salida) {
        if s {
            *c = ultima;
        }
    }
    cols
}

/// Curva de Bézier cuadrática de `a` a `b`
fn curva(a: Pos2, control: Pos2, b: Pos2) -> impl Iterator<Item = Pos2> {
    (0..=12).map(move |i| {
        let s = i as f32 / 12.0;
        let u = 1.0 - s;
        (a.to_vec2() * u * u + control.to_vec2() * 2.0 * u * s + b.to_vec2() * s * s).to_pos2()
    })
}

/// Contorno de la compuerta dentro de `r` (sin la burbuja de negación)
fn contorno(c: Compuerta, r: Rect) -> Vec<Pos2> {
    let (l, t, b, w, cy) = (r.left(), r.top(), r.bottom(), r.width(), r.center().y);
    match c {
        Compuerta::And | Compuerta::Nand => {
            let mut puntos = vec![pos2(l, t)];
            puntos.extend((0..=16).map(|i| {
                let a = (-90.0 + 180.0 * i as f32 / 16.0_f32).to_radians();
                pos2(
                    l + w / 2.0 + w / 2.0 * a.cos(),
                    cy + r.height() / 2.0 * a.sin(),
                )
            }));
            puntos.push(pos2(l, b));
            puntos
        }
        Compuerta::Or | Compuerta::Nor | Compuerta::Xor | Compuerta::Xnor => {
            // La XOR deja espacio para su curva extra
            let l = if matches!(c, Compuerta::Xor | Compuerta::Xnor) {
                l + 6.0
            } else {
                l
            };
            let w = r.right() - l;
            let punta = pos2(l + w, cy);
            curva(pos2(l, t), pos2(l + w * 0.25, cy), pos2(l, b))
                .chain(curva(pos2(l, b), pos2(l + w * 0.6, b), punta))
                .chain(curva(punta, pos2(l + w * 0.6, t), pos2(l, t)))
                .collect()
        }
        Compuerta::Not | Compuerta::Buffer => vec![pos2(l, t), pos2(l + w, cy), pos2(l, b)],
    }
}

/// Lo que se ve en el lienzo: un nodo suelto o la caja de una instancia (sus
/// compuertas van adentro y no se dibujan).
#[derive(Clone, Copy)]
enum Pieza {
    Nodo(usize),
    Caja(usize),
}

/// Para recordar dónde la dejó el usuario aunque el código cambie
fn clave(red: &Netlist, pieza: Pieza) -> String {
    match pieza {
        Pieza::Caja(g) => format!("caja {}", red.instancias[g].nombre),
        Pieza::Nodo(i) => {
            let n = &red.nodos[i];
            match &n.tipo {
                Tipo::Entrada(x) => format!("in {x}"),
                Tipo::Salida(x) => format!("out {x}"),
                _ if !n.nombre.is_empty() => format!("= {}", n.nombre),
                // ponytail: las anidadas se ubican por posición; si cambia el código pueden saltar
                _ => format!("#{i}"),
            }
        }
    }
}

/// Lo que se ve al pasar el mouse
fn descripcion(red: &Netlist, pieza: Pieza) -> String {
    match pieza {
        Pieza::Caja(g) => {
            let inst = &red.instancias[g];
            format!("{}: {}", inst.nombre, inst.circuito)
        }
        Pieza::Nodo(i) => {
            let n = &red.nodos[i];
            match &n.tipo {
                Tipo::Entrada(x) => format!("Entrada {x} (click para cambiarla)"),
                Tipo::Salida(x) => format!("Salida {x}"),
                Tipo::Constante(v) => format!("Constante {}", *v as u8),
                Tipo::Compuerta(g) if n.nombre.is_empty() => g.nombre().to_uppercase(),
                Tipo::Compuerta(g) => format!("{} → {}", g.nombre().to_uppercase(), n.nombre),
            }
        }
    }
}

/// Altura del pin `k` de `n` repartidos en el lado de `r`
fn altura_pin(r: Rect, k: usize, n: usize) -> f32 {
    r.top() + r.height() * (k + 1) as f32 / (n + 1) as f32
}

fn dibujar(
    ui: &mut egui::Ui,
    rect: Rect,
    red: &Netlist,
    valores: &[bool],
    entradas: &mut [bool],
    posiciones: &mut HashMap<String, Pos2>,
) {
    let c = paleta(ui);
    let p = ui.painter_at(rect);

    // Piezas: los nodos sueltos y una caja por instancia
    let mut piezas: Vec<Pieza> = (0..red.nodos.len())
        .filter(|&i| red.nodos[i].instancia.is_none())
        .map(Pieza::Nodo)
        .collect();
    let primera_caja = piezas.len();
    piezas.extend((0..red.instancias.len()).map(Pieza::Caja));
    let total = piezas.len();
    let mut pieza_de = vec![0; red.nodos.len()];
    for (j, pieza) in piezas.iter().enumerate() {
        if let Pieza::Nodo(i) = *pieza {
            pieza_de[i] = j;
        }
    }
    for (i, n) in red.nodos.iter().enumerate() {
        if let Some(g) = n.instancia {
            pieza_de[i] = primera_caja + g;
        }
    }
    // Nodos que alimentan cada pieza, en el orden de sus pines
    let fuentes: Vec<Vec<usize>> = piezas
        .iter()
        .map(|pieza| match *pieza {
            Pieza::Nodo(i) => red.nodos[i].entradas.clone(),
            Pieza::Caja(g) => red.instancias[g].entradas.iter().map(|e| e.1).collect(),
        })
        .collect();
    let tam = |pieza: Pieza| match pieza {
        Pieza::Nodo(i) => tamano(&red.nodos[i]),
        Pieza::Caja(g) => {
            let inst = &red.instancias[g];
            let pines = inst.entradas.len().max(inst.salidas.len());
            vec2(
                (inst.circuito.len() as f32 * 7.0 + 20.0).max(96.0),
                (pines as f32 * 18.0 + 30.0).max(48.0),
            )
        }
    };

    let por_pieza: Vec<Vec<usize>> = fuentes
        .iter()
        .map(|f| f.iter().map(|&n| pieza_de[n]).collect())
        .collect();
    let salida: Vec<bool> = piezas
        .iter()
        .map(|pieza| matches!(*pieza, Pieza::Nodo(i) if matches!(red.nodos[i].tipo, Tipo::Salida(_))))
        .collect();
    let cols = columnas(&por_pieza, &salida);

    // Fila de cada pieza dentro de su columna
    let num_cols = cols.iter().max().map_or(0, |m| m + 1);
    let mut por_columna = vec![0; num_cols];
    let filas: Vec<usize> = cols
        .iter()
        .map(|&col| {
            por_columna[col] += 1;
            por_columna[col] - 1
        })
        .collect();
    let max_filas = por_columna.iter().copied().max().unwrap_or(1);

    // Centrado en el lienzo; si no cabe, empieza arriba a la izquierda
    // ponytail: sin zoom ni desplazamiento; llegan con los botones del lienzo
    let medida = vec2(
        num_cols.saturating_sub(1) as f32 * ANCHO_COLUMNA,
        max_filas.saturating_sub(1) as f32 * ALTO_FILA,
    );
    let origen = pos2(
        (rect.center().x - medida.x / 2.0).max(rect.left() + 70.0),
        (rect.center().y - medida.y / 2.0).max(rect.top() + 40.0),
    );
    let mut centros: Vec<Pos2> = (0..total)
        .map(|j| {
            let hueco = (max_filas - por_columna[cols[j]]) as f32 * ALTO_FILA / 2.0;
            origen
                + vec2(
                    cols[j] as f32 * ANCHO_COLUMNA,
                    filas[j] as f32 * ALTO_FILA + hueco,
                )
        })
        .collect();

    // Arrastrar mueve la pieza; click en una entrada la cambia; encima, su nombre
    let mut k = 0;
    for (j, &pieza) in piezas.iter().enumerate() {
        let clave = clave(red, pieza);
        if let Some(&guardada) = posiciones.get(&clave) {
            centros[j] = guardada + rect.min.to_vec2();
        }
        let r = ui.interact(
            Rect::from_center_size(centros[j], tam(pieza)),
            ui.scope_id().with(&clave),
            Sense::click_and_drag(),
        );
        if r.dragged() {
            centros[j] = (centros[j] + r.drag_delta()).clamp(rect.min, rect.max);
            posiciones.insert(clave, centros[j] - rect.min.to_vec2());
        }
        if matches!(pieza, Pieza::Nodo(i) if matches!(red.nodos[i].tipo, Tipo::Entrada(_))) {
            if r.clicked() {
                entradas[k] = !entradas[k];
            }
            k += 1;
        }
        r.on_hover_text(descripcion(red, pieza))
            .on_hover_cursor(egui::CursorIcon::Grab);
    }

    let caja = |j: usize| Rect::from_center_size(centros[j], tam(piezas[j]));
    let valor = |i: usize| valores.get(i).copied().unwrap_or(false);
    let color = |v: bool| if v { c.uno } else { c.tenue };

    // Dónde sale la señal del nodo `f`: de su pieza o del pin de la caja que lo tiene
    let pin_salida = |f: usize| {
        let j = pieza_de[f];
        let r = caja(j);
        match piezas[j] {
            Pieza::Nodo(i) => {
                let burbuja = matches!(red.nodos[i].tipo, Tipo::Compuerta(g) if invertida(g));
                pos2(r.right() + if burbuja { 8.0 } else { 0.0 }, r.center().y)
            }
            Pieza::Caja(g) => {
                let salidas = &red.instancias[g].salidas;
                let k = salidas.iter().position(|s| s.1 == f).unwrap_or(0);
                pos2(r.right(), altura_pin(r, k, salidas.len()))
            }
        }
    };
    // Dónde llega la entrada `k` de la pieza `j`
    let pin_entrada = |j: usize, k: usize| {
        let r = caja(j);
        let n = fuentes[j].len();
        let y = altura_pin(r, k, n);
        // Las OR y XOR tienen la parte de atrás curva
        let atras = match piezas[j] {
            Pieza::Nodo(i) => match red.nodos[i].tipo {
                Tipo::Compuerta(
                    Compuerta::Or | Compuerta::Nor | Compuerta::Xor | Compuerta::Xnor,
                ) => {
                    let s = (k + 1) as f32 / (n + 1) as f32;
                    0.5 * r.width() * s * (1.0 - s)
                }
                _ => 0.0,
            },
            Pieza::Caja(_) => 0.0,
        };
        pos2(r.left() + atras, y)
    };

    // Cables (debajo de los componentes), en ángulo recto
    let abajo = (0..total)
        .map(|j| caja(j).bottom())
        .fold(rect.top(), f32::max);
    let mut lazos = 0;
    for (j, fs) in fuentes.iter().enumerate() {
        for (k, &fuente) in fs.iter().enumerate() {
            let a = pin_salida(fuente);
            let b = pin_entrada(j, k);
            let puntos = if b.x > a.x + 12.0 {
                let x = (b.x - 14.0 - 5.0 * k as f32).max(a.x + 6.0);
                vec![a, pos2(x, a.y), pos2(x, b.y), b]
            } else {
                // Realimentación: sale a la derecha, pasa por debajo de todo y vuelve
                let y = abajo + 18.0 + 8.0 * lazos as f32;
                lazos += 1;
                vec![
                    a,
                    pos2(a.x + 10.0, a.y),
                    pos2(a.x + 10.0, y),
                    pos2(b.x - 10.0, y),
                    pos2(b.x - 10.0, b.y),
                    b,
                ]
            };
            p.add(Shape::line(puntos, Stroke::new(1.6, color(valor(fuente)))));
        }
    }

    // Componentes
    let trazo = Stroke::new(1.6, c.tinta);
    let texto = |pos: Pos2, alinear: Align2, t: &str| {
        p.text(pos, alinear, t, FontId::monospace(12.0), c.tinta);
    };
    for (j, &pieza) in piezas.iter().enumerate() {
        let r = caja(j);
        let i = match pieza {
            Pieza::Nodo(i) => i,
            Pieza::Caja(g) => {
                // Caja con el nombre del circuito, la instancia arriba y los pines rotulados
                let inst = &red.instancias[g];
                let pequeno = FontId::monospace(10.0);
                p.rect(r, 6.0, c.panel, trazo, StrokeKind::Inside);
                p.text(
                    r.center_top() - vec2(0.0, 4.0),
                    Align2::CENTER_BOTTOM,
                    &inst.nombre,
                    FontId::monospace(11.0),
                    c.tenue,
                );
                p.text(
                    r.center_top() + vec2(0.0, 5.0),
                    Align2::CENTER_TOP,
                    &inst.circuito,
                    FontId::proportional(11.0),
                    c.tinta,
                );
                for (k, (nombre, _)) in inst.entradas.iter().enumerate() {
                    let y = altura_pin(r, k, inst.entradas.len());
                    p.text(
                        pos2(r.left() + 5.0, y),
                        Align2::LEFT_CENTER,
                        nombre,
                        pequeno.clone(),
                        c.tenue,
                    );
                }
                for (k, (nombre, _)) in inst.salidas.iter().enumerate() {
                    let y = altura_pin(r, k, inst.salidas.len());
                    p.text(
                        pos2(r.right() - 5.0, y),
                        Align2::RIGHT_CENTER,
                        nombre,
                        pequeno.clone(),
                        c.tenue,
                    );
                }
                continue;
            }
        };
        let v = valor(i);
        match &red.nodos[i].tipo {
            Tipo::Entrada(nombre) => {
                let fondo = if v { c.uno } else { c.panel };
                p.rect(r, 5.0, fondo, trazo, StrokeKind::Inside);
                texto(r.center(), Align2::CENTER_CENTER, if v { "1" } else { "0" });
                texto(
                    r.left_center() - vec2(8.0, 0.0),
                    Align2::RIGHT_CENTER,
                    nombre,
                );
            }
            Tipo::Constante(v) => {
                p.rect(r, 3.0, c.suave, trazo, StrokeKind::Inside);
                texto(
                    r.center(),
                    Align2::CENTER_CENTER,
                    if *v { "1" } else { "0" },
                );
            }
            Tipo::Salida(nombre) => {
                let fondo = if v { c.uno } else { c.panel };
                p.circle(r.center(), r.width() / 2.0, fondo, trazo);
                texto(
                    r.right_center() + vec2(8.0, 0.0),
                    Align2::LEFT_CENTER,
                    nombre,
                );
            }
            Tipo::Compuerta(g) => {
                p.add(Shape::closed_line(contorno(*g, r), trazo));
                if matches!(g, Compuerta::Xor | Compuerta::Xnor) {
                    let (l, w) = (r.left(), r.width());
                    let extra = curva(
                        r.left_top(),
                        pos2(l + w * 0.25, r.center().y),
                        r.left_bottom(),
                    );
                    p.add(Shape::line(extra.collect(), trazo));
                }
                if invertida(*g) {
                    p.circle_stroke(r.right_center() + vec2(4.0, 0.0), 4.0, trazo);
                }
            }
        }
    }
}

/// Encabezado de 38 px con línea inferior, para el editor y el lienzo
fn cabecera(ui: &mut egui::Ui, texto: &str) {
    let c = paleta(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, c.panel);
    p.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, c.linea),
    );
    p.text(
        pos2(rect.left() + 13.0, rect.center().y),
        Align2::LEFT_CENTER,
        texto,
        FontId::proportional(12.0),
        c.tinta,
    );
}

#[derive(Clone, Copy)]
enum Icono {
    Ejecutar,
    Paso,
    Reiniciar,
    Luna,
    Sol,
}

/// Botón plano con un ícono dibujado a la izquierda del texto
fn boton(ui: &mut egui::Ui, texto: &str, icono: Icono, principal: bool) -> egui::Response {
    let c = paleta(ui);
    let (fondo, borde, color) = if principal {
        (c.salvia_fondo, c.salvia_borde, c.salvia_texto)
    } else {
        (c.panel, c.linea, c.tinta)
    };
    // Los espacios dejan lugar para el ícono
    let r = ui.add(
        egui::Button::new(
            RichText::new(format!("     {texto}"))
                .size(12.5)
                .color(color),
        )
        .fill(fondo)
        .stroke(Stroke::new(1.0, borde))
        .min_size(vec2(0.0, 28.0)),
    );
    let centro = pos2(r.rect.left() + 15.0, r.rect.center().y);
    let p = ui.painter();
    let trazo = Stroke::new(1.5, color);
    match icono {
        Icono::Ejecutar => {
            let triangulo = vec![
                centro + vec2(-4.0, -5.0),
                centro + vec2(5.0, 0.0),
                centro + vec2(-4.0, 5.0),
            ];
            p.add(Shape::convex_polygon(triangulo, color, Stroke::NONE));
        }
        Icono::Paso => {
            let triangulo = vec![
                centro + vec2(-5.0, -5.0),
                centro + vec2(2.0, 0.0),
                centro + vec2(-5.0, 5.0),
            ];
            p.add(Shape::convex_polygon(triangulo, color, Stroke::NONE));
            p.line_segment([centro + vec2(5.0, -5.0), centro + vec2(5.0, 5.0)], trazo);
        }
        Icono::Reiniciar => {
            // ↻: arco de 290° abierto abajo que termina en una punta rellena
            let radio = 5.0;
            let punto = |g: f32| {
                let a = g.to_radians();
                centro + radio * vec2(-a.cos(), a.sin())
            };
            let arco: Vec<Pos2> = (0..=24)
                .map(|i| punto(40.0 - 290.0 * i as f32 / 24.0))
                .collect();
            let fin = arco[arco.len() - 1];
            let a = (40.0_f32 - 290.0).to_radians();
            // Tangente en el sentido del recorrido y dirección hacia afuera del círculo
            let tangente = vec2(-a.sin(), -a.cos());
            let afuera = vec2(-a.cos(), a.sin());
            p.add(Shape::line(arco, trazo));
            p.add(Shape::convex_polygon(
                vec![fin + afuera * 3.0, fin + tangente * 3.5, fin - afuera * 3.0],
                color,
                Stroke::NONE,
            ));
        }
        Icono::Luna => {
            // Círculo tapado por otro del color del fondo
            p.circle_filled(centro, 5.5, color);
            p.circle_filled(centro + vec2(2.8, -2.2), 4.6, fondo);
        }
        Icono::Sol => {
            p.circle_filled(centro, 2.8, color);
            for i in 0..8 {
                let a = (i as f32 * 45.0).to_radians();
                let d = vec2(a.cos(), a.sin());
                p.line_segment([centro + d * 4.6, centro + d * 6.4], trazo);
            }
        }
    }
    r
}
