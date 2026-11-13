//! Contenido del menú Ayuda: guía del lenguaje, atajos, ejemplos y "Acerca de".

use eframe::egui::{self, CornerRadius, Margin, RichText, TextureHandle, vec2};

// ---------- Ejemplos: circuitos clásicos ----------
// Cada archivo es (nombre, código). Un ejemplo agrega sus archivos al proyecto,
// incluidos los circuitos que usa, y abre el último.

const MEDIO_SUMADOR: (&str, &str) = (
    "MedioSumador.SA",
    "// Medio sumador: suma dos bits.
circuit MedioSumador {
    in A, B;
    out Suma, Acarreo;

    Suma    = xor(A, B);
    Acarreo = and(A, B);
}
",
);

const SUMADOR_COMPLETO: (&str, &str) = (
    "SumadorCompleto.SA",
    "// Sumador completo: suma dos bits y el acarreo que llega.
// Usa dos MedioSumador (están en MedioSumador.SA).
circuit SumadorCompleto {
    in A, B, Cin;
    out S, Cout;

    m1 = MedioSumador(A, B);
    m2 = MedioSumador(m1.Suma, Cin);

    S    = m2.Suma;
    Cout = or(m1.Acarreo, m2.Acarreo);
}
",
);

const SUMADOR_2_BITS: (&str, &str) = (
    "Sumador2.SA",
    "// Sumador de 2 bits: A1 A0 + B1 B0.
// Encadena dos SumadorCompleto: el acarreo del bit 0 entra al bit 1.
circuit Sumador2 {
    in A1, A0, B1, B0;
    out S1, S0, Cout;

    b0 = SumadorCompleto(A0, B0, 0);
    b1 = SumadorCompleto(A1, B1, b0.Cout);

    S0   = b0.S;
    S1   = b1.S;
    Cout = b1.Cout;
}
",
);

const MEDIO_RESTADOR: (&str, &str) = (
    "MedioRestador.SA",
    "// Medio restador: A - B.
circuit MedioRestador {
    in A, B;
    out Resta, Prestamo;

    Resta    = xor(A, B);
    Prestamo = and(not(A), B);
}
",
);

const MUX2: (&str, &str) = (
    "Mux2.SA",
    "// Multiplexor 2 a 1: con S = 0 sale A, con S = 1 sale B.
circuit Mux2 {
    in A, B, S;
    out Y;

    Y = or(and(A, not(S)), and(B, S));
}
",
);

const MUX4: (&str, &str) = (
    "Mux4.SA",
    "// Multiplexor 4 a 1: S1 S0 elige cuál de D0..D3 sale.
// Hecho con tres Mux2 (están en Mux2.SA).
circuit Mux4 {
    in D0, D1, D2, D3, S1, S0;
    out Y;

    bajo = Mux2(D0, D1, S0);
    alto = Mux2(D2, D3, S0);
    fin  = Mux2(bajo.Y, alto.Y, S1);

    Y = fin.Y;
}
",
);

const DECO_2A4: (&str, &str) = (
    "Deco2a4.SA",
    "// Decodificador 2 a 4: enciende solo la salida cuyo número forman A1 A0.
circuit Deco2a4 {
    in A1, A0;
    out Y0, Y1, Y2, Y3;

    Y0 = and(not(A1), not(A0));
    Y1 = and(not(A1), A0);
    Y2 = and(A1, not(A0));
    Y3 = and(A1, A0);
}
",
);

const CODIFICADOR_4A2: (&str, &str) = (
    "Codificador4a2.SA",
    "// Codificador con prioridad 4 a 2: da el número de la entrada más alta encendida.
// V vale 1 si hay alguna entrada encendida.
circuit Codificador4a2 {
    in D3, D2, D1, D0;
    out A1, A0, V;

    A1 = or(D3, D2);
    A0 = or(D3, and(D1, not(D2)));
    V  = or(D3, D2, D1, D0);
}
",
);

const COMPARADOR_1: (&str, &str) = (
    "Comparador1.SA",
    "// Comparador de 1 bit.
circuit Comparador1 {
    in A, B;
    out Mayor, Igual, Menor;

    Mayor = and(A, not(B));
    Menor = and(not(A), B);
    Igual = xnor(A, B);
}
",
);

const COMPARADOR_2: (&str, &str) = (
    "Comparador2.SA",
    "// Comparador de 2 bits: A1 A0 contra B1 B0.
// Decide el bit alto; si son iguales, decide el bajo. Usa Comparador1.SA.
circuit Comparador2 {
    in A1, A0, B1, B0;
    out Mayor, Igual, Menor;

    alto = Comparador1(A1, B1);
    bajo = Comparador1(A0, B0);

    Mayor = or(alto.Mayor, and(alto.Igual, bajo.Mayor));
    Menor = or(alto.Menor, and(alto.Igual, bajo.Menor));
    Igual = and(alto.Igual, bajo.Igual);
}
",
);

