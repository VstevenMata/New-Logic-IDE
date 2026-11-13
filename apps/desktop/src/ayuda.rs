//! Contenido del menú Ayuda: guía del lenguaje, atajos, ejemplos y "Acerca de".

use eframe::egui::{self, CornerRadius, Margin, RichText, TextureHandle, vec2};

/// Código de Archivo → Nuevo
pub const PLANTILLA: &str = "circuit Nuevo {
    in A, B;
    out X;

    X = and(A, B);
}
";

/// Ayuda → Ejemplos (el primero es el que aparece al abrir la app)
pub const EJEMPLOS: [(&str, &str); 4] = [
    (
        "Compuertas básicas",
        "circuit Demo {
    in A, B, C;
    out X, Y;

    X = and(A, B);
    Y = or(X, not(C));
}
",
    ),
    (
        "Cadena con las 8 compuertas",
        "circuit Cadena {
    in A, B, C, D;
    out S1, S2, S3, Final;

    // Paso 1: leen las entradas
    p1 = and(A, B);
    p2 = or(C, D);
    p3 = not(A);

    // Paso 2: usan las salidas del paso 1
    q1 = xor(p1, p2);
    q2 = nand(p2, p3);
    q3 = buffer(p3);

    // Paso 3: usan las salidas del paso 2
    r1 = nor(q1, q2);
    r2 = xnor(q2, q3, D);

    S1 = q1;
    S2 = r1;
    S3 = r2;
    Final = and(r1, or(r2, q3));
}
",
    ),
    (
        "Medio sumador",
        "circuit MedioSumador {
    in A, B;
    out Suma, Acarreo;

    Suma    = xor(A, B);
    Acarreo = and(A, B);
}
",
    ),
    (
        "Flip-flop SR (memoria)",
        "// Enciende S y apágalo: Q se queda en 1.
// Enciende R y apágalo: Q se queda en 0.
circuit FlipFlop {
    in S, R;
    out Q, Qn;

    Q  = nor(R, Qn);
    Qn = nor(S, Q);
}
",
    ),
];

fn seccion(ui: &mut egui::Ui, texto: &str) {
    ui.add_space(12.0);
    ui.label(RichText::new(texto).size(14.0).strong());
    ui.add_space(4.0);
}

fn codigo(ui: &mut egui::Ui, texto: &str) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(texto).monospace());
        });
}

pub fn guia(ui: &mut egui::Ui) {
    ui.label(
        "En LogicSM el circuito se escribe como código. La app lo dibuja y lo simula \
         mientras escribes.",
    );

    seccion(ui, "Un circuito");
    ui.label("Se declaran las entradas (in), las salidas (out) y cómo se conectan:");
    codigo(
        ui,
        "circuit MiCircuito {\n    in A, B;\n    out X;\n\n    X = and(A, B);\n}",
    );

    seccion(ui, "Compuertas");
    egui::Grid::new("guia_compuertas")
        .striped(true)
        .spacing(vec2(24.0, 4.0))
        .show(ui, |ui| {
            for (nombre, uso) in [
                ("and, or, xor", "2 o más entradas: and(A, B, C)"),
                ("nand, nor, xnor", "2 o más entradas: nor(A, B)"),
                ("not", "1 entrada: not(A)"),
                ("buffer", "1 entrada: buffer(A)"),
            ] {
                ui.label(RichText::new(nombre).monospace().strong());
                ui.label(RichText::new(uso).monospace());
                ui.end_row();
            }
        });
    ui.label("Cada llamada es una compuerta nueva. Se pueden anidar:");
    codigo(ui, "Y = or(X, not(C));   // la salida del not entra al or");

    seccion(ui, "Señales intermedias");
    ui.label("Asignar a un nombre nuevo crea una señal que se puede usar después:");
    codigo(ui, "p = and(A, B);\nX = xor(p, C);");

    seccion(ui, "Constantes");
    codigo(ui, "X = and(A, 1);   // 0 y 1 van donde va una señal");

    seccion(ui, "El orden no importa");
    ui.label(
        "Una señal se puede usar antes de definirse. Así se arma la realimentación, \
         que es la base de la memoria:",
    );
    codigo(ui, "Q  = nor(R, Qn);\nQn = nor(S, Q);");

    seccion(ui, "Comentarios");
    codigo(ui, "// una línea\n/* varias\n   líneas */");

    seccion(ui, "Reglas");
    for regla in [
        "Distingue mayúsculas: Q y q son señales distintas.",
        "Cada señal se asigna una sola vez.",
        "Toda salida (out) debe recibir un valor.",
        "Los nombres usan letras sin tilde ni ñ, dígitos y _.",
        "Las palabras reservadas (circuit, in, out, and, led…) no se pueden usar como nombres.",
    ] {
        ui.label(format!("•  {regla}"));
    }

    seccion(ui, "Usar la app");
    for paso in [
        "Haz click en una entrada (en el dibujo o a la derecha) para cambiarla entre 0 y 1.",
        "Cables y salidas en verde valen 1; en gris valen 0.",
        "Los errores aparecen en la pestaña Problemas con su número de línea.",
        "La pestaña Tabla de verdad muestra todas las combinaciones de entradas.",
    ] {
        ui.label(format!("•  {paso}"));
    }

    seccion(ui, "Todavía no disponible");
    ui.label(
        "Buses (A[8]), for, circuitos propios, button, clock, led, display7, matrix, \
         probe, tristate, rom y ram. Llegan en las próximas versiones.",
    );
}

