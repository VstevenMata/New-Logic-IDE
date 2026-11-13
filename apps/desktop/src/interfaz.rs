//! Interfaz de LogicSM, por ahora solo visual: barra superior, proyecto, código, lienzo,
//! entradas y salidas, y pestañas inferiores. Diseño y paleta tomados de index.html.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Shape,
    Stroke, Theme, pos2, vec2,
};

/// Colores de un tema. Hay dos: CLARO (pastel de index.html) y OSCURO.
struct Paleta {
    fondo: Color32,
    panel: Color32,
    tinta: Color32,
    tenue: Color32,
    numero_linea: Color32,
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
}

const CLARO: Paleta = Paleta {
    fondo: Color32::from_rgb(0xf8, 0xf8, 0xf5),
    panel: Color32::WHITE,
    tinta: Color32::from_rgb(0x29, 0x2b, 0x2a),
    tenue: Color32::from_rgb(0x7f, 0x82, 0x7e),
    numero_linea: Color32::from_rgb(0xa6, 0xaa, 0xa5),
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
};

// Gris verdoso (no negro puro) para que el pastel siga combinando
const OSCURO: Paleta = Paleta {
    fondo: Color32::from_rgb(0x1d, 0x20, 0x1e),
    panel: Color32::from_rgb(0x25, 0x28, 0x26),
    tinta: Color32::from_rgb(0xe3, 0xe5, 0xe0),
    tenue: Color32::from_rgb(0x92, 0x97, 0x90),
    numero_linea: Color32::from_rgb(0x5f, 0x64, 0x5e),
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
            for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
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

pub struct Interfaz;

impl eframe::App for Interfaz {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let c = paleta(ui);
        egui::Panel::top("barra")
            .exact_size(48.0)
            .resizable(false)
            .frame(Frame::new().fill(c.panel).inner_margin(Margin::symmetric(14, 0)))
            .show(ui, barra_superior);
        egui::Panel::left("proyecto")
            .exact_size(174.0)
            .resizable(false)
            .frame(Frame::new().fill(c.panel).inner_margin(Margin::symmetric(14, 15)))
            .show(ui, proyecto);
        egui::Panel::right("senales")
            .exact_size(222.0)
            .resizable(false)
            .frame(Frame::new().fill(c.panel).inner_margin(Margin::symmetric(14, 15)))
            .show(ui, senales);
        egui::Panel::bottom("pestanas")
            .exact_size(166.0)
            .resizable(false)
            .frame(Frame::new().fill(c.panel))
            .show(ui, pestanas);
        egui::Panel::left("codigo")
            .default_size(420.0)
            .resizable(true)
            .frame(Frame::new().fill(c.codigo))
            .show(ui, editor);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(c.fondo))
            .show(ui, lienzo);
    }
}