const PARIDAD: (&str, &str) = (
    "Paridad4.SA",
    "// Generador de paridad par: P hace que el total de unos (D3..D0 y P) sea par.
circuit Paridad4 {
    in D3, D2, D1, D0;
    out P;

    P = xor(D3, D2, D1, D0);
}
",
);

const LATCH_SR: (&str, &str) = (
    "LatchSR.SA",
    "// Latch SR con NOR: la realimentación guarda un bit.
// Enciende S y apágalo: Q se queda en 1. Enciende R y apágalo: Q vuelve a 0.
circuit LatchSR {
    in S, R;
    out Q, Qn;

    Q  = nor(R, Qn);
    Qn = nor(S, Q);
}
",
);

const LATCH_D: (&str, &str) = (
    "LatchD.SA",
    "// Latch D: mientras E = 1, Q copia a D; con E = 0 recuerda el último valor.
// Usa LatchSR.SA.
circuit LatchD {
    in D, E;
    out Q, Qn;

    sr = LatchSR(and(D, E), and(not(D), E));

    Q  = sr.Q;
    Qn = sr.Qn;
}
",
);

/// Ayuda → Ejemplos y el botón "Agregar ejemplo" de Proyecto
pub const EJEMPLOS: &[(&str, &[(&str, &str)])] = &[
    ("Medio sumador", &[MEDIO_SUMADOR]),
    ("Sumador completo", &[MEDIO_SUMADOR, SUMADOR_COMPLETO]),
    (
        "Sumador de 2 bits",
        &[MEDIO_SUMADOR, SUMADOR_COMPLETO, SUMADOR_2_BITS],
    ),
    ("Medio restador", &[MEDIO_RESTADOR]),
    ("Multiplexor 2 a 1", &[MUX2]),
    ("Multiplexor 4 a 1", &[MUX2, MUX4]),
    ("Decodificador 2 a 4", &[DECO_2A4]),
    ("Codificador con prioridad 4 a 2", &[CODIFICADOR_4A2]),
    ("Comparador de 1 bit", &[COMPARADOR_1]),
    ("Comparador de 2 bits", &[COMPARADOR_1, COMPARADOR_2]),
    ("Generador de paridad", &[PARIDAD]),
    ("Latch SR (memoria)", &[LATCH_SR]),
    ("Latch D", &[LATCH_SR, LATCH_D]),
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

    seccion(ui, "Circuitos propios");
    ui.label(
        "Un circuito se puede usar dentro de otro, como si fuera una compuerta. \
         Cada uso es una copia nueva con sus propias compuertas. Puede estar en el \
         mismo archivo o en cualquier otro archivo del proyecto, sin import:",
    );
    codigo(
        ui,
        "m1 = MedioSumador(A, B);          // las entradas van en el orden de su `in`\n\
         m2 = MedioSumador(m1.Suma, Cin);  // sus salidas se usan con .nombre\n\
         S  = m2.Suma;",
    );
    ui.label(
        "Se simula el archivo abierto: su `main`, o si no tiene, el último circuito \
         del archivo que ningún otro usa.",
    );

    seccion(ui, "Proyecto");
    for paso in [
        "Un proyecto es una carpeta con archivos .SA.",
        "El botón + del panel Proyecto crea un archivo nuevo con un circuito del mismo nombre.",
        "Click derecho sobre un archivo para renombrarlo o eliminarlo.",
        "Ayuda → Ejemplos (o \"Agregar ejemplo\") suma circuitos clásicos al proyecto.",
        "Guardar pide la carpeta la primera vez; después guarda ahí todos los archivos.",
    ] {
        ui.label(format!("•  {paso}"));
    }

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
        "Arrastra compuertas, entradas y cajas para acomodarlas; Reordenar las devuelve.",
        "Pasa el mouse sobre una pieza para ver su nombre. Cada circuito propio se ve como una caja.",
        "Los errores aparecen en la pestaña Problemas con su número de línea.",
        "La pestaña Tabla de verdad muestra todas las combinaciones de entradas.",
    ] {
        ui.label(format!("•  {paso}"));
    }

    seccion(ui, "Todavía no disponible");
    ui.label(
        "Buses (A[8]), for, button, clock, led, display7, matrix, probe, tristate, \
         rom y ram. Llegan en las próximas versiones.",
    );
}

pub fn atajos(ui: &mut egui::Ui) {
    egui::Grid::new("atajos")
        .striped(true)
        .spacing(vec2(32.0, 6.0))
        .show(ui, |ui| {
            for (accion, atajo) in [
                ("Nuevo proyecto", "Ctrl+N"),
                ("Abrir carpeta", "Ctrl+O"),
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
