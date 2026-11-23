//! Colores de los dos temas, estilo de egui y botones dibujados a mano.

use super::*;

/// Colores de un tema. Hay dos: CLARO (pastel de index.html) y OSCURO.
pub(super) struct Paleta {
    pub(super) fondo: Color32,
    pub(super) panel: Color32,
    pub(super) tinta: Color32,
    pub(super) tenue: Color32,
    pub(super) linea: Color32,
    pub(super) suave: Color32,
    pub(super) lavanda: Color32,
    pub(super) lavanda_fondo: Color32,
    pub(super) salvia_fondo: Color32,
    pub(super) salvia_borde: Color32,
    pub(super) salvia_texto: Color32,
    pub(super) codigo: Color32,
    /// Puntos de la cuadrícula del lienzo (premultiplicado)
    pub(super) puntos: Color32,
    /// Cables y LEDs en 1 (en 0 usan `tenue`)
    pub(super) uno: Color32,
    /// Colores del código: estructura (`circuit`, `in`…), componentes (`and`, `split`…),
    /// números y textos, comentarios y lo que el lexer no entiende
    pub(super) clave: Color32,
    pub(super) componente: Color32,
    pub(super) numero: Color32,
    pub(super) comentario: Color32,
    pub(super) error: Color32,
}

pub(super) const CLARO: Paleta = Paleta {
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
    // Colores de lirios: lila, rosa, lirio tigre (naranja) y tallo (verde)
    clave: Color32::from_rgb(0x86, 0x58, 0xc0),
    componente: Color32::from_rgb(0xc2, 0x4f, 0x80),
    numero: Color32::from_rgb(0xc8, 0x6e, 0x1e),
    comentario: Color32::from_rgb(0x7d, 0x98, 0x74),
    error: Color32::from_rgb(0xc4, 0x3d, 0x33),
};

// Gris verdoso (no negro puro) para que el pastel siga combinando
pub(super) const OSCURO: Paleta = Paleta {
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
    clave: Color32::from_rgb(0xc9, 0xa8, 0xf0),
    componente: Color32::from_rgb(0xf0, 0xa0, 0xc4),
    numero: Color32::from_rgb(0xf2, 0xb4, 0x7a),
    comentario: Color32::from_rgb(0x84, 0x9c, 0x7e),
    error: Color32::from_rgb(0xf0, 0x7c, 0x70),
};

impl Paleta {
    /// Color de un valor: verde en 1, gris en 0, rojo en X y lavanda en Z
    pub(super) fn de(&self, v: Valor) -> Color32 {
        match v {
            Valor::Uno => self.uno,
            Valor::Cero => self.tenue,
            Valor::X => self.error,
            Valor::Z => self.lavanda,
        }
    }
}

/// La paleta del tema activo
pub(super) fn paleta(ui: &egui::Ui) -> &'static Paleta {
    match ui.ctx().theme() {
        Theme::Light => &CLARO,
        Theme::Dark => &OSCURO,
    }
}

/// Colores y bordes de los widgets de egui para los dos temas.
/// La primera vez arranca en claro; después, con el último que se eligió.
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