/// Nombre y menú a la izquierda, controles de simulación junto a ellos y el tema a la derecha
fn barra_superior(ui: &mut egui::Ui) {
    ui.horizontal_centered(|ui| {
        ui.label(RichText::new("LogicSM").size(14.0).strong());
        ui.add_space(6.0);
        menu(ui);
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);
        boton(ui, "Ejecutar", Icono::Ejecutar, true);
        boton(ui, "Paso a paso", Icono::Paso, false);
        boton(ui, "Reiniciar", Icono::Reiniciar, false);

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

/// Formatos de exportación (menú Archivo y botón Exportar de Proyecto)
const FORMATOS: [&str; 3] = ["Imagen (PNG)", "Tabla de verdad (CSV)", "Verilog (.v)"];

/// Menú principal al estilo de VS Code. Por ahora solo visual: las opciones no hacen nada.
fn menu(ui: &mut egui::Ui) {
    let opcion = |ui: &mut egui::Ui, texto: &str, atajo: &str| {
        ui.add(egui::Button::new(texto).shortcut_text(atajo));
    };
    let deshabilitada = |ui: &mut egui::Ui, texto: &str| {
        ui.add_enabled(false, egui::Button::new(texto));
    };

    ui.menu_button("Archivo", |ui| {
        opcion(ui, "Nuevo proyecto", "Ctrl+N");
        opcion(ui, "Abrir…", "Ctrl+O");
        ui.menu_button("Abrir reciente", |ui| deshabilitada(ui, "Sin proyectos recientes"));
        ui.separator();
        opcion(ui, "Guardar", "Ctrl+S");
        opcion(ui, "Guardar como…", "Ctrl+Shift+S");
        ui.menu_button("Exportar", |ui| {
            for formato in FORMATOS {
                deshabilitada(ui, formato);
            }
        });
        ui.separator();
        opcion(ui, "Salir", "Ctrl+Q");
    });
    ui.menu_button("Editar", |ui| {
        opcion(ui, "Deshacer", "Ctrl+Z");
        opcion(ui, "Rehacer", "Ctrl+Y");
        ui.separator();
        opcion(ui, "Cortar", "Ctrl+X");
        opcion(ui, "Copiar", "Ctrl+C");
        opcion(ui, "Pegar", "Ctrl+V");
        ui.separator();
        opcion(ui, "Buscar", "Ctrl+F");
        opcion(ui, "Reemplazar", "Ctrl+H");
    });
    ui.menu_button("Ver", |ui| {
        opcion(ui, "Acercar", "Ctrl++");
        opcion(ui, "Alejar", "Ctrl+-");
        opcion(ui, "Ajustar el circuito", "Ctrl+0");
        ui.separator();
        opcion(ui, "Panel de proyecto", "Ctrl+B");
        opcion(ui, "Entradas y salidas", "");
        opcion(ui, "Panel inferior", "Ctrl+J");
    });
    ui.menu_button("Simulación", |ui| {
        opcion(ui, "Ejecutar", "F5");
        opcion(ui, "Paso a paso", "F10");
        opcion(ui, "Reiniciar", "Ctrl+Shift+F5");
        ui.separator();
        opcion(ui, "Tabla de verdad", "");
        opcion(ui, "Ondas", "");
    });
    ui.menu_button("Ayuda", |ui| {
        opcion(ui, "Guía del lenguaje", "F1");
        opcion(ui, "Ejemplos", "");
        opcion(ui, "Atajos de teclado", "");
        ui.separator();
        opcion(ui, "Buscar actualizaciones", "");
        opcion(ui, "Reportar un problema", "");
        ui.separator();
        // La versión instalada, para confirmar que logicsm update funcionó
        deshabilitada(ui, concat!("LogicSM ", env!("CARGO_PKG_VERSION")));
    });
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

/// Acciones del proyecto. Por ahora no hacen nada: llegan con el manejo de archivos.
fn proyecto(ui: &mut egui::Ui) {
    titulo(ui, "Proyecto");
    let ancho = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
    let accion = |ui: &mut egui::Ui, texto: &str| {
        ui.add(egui::Button::new(RichText::new(texto).size(12.0)).min_size(vec2(ancho, 28.0)))
    };
    ui.horizontal(|ui| {
        accion(ui, "Nuevo");
        accion(ui, "Abrir");
    });
    ui.horizontal(|ui| {
        accion(ui, "Guardar");
        let exportar = accion(ui, "Exportar ▾");
        egui::Popup::menu(&exportar).show(|ui| {
            for formato in FORMATOS {
                ui.add_enabled(false, egui::Button::new(formato));
            }
        });
    });
    ui.add_space(14.0);
    vacio(ui, "Abre una carpeta o crea un proyecto nuevo.");
}

fn senales(ui: &mut egui::Ui) {
    titulo(ui, "Entradas");
    vacio(ui, "Aparecen al ejecutar el código.");
    ui.add_space(24.0);
    titulo(ui, "Salidas");
    vacio(ui, "Aparecen al ejecutar el código.");
}

/// Encabezado de 38 px con línea inferior, para el editor y el lienzo
fn cabecera(ui: &mut egui::Ui, texto: &str) {
    let c = paleta(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, c.panel);
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, c.linea));
    p.text(
        pos2(rect.left() + 13.0, rect.center().y),
        Align2::LEFT_CENTER,
        texto,
        FontId::proportional(12.0),
        c.tinta,
    );
}

fn editor(ui: &mut egui::Ui) {
    cabecera(ui, "Código");
    let arriba = ui.cursor().top();
    ui.painter().text(
        pos2(ui.max_rect().left() + 40.0, arriba + 24.0),
        Align2::RIGHT_CENTER,
        "1",
        FontId::monospace(12.0),
        paleta(ui).numero_linea,
    );
}

fn lienzo(ui: &mut egui::Ui) {
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
    // Aquí van a ir las compuertas y los demás componentes del circuito
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "ESTÁ EN MODO CARPINTERÍA",
        FontId::monospace(13.0),
        c.tenue,
    );

    // Controles del lienzo, abajo a la derecha
    let mut x = rect.right() - 12.0;
    let y = rect.bottom() - 39.0;
    for (texto, ancho) in [("Reordenar", 82.0), ("Ajustar", 64.0), ("−", 29.0), ("+", 29.0)] {
        x -= ancho;
        ui.put(
            Rect::from_min_size(pos2(x, y), vec2(ancho, 27.0)),
            egui::Button::new(RichText::new(texto).size(11.0).color(c.tenue)),
        );
        x -= 4.0;
    }
}

fn pestanas(ui: &mut egui::Ui) {
    let c = paleta(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 37.0), Sense::hover());
    let p = ui.painter();
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, c.linea));
    let mut x = rect.left() + 15.0;
    for (i, nombre) in ["Problemas", "Tabla de verdad", "Ondas", "Salida"]
        .into_iter()
        .enumerate()
    {
        let color = if i == 0 { c.tinta } else { c.tenue };
        let r = p.text(
            pos2(x, rect.center().y),
            Align2::LEFT_CENTER,
            nombre,
            FontId::proportional(12.0),
            color,
        );
        if i == 0 {
            let y = rect.bottom() - 1.0;
            p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], Stroke::new(2.0, c.lavanda));
        }
        x = r.right() + 21.0;
    }
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(15.0);
        vacio(ui, "Sin problemas.");
    });
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
        egui::Button::new(RichText::new(format!("     {texto}")).size(12.5).color(color))
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
            let arco: Vec<Pos2> = (0..=24).map(|i| punto(40.0 - 290.0 * i as f32 / 24.0)).collect();
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