pub fn atajos(ui: &mut egui::Ui) {
    egui::Grid::new("atajos")
        .striped(true)
        .spacing(vec2(32.0, 6.0))
        .show(ui, |ui| {
            for (accion, atajo) in [
                ("Nuevo", "Ctrl+N"),
                ("Abrir", "Ctrl+O"),
                ("Guardar", "Ctrl+S"),
                ("Guardar como", "Ctrl+Shift+S"),
                ("Salir", "Ctrl+Q"),
                ("Deshacer", "Ctrl+Z"),
                ("Rehacer", "Ctrl+Y"),
                ("Cortar / Copiar / Pegar", "Ctrl+X / Ctrl+C / Ctrl+V"),
                ("Seleccionar todo", "Ctrl+A"),
                ("Acercar / Alejar", "Ctrl++ / Ctrl+-"),
                ("Tamaño normal", "Ctrl+0"),
                ("Panel de proyecto", "Ctrl+B"),
                ("Panel inferior", "Ctrl+J"),
                ("Reiniciar simulación", "Ctrl+Shift+F5"),
                ("Guía del lenguaje", "F1"),
            ] {
                ui.label(accion);
                ui.label(RichText::new(atajo).monospace());
                ui.end_row();
            }
        });
}

pub fn actualizar(ui: &mut egui::Ui) {
    ui.label(concat!("Versión instalada: ", env!("CARGO_PKG_VERSION")));
    ui.add_space(8.0);
    ui.label("Para actualizar, cierra LogicSM y ejecuta en una terminal:");
    codigo(ui, "logicsm update");
    ui.add_space(4.0);
    ui.hyperlink_to(
        "Ver las versiones publicadas",
        concat!(env!("CARGO_PKG_REPOSITORY"), "/releases"),
    );
}

pub fn acerca(ui: &mut egui::Ui, logo: &TextureHandle) {
    ui.vertical_centered(|ui| {
        ui.image((logo.id(), vec2(96.0, 96.0)));
        ui.add_space(6.0);
        ui.label(RichText::new("LogicSM").size(18.0).strong());
        ui.label(concat!("Versión ", env!("CARGO_PKG_VERSION")));
        ui.add_space(6.0);
        ui.label("Entorno para aprender electrónica digital escribiendo circuitos.");
        ui.label("Licencia MIT o Apache-2.0.");
        ui.add_space(4.0);
        ui.hyperlink_to("Código fuente", env!("CARGO_PKG_REPOSITORY"));
    });
}