/// Triángulo ▾ a la derecha de un botón que abre un menú. Se dibuja en vez de escribirlo:
/// en egui los símbolos dependen de las fuentes del sistema y en Linux salía un cuadro vacío.
pub(super) fn flecha_abajo(ui: &egui::Ui, boton: &egui::Response) {
    let c = boton.rect.right_center() - vec2(12.0, 0.0);
    let color = ui.visuals().widgets.style(boton).fg_stroke.color;
    ui.painter().add(Shape::convex_polygon(
        vec![
            c + vec2(-4.0, -2.0),
            c + vec2(4.0, -2.0),
            c + vec2(0.0, 3.0),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

/// Línea ondulada de `x0` a `x1` a la altura `y`, como la de los correctores
pub(super) fn zigzag(p: &egui::Painter, x0: f32, x1: f32, y: f32, color: Color32) {
    let paso = 2.0;
    let puntos = (0..=((x1 - x0) / paso).ceil() as usize)
        .map(|i| {
            let x = (x0 + i as f32 * paso).min(x1);
            pos2(x, if i % 2 == 0 { y } else { y - 2.0 })
        })
        .collect();
    p.add(Shape::line(puntos, Stroke::new(1.0, color)));
}

/// El código con un color por tipo de token, para el editor
pub(super) fn colorear(codigo: &str, fuente: &FontId, c: &Paleta) -> egui::text::LayoutJob {
    use logicsm_core::lexer::{Reservada as R, Tipo as T, resaltar};
    let (tokens, errores) = resaltar(codigo);
    let mut tramos: Vec<(std::ops::Range<usize>, Color32)> = tokens
        .iter()
        .map(|t| {
            let color = match &t.tipo {
                T::Reservada(R::Circuit | R::Main | R::For | R::In | R::Out) => c.clave,
                T::Reservada(_) => c.componente,
                T::Entero(_) | T::Texto(_) => c.numero,
                T::Comentario => c.comentario,
                T::Ident(_) => c.tinta,
                _ => c.tenue,
            };
            (t.rango.clone(), color)
        })
        .chain(errores.iter().map(|e| (e.rango.clone(), c.error)))
        .collect();
    // Estable: un comentario sin cerrar es token y error a la vez, y gana el token
    tramos.sort_by_key(|(r, _)| r.start);

    let mut trabajo = egui::text::LayoutJob::default();
    let mut agregar = |desde: usize, hasta: usize, color| {
        let formato = egui::TextFormat::simple(fuente.clone(), color);
        trabajo.append(&codigo[desde..hasta], 0.0, formato);
    };
    let mut fin = 0;
    for (r, color) in tramos {
        if r.start < fin {
            continue;
        }
        // Espacios y saltos de línea entre tokens
        agregar(fin, r.start, c.tinta);
        agregar(r.start, r.end, color);
        fin = r.end;
    }
    agregar(fin, codigo.len(), c.tinta);
    trabajo
}

/// Título de sección ("Proyecto", "Entradas"…)
pub(super) fn titulo(ui: &mut egui::Ui, texto: &str) {
    ui.label(RichText::new(texto).size(12.0).strong());
    ui.add_space(8.0);
}

/// Texto tenue para secciones vacías
pub(super) fn vacio(ui: &mut egui::Ui, texto: &str) {
    ui.label(RichText::new(texto).size(11.5).color(paleta(ui).tenue));
}

/// "0,5 Hz", "10 Hz"
pub(super) fn hz(f: f64) -> String {
    format!("{f} Hz").replace('.', ",")
}

/// Encabezado de 38 px con línea inferior, para el editor y el lienzo
pub(super) fn cabecera(ui: &mut egui::Ui, texto: &str) {
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

pub(super) enum Lado {
    Izquierda,
    Abajo,
    Derecha,
}

/// Muestra u oculta un panel. Dibuja la ventana con ese panel marcado, como en VS Code:
/// relleno si se ve, solo el borde si está oculto.
pub(super) fn boton_panel(ui: &mut egui::Ui, visible: bool, lado: Lado, ayuda: &str) -> bool {
    let c = paleta(ui);
    let (rect, r) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    let p = ui.painter();
    if r.hovered() {
        p.rect_filled(rect, 5.0, c.suave);
    }
    let color = if visible { c.tinta } else { c.tenue };
    let trazo = Stroke::new(1.2, color);
    let marco = Rect::from_center_size(rect.center(), vec2(16.0, 12.0));
    let parte = match lado {
        Lado::Izquierda => Rect::from_min_max(marco.min, pos2(marco.left() + 5.0, marco.bottom())),
        Lado::Abajo => Rect::from_min_max(pos2(marco.left(), marco.bottom() - 4.0), marco.max),
        Lado::Derecha => Rect::from_min_max(pos2(marco.right() - 5.0, marco.top()), marco.max),
    };
    if visible {
        p.rect_filled(parte, 0.0, color);
    } else {
        p.rect_stroke(parte, 0.0, trazo, StrokeKind::Inside);
    }
    p.rect_stroke(marco, 2.0, trazo, StrokeKind::Inside);
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(ayuda)
        .clicked()
}

#[derive(Clone, Copy)]
pub(super) enum Icono {
    Ejecutar,
    Pausa,
    Paso,
    Reiniciar,
    Luna,
    Sol,
}

/// Botón plano con un ícono dibujado a la izquierda del texto
pub(super) fn boton(
    ui: &mut egui::Ui,
    texto: &str,
    icono: Icono,
    principal: bool,
) -> egui::Response {
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
        Icono::Pausa => {
            for x in [-3.0, 3.0] {
                let barra = Rect::from_center_size(centro + vec2(x, 0.0), vec2(3.0, 10.0));
                p.rect_filled(barra, 1.0, color);
            }
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
