//! Interfaz de LogicSM: barra superior, proyecto, código, lienzo, entradas y salidas, y
//! pestañas inferiores. Diseño y paleta tomados de index.html.
//!
//! El código se compila en cada cambio y el lienzo dibuja el circuito al instante.

use crate::ayuda;
use crate::plataforma::{self, Leido, Llegada, Subido, WEB};
use crate::proyecto::{Posiciones, Problema, Proyecto, es_sa};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Shape,
    Stroke, StrokeKind, Theme, Vec2, pos2, vec2,
};
use logicsm_core::netlist::{Clase, Netlist, Nodo, Split, Tipo, Visor};
use logicsm_core::solucion::{Producto, escribir, minimizar};
use logicsm_core::{Compuerta, Valor};
use logicsm_sim::{Estado, Tabla};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

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
    /// Colores del código: estructura (`circuit`, `in`…), componentes (`and`, `split`…),
    /// números y textos, comentarios y lo que el lexer no entiende
    clave: Color32,
    componente: Color32,
    numero: Color32,
    comentario: Color32,
    error: Color32,
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
    // Colores de lirios: lila, rosa, lirio tigre (naranja) y tallo (verde)
    clave: Color32::from_rgb(0x86, 0x58, 0xc0),
    componente: Color32::from_rgb(0xc2, 0x4f, 0x80),
    numero: Color32::from_rgb(0xc8, 0x6e, 0x1e),
    comentario: Color32::from_rgb(0x7d, 0x98, 0x74),
    error: Color32::from_rgb(0xc4, 0x3d, 0x33),
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
    clave: Color32::from_rgb(0xc9, 0xa8, 0xf0),
    componente: Color32::from_rgb(0xf0, 0xa0, 0xc4),
    numero: Color32::from_rgb(0xf2, 0xb4, 0x7a),
    comentario: Color32::from_rgb(0x84, 0x9c, 0x7e),
    error: Color32::from_rgb(0xf0, 0x7c, 0x70),
};

impl Paleta {
    /// Color de un valor: verde en 1, gris en 0, rojo en X y lavanda en Z
    fn de(&self, v: Valor) -> Color32 {
        match v {
            Valor::Uno => self.uno,
            Valor::Cero => self.tenue,
            Valor::X => self.error,
            Valor::Z => self.lavanda,
        }
    }
}

/// La paleta del tema activo
fn paleta(ui: &egui::Ui) -> &'static Paleta {
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

/// Lo que se hace después de confirmar que se pueden perder los cambios sin guardar
#[derive(Clone)]
enum Accion {
    /// Pide nombre y lugar del proyecto y lo crea
    Nuevo,
    /// Pide un archivo `.SA`
    Abrir,
    /// Un `.SA`: recientes y arrastrar
    AbrirRuta(PathBuf),
    /// Vuelve a la pantalla de inicio
    Cerrar,
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

/// Proyectos recientes que se recuerdan
const RECIENTES: usize = 8;
/// Movimientos del lienzo que se pueden deshacer
const DESHACER: usize = 100;

/// Aviso para Ejecutar y Paso a paso cuando el circuito no tiene `clock`
const SIN_RELOJ: &str = "Agrega un clock al circuito para usar el reloj.";
/// Frecuencias del reloj del sistema que se pueden elegir, en Hz
const FRECUENCIAS: [f64; 8] = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0];
/// Medios ciclos que se simulan como mucho en un cuadro; si no alcanza, el reloj va más lento
const MAX_TICS: usize = 1000;
/// Muestras que guarda y dibuja cada sonda
const MUESTRAS: usize = 48;

pub struct Interfaz {
    proyecto: Proyecto,
    /// Pantalla de inicio: no hay nada abierto (`proyecto` queda de relleno)
    sin_proyecto: bool,
    /// Carpetas abiertas o guardadas hace poco, la más reciente primero
    recientes: Vec<PathBuf>,
    /// Último circuito que compiló; se queda mientras el código tiene errores.
    red: Option<Netlist>,
    problemas: Vec<Problema>,
    /// Advertencias del circuito que compiló (entradas sin usar)
    avisos: Vec<String>,
    /// Columna "esperado" de la tabla de verdad de cada archivo: [fila][salida], `None` = da igual
    esperado: HashMap<String, Vec<Vec<Option<bool>>>>,
    /// Archivo con el que se compara el abierto en la tabla de verdad
    comparar: Option<String>,
    /// Matrices de LEDs con cuadrados en vez de círculos (clave de la pieza), por archivo
    // ponytail: solo mientras la app está abierta; guardarlo en el .SA si se pide
    cuadradas: HashMap<String, HashSet<String>>,
    /// Valor de cada `in`, en orden
    entradas: Vec<bool>,
    /// Valor de cada nodo de `red`, memoria de los flip-flops y de las RAM
    estado: Estado,
    estable: bool,
    /// El reloj del sistema está latiendo (Ejecutar)
    corriendo: bool,
    /// Frecuencia del reloj en Hz
    frecuencia: f64,
    /// Valor actual de todos los `clock`
    reloj: bool,
    /// Momento (de `egui`) del último medio ciclo
    ultimo_tic: f64,
    /// Paso a paso movió el reloj: la sonda guarda una muestra aunque nada cambie
    tic: bool,
    /// Valores de todos los nodos en las últimas muestras, para dibujar las sondas
    ondas: VecDeque<Vec<Valor>>,
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
    /// Ícono de los proyectos `.SA` (Recientes)
    icono_sa: egui::TextureHandle,
    /// Parte del lienzo que se ve (zoom y desplazamiento), en coordenadas del circuito
    vista: Rect,
    /// Encuadrar el circuito en el próximo cuadro
    ajustar: bool,
    /// Posiciones del archivo abierto antes de cada movimiento, para Ctrl+Z / Ctrl+Y
    deshacer: Vec<Posiciones>,
    rehacer: Vec<Posiciones>,
    /// Lo último que se hizo fue mover piezas: Editar → Deshacer va al lienzo
    ultimo_lienzo: bool,
    /// Línea a la que saltar en el editor (click en Problemas): (carácter, línea)
    ir_a: Option<(usize, usize)>,
    /// Donde está el lienzo en la ventana: lo que se recorta al exportar la imagen
    rect_lienzo: Rect,
    /// Ventana para elegir qué archivos exportar: formato y archivos marcados
    elegir: Option<(Formato, Vec<bool>)>,
    /// Exportación a una carpeta donde ya hay archivos con esos nombres: se pregunta
    reemplazar: Option<(Formato, Vec<(usize, PathBuf)>)>,
    /// Imágenes por exportar (archivo, ruta; PNG o SVG según la extensión); la primera es
    /// la que se está capturando.
    /// Mientras haya, se esconden los controles del lienzo.
    cola_png: VecDeque<(usize, PathBuf)>,
    /// Ya se pidió la captura de la primera de la cola
    pedida: bool,
    /// El archivo que estaba abierto antes de exportar imágenes, para volver a él
    volver_a: usize,
    /// Resultado de guardar la captura, que llega desde otro hilo
    captura: (mpsc::Sender<Exportado>, mpsc::Receiver<Exportado>),
    /// Proyectos leídos (soltados sobre la ventana, o subidos en el navegador)
    subidas: (Llegada, mpsc::Receiver<Leido>),
}

/// Un archivo de la lista que se está arrastrando (su índice)
struct Arrastre(usize);

/// Lo que se puede exportar de cada archivo
#[derive(Clone, Copy)]
enum Formato {
    Sa,
    Png,
    Svg,
    Csv,
}

impl Formato {
    const TODOS: [Formato; 4] = [Formato::Sa, Formato::Png, Formato::Svg, Formato::Csv];

    fn menu(self) -> &'static str {
        match self {
            Formato::Sa => "Archivo .SA…",
            Formato::Png => "Imagen (PNG)…",
            Formato::Svg => "Imagen (SVG)…",
            Formato::Csv => "Tabla de verdad (CSV)…",
        }
    }

    fn titulo(self) -> &'static str {
        match self {
            Formato::Sa => "Exportar como archivo .SA",
            Formato::Png => "Exportar como imagen",
            Formato::Svg => "Exportar como imagen SVG",
            Formato::Csv => "Exportar la tabla de verdad",
        }
    }

    fn tipo(self) -> &'static str {
        match self {
            Formato::Sa => "Circuito de LogicSM",
            Formato::Png => "Imagen PNG",
            Formato::Svg => "Imagen SVG",
            Formato::Csv => "Tabla CSV",
        }
    }

    fn ext(self) -> &'static str {
        match self {
            Formato::Sa => "SA",
            Formato::Png => "png",
            Formato::Svg => "svg",
            Formato::Csv => "csv",
        }
    }
}

impl Interfaz {
    /// Abre `proyecto` o, si no se pasa, el último que quedó abierto
    pub fn nueva(cc: &eframe::CreationContext, proyecto: Option<PathBuf>) -> Self {
        let ctx = &cc.egui_ctx;
        let recientes: Option<Vec<PathBuf>> =
            cc.storage.and_then(|s| eframe::get_value(s, "recientes"));
        let ultimo: Option<PathBuf> = cc
            .storage
            .and_then(|s| eframe::get_value::<Option<PathBuf>>(s, "ultimo"))
            .flatten();
        if recientes.is_none() {
            // Primera vez: claro. Después egui recuerda el tema elegido.
            ctx.set_theme(Theme::Light);
        }
        let textura = |nombre: &str, png: &[u8]| {
            // Con `image` y no con eframe::icon_data, que no existe en el navegador
            let icono = image::load_from_memory(png)
                .expect("los PNG de assets son válidos")
                .to_rgba8();
            let imagen = egui::ColorImage::from_rgba_unmultiplied(
                [icono.width() as usize, icono.height() as usize],
                icono.as_raw(),
            );
            ctx.load_texture(nombre, imagen, egui::TextureOptions::LINEAR)
        };
        let mut s = Self {
            proyecto: Proyecto::nuevo(),
            sin_proyecto: true,
            recientes: recientes.unwrap_or_default(),
            red: None,
            problemas: Vec::new(),
            avisos: Vec::new(),
            esperado: HashMap::new(),
            comparar: None,
            cuadradas: HashMap::new(),
            entradas: Vec::new(),
            estado: Estado::default(),
            estable: true,
            corriendo: false,
            frecuencia: 1.0,
            reloj: false,
            ultimo_tic: 0.0,
            tic: false,
            ondas: VecDeque::new(),
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
            logo: textura("logo", crate::LOGO),
            icono_sa: textura("icono_sa", crate::ICONO_SA),
            vista: Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)),
            ajustar: true,
            deshacer: Vec::new(),
            rehacer: Vec::new(),
            ultimo_lienzo: false,
            ir_a: None,
            rect_lienzo: Rect::NOTHING,
            elegir: None,
            reemplazar: None,
            cola_png: VecDeque::new(),
            pedida: false,
            volver_a: 0,
            captura: mpsc::channel(),
            subidas: mpsc::channel(),
        };
        // El que se pidió (si no existe, cargar avisa) o el último, si todavía existe
        if let Some(ruta) = proyecto.or(ultimo.filter(|r| r.is_file())) {
            s.cargar(ruta);
        }
        // En el navegador, el que quedó guardado en el navegador
        let guardado = cc
            .storage
            .and_then(|s| eframe::get_value::<Option<Subido>>(s, "proyecto_web"))
            .flatten();
        if let Some((nombre, texto)) = guardado.filter(|_| WEB) {
            s.usar(Proyecto::leido(PathBuf::from(nombre), &texto));
        }
        s
    }

    fn compilar(&mut self) {
        if self.sin_proyecto {
            self.red = None;
            self.problemas.clear();
            self.avisos.clear();
            return;
        }
        match self.proyecto.compilar() {
            Ok(red) => {
                self.problemas.clear();
                let n = red
                    .nodos
                    .iter()
                    .filter(|n| matches!(n.tipo, Tipo::Entrada(..)))
                    .count();
                self.entradas.resize(n, false);
                self.estado = Estado::default();
                self.ondas.clear();
                self.avisos = entradas_sin_usar(&red);
                self.red = Some(red);
            }
            Err(problemas) => {
                self.problemas = problemas;
                self.avisos.clear();
            }
        }
    }

    /// Se cambió de archivo o de proyecto: se dibuja desde cero
    fn recompilar_desde_cero(&mut self) {
        self.red = None;
        self.entradas.clear();
        self.ajustar = true;
        self.deshacer.clear();
        self.rehacer.clear();
        self.compilar();
    }

    /// Abre el `.SA` de un proyecto
    fn cargar(&mut self, ruta: PathBuf) {
        match Proyecto::abrir(ruta.clone()) {
            Ok(proyecto) => self.usar(proyecto),
            Err(e) => self.aviso(format!("No se pudo abrir {}: {e}", ruta.display())),
        }
    }

    /// Pasa a ser el proyecto abierto
    fn usar(&mut self, proyecto: Proyecto) {
        if let Some(ruta) = proyecto.ruta.clone() {
            self.salida.push(format!("Abierto: {}", ruta.display()));
            self.recordar(ruta);
        }
        self.proyecto = proyecto;
        self.sin_proyecto = false;
        self.recompilar_desde_cero();
    }

    /// La pone primera en los recientes
    fn recordar(&mut self, ruta: PathBuf) {
        self.recientes.retain(|r| *r != ruta);
        self.recientes.insert(0, ruta);
        self.recientes.truncate(RECIENTES);
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
                let Some(ruta) =
                    plataforma::guardar_como("Nuevo proyecto", plataforma::SA, "Proyecto.SA", None)
                else {
                    return;
                };
                let mut proyecto = Proyecto::nuevo();
                // En el navegador no se descarga hasta que se guarde
                if WEB {
                    proyecto.ruta = Some(ruta);
                    return self.usar(proyecto);
                }
                match proyecto.guardar(Some(ruta)) {
                    Ok(()) => self.usar(proyecto),
                    Err(e) => self.aviso(e),
                }
            }
            Accion::Cerrar => {
                self.proyecto = Proyecto::nuevo();
                self.sin_proyecto = true;
                self.recompilar_desde_cero();
            }
            Accion::Abrir => {
                if let Some(ruta) = plataforma::abrir_sa(ctx, "Abrir un proyecto", &self.subidas.0)
                {
                    self.cargar(ruta);
                }
            }
            Accion::AbrirRuta(ruta) => self.cargar(ruta),
            Accion::Salir => {
                self.saliendo = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Guarda en el archivo del proyecto, o pide nombre y lugar (`otra`, o si nunca
    /// se guardó). Devuelve `false` si no se guardó.
    fn guardar(&mut self, otra: bool) -> bool {
        if self.sin_proyecto {
            return false;
        }
        let nueva = if otra || self.proyecto.ruta.is_none() {
            let ruta = self.proyecto.ruta.as_deref();
            let nombre = ruta
                .and_then(|r| r.file_name())
                .map_or("Proyecto.SA".into(), |n| n.to_string_lossy());
            let carpeta = ruta.and_then(|r| r.parent());
            let Some(ruta) =
                plataforma::guardar_como("Guardar el proyecto", plataforma::SA, &nombre, carpeta)
            else {
                return false;
            };
            Some(ruta)
        } else {
            None
        };
        match self.proyecto.guardar(nueva) {
            Ok(()) => {
                if let Some(ruta) = self.proyecto.ruta.clone() {
                    let hecho = if WEB { "Descargado" } else { "Guardado en" };
                    self.salida.push(format!("{hecho} {}", ruta.display()));
                    self.recordar(ruta);
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
        if self.sin_proyecto {
            // Desde la pantalla de inicio: un proyecto nuevo solo con el ejemplo
            self.proyecto.archivos.clear();
            self.sin_proyecto = false;
        }
        for aviso in self.proyecto.agregar(archivos) {
            self.aviso(aviso);
        }
        let nombres: Vec<&str> = archivos.iter().map(|(nombre, _)| *nombre).collect();
        self.salida
            .push(format!("Agregado al proyecto: {}", nombres.join(", ")));
        self.recompilar_desde_cero();
    }

    /// Un `.SA` soltado sobre la ventana con un proyecto abierto: sus archivos se agregan
    /// y su circuito (el del último archivo) se declara en el archivo abierto
    fn importar(&mut self, otro: Proyecto) {
        if otro.ruta.is_some() && otro.ruta == self.proyecto.ruta && !WEB {
            self.aviso("Ese es el proyecto abierto.".into());
            return;
        }
        let archivos: Vec<(&str, &str)> = otro
            .archivos
            .iter()
            .map(|a| (a.nombre.as_str(), a.codigo.as_str()))
            .collect();
        let activo = self.proyecto.activo;
        for aviso in self.proyecto.agregar(&archivos) {
            self.aviso(aviso);
        }
        let nombres: Vec<&str> = archivos.iter().map(|(nombre, _)| *nombre).collect();
        self.salida
            .push(format!("Agregado al proyecto: {}", nombres.join(", ")));
        // `agregar` deja abierto el último: ese es el componente
        let componente = self.proyecto.activo;
        self.proyecto.activo = activo;
        self.declarar(componente);
    }

    /// Declara el circuito del archivo `i` al final del archivo abierto:
    /// `fullAdder1 = FullAdder(A, B, Cin);`, con sus entradas como argumentos
    fn declarar(&mut self, i: usize) {
        let archivo = &self.proyecto.archivos[i].nombre;
        if i == self.proyecto.activo {
            return self.aviso(format!("{archivo} no puede usarse dentro de sí mismo."));
        }
        let Ok(circuitos) = logicsm_core::parser::parsear(&self.proyecto.archivos[i].codigo) else {
            return self.aviso(format!(
                "{archivo}: tiene errores, corrígelo antes de usarlo."
            ));
        };
        let Some(c) = logicsm_core::principal(&circuitos) else {
            return self.aviso(format!("{archivo}: no tiene ningún circuito."));
        };
        if c.nombre.texto == "main" {
            return self.aviso(format!(
                "{archivo}: es un main y no se puede usar como componente \
                 (cámbialo a circuit {archivo})."
            ));
        }
        let circuito = c.nombre.texto.clone();
        let entradas: Vec<&str> = c.entradas.iter().map(|e| e.nombre.texto.as_str()).collect();
        let Some(linea) =
            insertar_declaracion(&mut self.proyecto.actual_mut().codigo, &circuito, &entradas)
        else {
            let actual = self.proyecto.actual().nombre.clone();
            return self.aviso(format!(
                "Escribe primero un circuito en {actual} para declarar {circuito} ahí."
            ));
        };
        self.salida.push(format!("Declarado: {linea}"));
        self.compilar();
    }

    /// Mientras se arrastra un archivo de la lista: su nombre junto al puntero, y si se
    /// suelta sobre `zona` (el código o el circuito) se declara en el archivo abierto
    fn soltar_componente(&mut self, ctx: &egui::Context, zona: Rect) {
        let Some(arrastre) = egui::DragAndDrop::payload::<Arrastre>(ctx) else {
            return;
        };
        let Some(archivo) = self.proyecto.archivos.get(arrastre.0) else {
            return;
        };
        let Some(puntero) = ctx.pointer_latest_pos() else {
            return;
        };
        let encima = zona.contains(puntero);
        egui::Area::new(egui::Id::unique("arrastre"))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .fixed_pos(puntero + vec2(14.0, 8.0))
            .show(ctx, |ui| {
                Frame::popup(ui.style()).show(ui, |ui| {
                    let texto = if encima {
                        format!("Declarar {}", archivo.nombre)
                    } else {
                        archivo.nombre.clone()
                    };
                    ui.label(texto);
                });
            });
        if encima && ctx.input(|i| i.pointer.any_released()) {
            egui::DragAndDrop::clear_payload(ctx);
            self.declarar(arrastre.0);
        }
    }

    /// Entradas en 0 y todo lo demás también, la memoria de los flip-flops incluida
    fn reiniciar(&mut self) {
        self.entradas.fill(false);
        self.reloj = false;
        self.ondas.clear();
        if let Some(red) = &self.red {
            self.estado.reiniciar(red);
        }
    }

    fn hay_reloj(&self) -> bool {
        self.red.as_ref().is_some_and(|r| {
            r.nodos
                .iter()
                .any(|n| matches!(n.tipo, Tipo::Entrada(_, Clase::Reloj)))
        })
    }

    /// Ejecutar / Pausa
    fn ejecutar(&mut self, ctx: &egui::Context) {
        self.corriendo = !self.corriendo && self.hay_reloj();
        self.ultimo_tic = ctx.input(|i| i.time);
    }

    /// Paso a paso: pausa y avanza medio ciclo del reloj
    fn paso(&mut self) {
        self.corriendo = false;
        if self.hay_reloj() {
            self.reloj = !self.reloj;
            self.tic = true;
        }
    }

    /// Con Ejecutar, da los medios ciclos que pasaron desde el último cuadro y
    /// simula después de cada uno
    fn latir(&mut self, ctx: &egui::Context) {
        let Some(red) = &self.red else { return };
        if !self.corriendo {
            return;
        }
        let ahora = ctx.input(|i| i.time);
        let medio = 0.5 / self.frecuencia;
        let mut tics = ((ahora - self.ultimo_tic) / medio) as usize;
        if tics > MAX_TICS {
            tics = MAX_TICS;
            self.ultimo_tic = ahora;
        } else {
            self.ultimo_tic += tics as f64 * medio;
        }
        for _ in 0..tics {
            self.reloj = !self.reloj;
            poner(red, &mut self.entradas, Clase::Reloj, self.reloj);
            self.estable = logicsm_sim::simular(red, &mut self.estado, &self.entradas);
            muestrear(red, &self.estado.valores, &mut self.ondas, true);
        }
        let falta = (self.ultimo_tic + medio - ahora).max(0.0);
        ctx.request_repaint_after(Duration::from_secs_f64(falta));
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
        if tecla(ctrl, Key::Q) && !WEB {
            self.pedir(ctx, Accion::Salir);
        }
        // Sin nada enfocado (el editor no tiene el cursor), Ctrl+Z deshace movimientos del lienzo
        if ctx.memory(|m| m.focused().is_none()) {
            if tecla(ctrl | Modifiers::SHIFT, Key::Z) || tecla(ctrl, Key::Y) {
                self.mover(true);
            } else if tecla(ctrl, Key::Z) {
                self.mover(false);
            }
        }
        // Primero la versión con Alt, como con Shift en Ctrl+S
        if tecla(ctrl | Modifiers::ALT, Key::B) {
            self.ver_senales = !self.ver_senales;
        } else if tecla(ctrl, Key::B) {
            self.ver_proyecto = !self.ver_proyecto;
        }
        if tecla(ctrl, Key::J) {
            self.ver_inferior = !self.ver_inferior;
        }
        if tecla(ctrl | Modifiers::SHIFT, Key::F5) {
            self.reiniciar();
        } else if tecla(Modifiers::NONE, Key::F5) {
            self.ejecutar(ctx);
        }
        if tecla(Modifiers::NONE, Key::F10) {
            self.paso();
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
        // Un archivo .SA soltado sobre la ventana: con un proyecto abierto se agrega como
        // componente; si no, se abre
        let soltado = ctx.input(|i| i.raw.dropped_files.first().cloned());
        if let Some(f) = soltado {
            if !es_sa(f.path()) {
                self.aviso(format!("{} no es un archivo .SA.", f.path().display()));
            } else if let Err(e) =
                plataforma::leer_soltado(ctx, f, self.sin_proyecto, &self.subidas.0)
            {
                self.aviso(format!("No se pudo abrir el archivo: {e}"));
            }
        }
        // Lo leído (soltado, o subido con Abrir en el navegador)
        while let Ok(((ruta, texto), abrir)) = self.subidas.1.try_recv() {
            let p = Proyecto::leido(PathBuf::from(ruta), &texto);
            if abrir {
                self.usar(p);
            } else {
                self.importar(p);
            }
        }
    }

    /// Deshace (o rehace) el último movimiento de piezas del archivo abierto
    fn mover(&mut self, rehacer: bool) {
        let (desde, hacia) = if rehacer {
            (&mut self.rehacer, &mut self.deshacer)
        } else {
            (&mut self.deshacer, &mut self.rehacer)
        };
        let Some(antes) = desde.pop() else {
            return;
        };
        let archivo = self.proyecto.actual().nombre.clone();
        hacia.push(
            self.proyecto
                .posiciones
                .insert(archivo, antes)
                .unwrap_or_default(),
        );
        self.ultimo_lienzo = true;
    }

    /// Guarda la posición actual para poder deshacer lo que viene
    fn antes_de_mover(&mut self, antes: Posiciones) {
        self.deshacer.push(antes);
        if self.deshacer.len() > DESHACER {
            self.deshacer.remove(0);
        }
        self.rehacer.clear();
        self.ultimo_lienzo = true;
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
                ui.label("Se quita del proyecto al guardar.");
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

        let Some(accion) = self.confirmar.clone() else {
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
        // Modal: centrada, dentro de la app, y no deja tocar nada más hasta cerrarla
        // (con ×, Esc o un click afuera)
        let mut cerrar = false;
        let r = egui::Modal::new(egui::Id::unique("ventana_ayuda")).show(ctx, |ui| {
            ui.set_width(560.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(titulo).size(15.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    cerrar = ui
                        .add(egui::Button::new(RichText::new("×").size(18.0)).frame(false))
                        .on_hover_text("Cerrar (Esc)")
                        .clicked();
                });
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(480.0)
                .show(ui, |ui| match ventana {
                    Ventana::Guia => ayuda::guia(ui),
                    Ventana::Atajos => ayuda::atajos(ui),
                    Ventana::Actualizar => ayuda::actualizar(ui),
                    Ventana::Acerca => ayuda::acerca(ui, &self.logo),
                });
        });
        if cerrar || r.should_close() {
            self.ventana = None;
        }
    }
}

impl eframe::App for Interfaz {
    /// Lo demás (tema, zoom, tamaño de la ventana) lo recuerdan egui y eframe
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let ultimo = (!self.sin_proyecto)
            .then(|| self.proyecto.ruta.clone())
            .flatten();
        eframe::set_value(storage, "recientes", &self.recientes);
        eframe::set_value(storage, "ultimo", &ultimo);
        // En el navegador no hay archivos: el proyecto abierto se guarda en el navegador
        if WEB {
            let abierto = (!self.sin_proyecto).then(|| {
                let nombre = self.proyecto.ruta.as_deref().and_then(|r| r.file_name());
                let nombre: String =
                    nombre.map_or("Proyecto.SA".into(), |n| n.to_string_lossy().into());
                (nombre, self.proyecto.texto())
            });
            eframe::set_value::<Option<Subido>>(storage, "proyecto_web", &abierto);
        }
    }

    /// En el navegador se guarda seguido: es lo único que conserva el proyecto
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(if WEB { 5 } else { 30 })
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.atajos(&ctx);
        while let Ok(resultado) = self.captura.1.try_recv() {
            self.informar(resultado);
            self.cola_png.pop_front();
            self.preparar_imagen();
        }
        self.corriendo &= self.hay_reloj();
        self.latir(&ctx);
        if let Some(red) = &self.red {
            poner(red, &mut self.entradas, Clase::Reloj, self.reloj);
            self.estable = logicsm_sim::simular(red, &mut self.estado, &self.entradas);
            muestrear(
                red,
                &self.estado.valores,
                &mut self.ondas,
                std::mem::take(&mut self.tic),
            );
            // Los pulsadores vuelven a 0; el dibujo los pone en 1 si siguen presionados.
            // Uno que se soltó necesita otro cuadro para simularse en 0.
            if poner(red, &mut self.entradas, Clase::Pulsador, false) {
                ctx.request_repaint();
            }
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
        if self.sin_proyecto {
            egui::CentralPanel::default()
                .frame(Frame::new().fill(c.fondo))
                .show(ui, |ui| self.bienvenida(ui));
            self.ventanas(&ctx);
            self.confirmacion(&ctx);
            return;
        }
        // Arrastrar el borde por debajo del mínimo oculta el panel; arrastrarlo de nuevo lo abre.
        // `take_available_space`: sin él egui achica el panel a su contenido y no se puede agrandar
        let (mut ver_proyecto, mut ver_senales, mut ver_inferior) =
            (self.ver_proyecto, self.ver_senales, self.ver_inferior);
        // El panel de abajo puede llegar hasta el 70 % del alto de la ventana
        let alto_inferior = (ui.available_height() * 0.7).max(200.0);
        egui::Panel::left("proyecto")
            .default_size(190.0)
            .size_range(150.0..=420.0)
            .resizable(true)
            .frame(
                Frame::new()
                    .fill(c.panel)
                    .inner_margin(Margin::symmetric(14, 15)),
            )
            .show_collapsible(ui, &mut ver_proyecto, |ui| {
                self.panel_proyecto(ui);
                ui.take_available_space();
            });
        egui::Panel::right("senales")
            .default_size(222.0)
            .size_range(170.0..=640.0)
            .resizable(true)
            .frame(
                Frame::new()
                    .fill(c.panel)
                    .inner_margin(Margin::symmetric(14, 15)),
            )
            .show_collapsible(ui, &mut ver_senales, |ui| {
                self.senales(ui);
                ui.take_available_space();
            });
        egui::Panel::bottom("pestanas")
            .default_size(166.0)
            .size_range(90.0..=alto_inferior)
            .resizable(true)
            .frame(Frame::new().fill(c.panel))
            .show_collapsible(ui, &mut ver_inferior, |ui| {
                self.pestanas(ui);
                ui.take_available_space();
            });
        (self.ver_proyecto, self.ver_senales, self.ver_inferior) =
            (ver_proyecto, ver_senales, ver_inferior);
        let codigo = egui::Panel::left("codigo")
            .default_size(420.0)
            .size_range(260.0..=900.0)
            .resizable(true)
            .frame(Frame::new().fill(c.codigo))
            .show(ui, |ui| self.editor(ui))
            .response
            .rect;
        let lienzo = egui::CentralPanel::default()
            .frame(Frame::new().fill(c.fondo))
            .show(ui, |ui| self.lienzo(ui))
            .response
            .rect;
        self.soltar_componente(&ctx, codigo.union(lienzo));
        self.ventanas(&ctx);
        self.confirmacion(&ctx);
        self.ventana_exportar(&ctx);
    }
}

/// Formatos de exportación que faltan
// ponytail: deshabilitados hasta que exista cada exportador
const FORMATOS: [&str; 2] = ["LogicCircuit (.CircuitProject)", "Verilog (.v)"];

/// Mensaje para Salida, o el error para avisar
type Exportado = Result<String, String>;

/// Más entradas que esto hacen una tabla de verdad demasiado grande para exportar
const MAX_ENTRADAS_CSV: usize = 16;

impl Interfaz {
    /// Menú Archivo → Exportar
    fn exportar(&mut self, ui: &mut egui::Ui) {
        let libre = self.cola_png.is_empty();
        for formato in Formato::TODOS {
            if ui
                .add_enabled(libre, egui::Button::new(formato.menu()))
                .on_disabled_hover_text("Exportando imágenes…")
                .clicked()
            {
                if self.proyecto.archivos.len() == 1 {
                    self.exportar_a(ui.ctx(), formato, vec![0]);
                } else {
                    // Marcado de entrada: el archivo abierto
                    let marcados = (0..self.proyecto.archivos.len())
                        .map(|i| i == self.proyecto.activo)
                        .collect();
                    self.elegir = Some((formato, marcados));
                }
            }
        }
        for formato in FORMATOS {
            ui.add_enabled(false, egui::Button::new(formato))
                .on_disabled_hover_text("Todavía no disponible");
        }
    }

    /// Ventanas de Exportar: qué archivos, y si se reemplazan los que ya existen
    fn ventana_exportar(&mut self, ctx: &egui::Context) {
        if let Some((formato, marcados)) = &mut self.elegir {
            let formato = *formato;
            let mut listo = None;
            let r = egui::Modal::new(egui::Id::unique("exportar")).show(ctx, |ui| {
                ui.set_width(340.0);
                ui.label(RichText::new(formato.titulo()).size(14.0).strong());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("¿Qué archivos?");
                    if ui.small_button("Todos").clicked() {
                        marcados.fill(true);
                    }
                    if ui.small_button("Ninguno").clicked() {
                        marcados.fill(false);
                    }
                });
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for (a, marcado) in self.proyecto.archivos.iter().zip(marcados.iter_mut()) {
                            ui.checkbox(marcado, &a.nombre);
                        }
                    });
                ui.add_space(8.0);
                let cuantos = marcados.iter().filter(|&&m| m).count();
                ui.label(
                    RichText::new(match cuantos {
                        0 => "Marca al menos uno.".to_string(),
                        1 => "Luego eliges el nombre y dónde guardarlo.".to_string(),
                        n => format!("Luego eliges la carpeta para los {n}."),
                    })
                    .size(11.5)
                    .color(paleta(ui).tenue),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(cuantos > 0, egui::Button::new("Exportar"))
                        .clicked()
                    {
                        listo = Some(true);
                    }
                    if ui.button("Cancelar").clicked() {
                        listo = Some(false);
                    }
                });
            });
            if r.should_close() && listo.is_none() {
                listo = Some(false);
            }
            if let Some(exportar) = listo
                && let Some((formato, marcados)) = self.elegir.take()
                && exportar
            {
                let indices = (0..marcados.len()).filter(|&i| marcados[i]).collect();
                self.exportar_a(ctx, formato, indices);
            }
        }

        let Some((formato, destinos)) = &self.reemplazar else {
            return;
        };
        let existen: Vec<String> = destinos
            .iter()
            .filter(|(_, r)| r.exists())
            .map(|(_, r)| r.file_name().unwrap_or_default().to_string_lossy().into())
            .collect();
        let formato = *formato;
        let mut reemplazar = None;
        let r = egui::Modal::new(egui::Id::unique("reemplazar")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(
                RichText::new("Ya existen en esa carpeta")
                    .size(14.0)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(existen.join(", "));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Reemplazar").clicked() {
                    reemplazar = Some(true);
                }
                if ui.button("Cancelar").clicked() {
                    reemplazar = Some(false);
                }
            });
        });
        if r.should_close() && reemplazar.is_none() {
            reemplazar = Some(false);
        }
        if let Some(si) = reemplazar
            && let Some((_, destinos)) = self.reemplazar.take()
            && si
        {
            self.escribir(formato, destinos);
        }
    }

    /// Pide dónde guardar (un archivo, o una carpeta si son varios) y exporta
    fn exportar_a(&mut self, ctx: &egui::Context, formato: Formato, indices: Vec<usize>) {
        let ext = formato.ext();
        let carpeta_proyecto = self
            .proyecto
            .ruta
            .as_deref()
            .and_then(|r| r.parent())
            .map(PathBuf::from);
        let destinos: Vec<(usize, PathBuf)> = if let [i] = indices[..] {
            let nombre = format!("{}.{ext}", self.proyecto.archivos[i].nombre);
            let extensiones = [ext];
            let filtro = (formato.tipo(), &extensiones[..]);
            let carpeta = carpeta_proyecto.as_deref();
            let Some(ruta) = plataforma::guardar_como(formato.titulo(), filtro, &nombre, carpeta)
            else {
                return;
            };
            let tiene = ruta
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(ext));
            vec![(
                i,
                if tiene {
                    ruta
                } else {
                    ruta.with_extension(ext)
                },
            )]
        } else {
            let titulo = format!("{}: elige la carpeta", formato.titulo());
            let Some(carpeta) = plataforma::elegir_carpeta(&titulo, carpeta_proyecto.as_deref())
            else {
                return;
            };
            indices
                .into_iter()
                .map(|i| {
                    let nombre = format!("{}.{ext}", self.proyecto.archivos[i].nombre);
                    (i, carpeta.join(nombre))
                })
                .collect()
        };
        // Nunca se escribe encima del proyecto (un archivo con su mismo nombre, en .SA);
        // en el navegador todo se descarga
        if !WEB
            && let Some(propio) = &self.proyecto.ruta
            && destinos.iter().any(|(_, r)| r == propio)
        {
            self.aviso(format!(
                "Exportar ahí reemplazaría el proyecto {}: elige otra carpeta.",
                propio.display()
            ));
            return;
        }
        // Con un solo archivo, el diálogo del sistema ya preguntó si se reemplaza
        if destinos.len() > 1 && destinos.iter().any(|(_, r)| r.exists()) {
            self.reemplazar = Some((formato, destinos));
            return;
        }
        self.escribir(formato, destinos);
        ctx.request_repaint();
    }

    fn escribir(&mut self, formato: Formato, destinos: Vec<(usize, PathBuf)>) {
        if let Formato::Png | Formato::Svg = formato {
            self.volver_a = self.proyecto.activo;
            self.cola_png = destinos.into();
            self.preparar_imagen();
            return;
        }
        for (i, ruta) in destinos {
            let contenido = match formato {
                Formato::Csv => self
                    .proyecto
                    .compilar_archivo(i)
                    .map_err(|_| "tiene errores".to_string())
                    .and_then(|red| tabla_csv(&red)),
                _ => Ok(self.proyecto.archivos[i].codigo.clone()),
            };
            let resultado = contenido
                .map_err(|e| format!("{}: {e}, no se exportó.", self.proyecto.archivos[i].nombre))
                .and_then(|texto| {
                    plataforma::escribir(&ruta, texto.as_bytes())
                        .map(|()| format!("Exportado: {}", ruta.display()))
                        .map_err(|e| format!("No se pudo exportar {}: {e}", ruta.display()))
                });
            self.informar(resultado);
        }
    }

    /// Abre el siguiente archivo de la cola de imágenes y lo encuadra; el lienzo lo
    /// captura cuando ya está dibujado así. Al terminar vuelve al archivo de antes.
    fn preparar_imagen(&mut self) {
        while let Some(&(i, _)) = self.cola_png.front() {
            if i != self.proyecto.activo {
                self.abrir_archivo(i);
            }
            if self.red.is_some() {
                self.ajustar = true;
                self.pedida = false;
                return;
            }
            let nombre = self.proyecto.archivos[i].nombre.clone();
            self.aviso(format!("{nombre}: tiene errores, no se exportó."));
            self.cola_png.pop_front();
        }
        if self.volver_a != self.proyecto.activo && self.volver_a < self.proyecto.archivos.len() {
            self.abrir_archivo(self.volver_a);
        }
    }

    fn informar(&mut self, resultado: Exportado) {
        match resultado {
            Ok(m) => self.salida.push(m),
            Err(e) => self.aviso(e),
        }
    }

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
            let hay = self.hay_reloj();
            let (texto, icono) = if self.corriendo {
                ("Pausa", Icono::Pausa)
            } else {
                ("Ejecutar", Icono::Ejecutar)
            };
            let (ejecutar, paso) = if hay {
                (
                    "Hace latir el reloj (F5)",
                    "Avanza medio ciclo del reloj (F10)",
                )
            } else {
                (SIN_RELOJ, SIN_RELOJ)
            };
            if ui
                .add_enabled_ui(hay, |ui| boton(ui, texto, icono, true))
                .inner
                .on_hover_text(ejecutar)
                .on_disabled_hover_text(ejecutar)
                .clicked()
            {
                self.ejecutar(ui.ctx());
            }
            if ui
                .add_enabled_ui(hay, |ui| boton(ui, "Paso a paso", Icono::Paso, false))
                .inner
                .on_hover_text(paso)
                .on_disabled_hover_text(paso)
                .clicked()
            {
                self.paso();
            }
            egui::ComboBox::from_id_salt("frecuencia")
                .width(70.0)
                .selected_text(hz(self.frecuencia))
                .show_ui(ui, |ui| {
                    for f in FRECUENCIAS {
                        ui.selectable_value(&mut self.frecuencia, f, hz(f));
                    }
                })
                .response
                .on_hover_text("Frecuencia del reloj");
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
                ui.add_space(6.0);
                // De derecha a izquierda: quedan en el orden del dibujo (izquierdo, abajo, derecho)
                if boton_panel(
                    ui,
                    self.ver_senales,
                    Lado::Derecha,
                    "Entradas y salidas (Ctrl+Alt+B)",
                ) {
                    self.ver_senales = !self.ver_senales;
                }
                if boton_panel(
                    ui,
                    self.ver_inferior,
                    Lado::Abajo,
                    "Panel inferior (Ctrl+J)",
                ) {
                    self.ver_inferior = !self.ver_inferior;
                }
                if boton_panel(
                    ui,
                    self.ver_proyecto,
                    Lado::Izquierda,
                    "Panel de proyecto (Ctrl+B)",
                ) {
                    self.ver_proyecto = !self.ver_proyecto;
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
            if opcion(ui, "Nuevo proyecto…", "Ctrl+N") {
                self.pedir(&ctx, Accion::Nuevo);
            }
            if opcion(ui, "Abrir proyecto…", "Ctrl+O") {
                self.pedir(&ctx, Accion::Abrir);
            }
            if !self.sin_proyecto && opcion(ui, "Nuevo archivo", "") {
                self.ver_proyecto = true;
                self.nombrando = Some((None, String::new()));
                self.error_nombre = None;
            }
            ui.separator();
            if opcion(ui, "Guardar", "Ctrl+S") {
                self.guardar(false);
            }
            if opcion(ui, "Guardar como…", "Ctrl+Shift+S") {
                self.guardar(true);
            }
            ui.menu_button("Exportar", |ui| self.exportar(ui));
            ui.separator();
            if !self.sin_proyecto && opcion(ui, "Cerrar proyecto", "") {
                self.pedir(&ctx, Accion::Cerrar);
            }
            if !WEB && opcion(ui, "Salir", "Ctrl+Q") {
                self.pedir(&ctx, Accion::Salir);
            }
        });
        ui.menu_button("Editar", |ui| {
            // Deshace en el lienzo si lo último fue mover piezas; si no, en el editor
            if opcion(ui, "Deshacer", "Ctrl+Z") {
                if self.ultimo_lienzo {
                    self.mover(false);
                } else {
                    self.para_editor = Some(tecla(egui::Key::Z));
                }
            }
            if opcion(ui, "Rehacer", "Ctrl+Y") {
                if self.ultimo_lienzo {
                    self.mover(true);
                } else {
                    self.para_editor = Some(tecla(egui::Key::Y));
                }
            }
            ui.separator();
            if opcion(ui, "Cortar", "Ctrl+X") {
                self.para_editor = Some(egui::Event::Cut);
            }
            if opcion(ui, "Copiar", "Ctrl+C") {
                self.para_editor = Some(egui::Event::Copy);
            }
            if opcion(ui, "Pegar", "Ctrl+V") {
                match plataforma::pegar() {
                    Ok(texto) => self.para_editor = Some(egui::Event::Paste(texto)),
                    Err(e) => self.aviso(e),
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
            ui.checkbox(&mut self.ver_senales, "Entradas y salidas  (Ctrl+Alt+B)");
            ui.checkbox(&mut self.ver_inferior, "Panel inferior  (Ctrl+J)");
        });
        ui.menu_button("Simulación", |ui| {
            let hay = self.hay_reloj();
            let texto = if self.corriendo { "Pausa" } else { "Ejecutar" };
            if ui
                .add_enabled(hay, egui::Button::new(texto).shortcut_text("F5"))
                .on_disabled_hover_text(SIN_RELOJ)
                .clicked()
            {
                self.ejecutar(&ctx);
            }
            if ui
                .add_enabled(hay, egui::Button::new("Paso a paso").shortcut_text("F10"))
                .on_disabled_hover_text(SIN_RELOJ)
                .clicked()
            {
                self.paso();
            }
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
            // En el navegador siempre está la última versión
            if !WEB && opcion(ui, "Buscar actualizaciones", "") {
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
        titulo(ui, "Proyecto");
        // Carpeta y botón para crear archivos
        let marca = if self.proyecto.sin_guardar() {
            " •"
        } else {
            ""
        };
        let carpeta = match &self.proyecto.ruta {
            Some(r) => r
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            None => "Sin guardar".into(),
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{carpeta}{marca}"))
                    .size(12.0)
                    .strong(),
            )
            .on_hover_text("El proyecto: un solo archivo .SA con todos estos adentro");
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
                let marca = if a.sin_guardar() { " •" } else { "" };
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
                    // Arrastrarlo al código o al circuito lo declara ahí
                    let r = ui.add(boton.sense(Sense::click_and_drag()));
                    r.dnd_set_drag_payload(Arrastre(i));
                    if r.clicked() && i != self.proyecto.activo {
                        self.abrir_archivo(i);
                    }
                    r.context_menu(|ui| {
                        if ui.button("Renombrar…").clicked() {
                            let nombre = self.proyecto.archivos[i].nombre.clone();
                            self.nombrando = Some((Some(i), nombre));
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
            egui::Button::new(RichText::new("Agregar ejemplo").size(12.0))
                .min_size(vec2(ui.available_width(), 28.0)),
        );
        flecha_abajo(ui, &ejemplos);
        egui::Popup::menu(&ejemplos).show(|ui| self.lista_de_ejemplos(ui));
    }

    /// Pantalla de inicio, sin proyecto abierto (como la bienvenida de VS Code)
    fn bienvenida(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let c = paleta(ui);
        let ancho = 260.0;
        let opcion = |ui: &mut egui::Ui, texto: &str, atajo: &str| {
            ui.add(
                egui::Button::new(RichText::new(texto).size(13.0))
                    .shortcut_text(atajo)
                    .min_size(vec2(ancho, 34.0)),
            )
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.18).max(30.0));
                ui.image((self.logo.id(), vec2(64.0, 64.0)));
                ui.add_space(12.0);
                ui.label(RichText::new("Pura Carpintería").size(30.0).strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Aquí va el código. Crea un proyecto o abre uno para empezar.")
                        .size(13.0)
                        .color(c.tenue),
                );
                ui.add_space(24.0);
                // Sin proyecto no hay cambios que perder: no hace falta confirmar
                if opcion(ui, "Nuevo proyecto…", "Ctrl+N").clicked() {
                    self.hacer(&ctx, Accion::Nuevo);
                }
                if opcion(ui, "Abrir proyecto…", "Ctrl+O").clicked() {
                    self.hacer(&ctx, Accion::Abrir);
                }
                let ejemplos = opcion(ui, "Empezar con un ejemplo", "");
                flecha_abajo(ui, &ejemplos);
                egui::Popup::menu(&ejemplos).show(|ui| self.lista_de_ejemplos(ui));

                // Solo las que todavía existen
                let recientes: Vec<PathBuf> = self
                    .recientes
                    .iter()
                    .filter(|r| r.is_file())
                    .cloned()
                    .collect();
                if !recientes.is_empty() {
                    ui.add_space(26.0);
                    ui.label(RichText::new("Recientes").size(12.0).strong());
                    ui.add_space(4.0);
                    for ruta in recientes {
                        let nombre = ruta.file_stem().unwrap_or_default().to_string_lossy();
                        let icono = egui::Image::new((self.icono_sa.id(), vec2(18.0, 18.0)));
                        if ui
                            .add(
                                egui::Button::image_and_text(
                                    icono,
                                    RichText::new(nombre).size(13.0),
                                )
                                .frame(false),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(ruta.display().to_string())
                            .clicked()
                        {
                            self.hacer(&ctx, Accion::AbrirRuta(ruta.clone()));
                        }
                    }
                }
                ui.add_space(26.0);
                vacio(ui, "También puedes arrastrar un archivo .SA a la ventana.");
            });
        });
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
        let marca = if a.sin_guardar() { " •" } else { "" };
        format!("Código — {}{marca}", a.nombre)
    }
}

/// Triángulo ▾ a la derecha de un botón que abre un menú. Se dibuja en vez de escribirlo:
/// en egui los símbolos dependen de las fuentes del sistema y en Linux salía un cuadro vacío.
fn flecha_abajo(ui: &egui::Ui, boton: &egui::Response) {
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
fn zigzag(p: &egui::Painter, x0: f32, x1: f32, y: f32, color: Color32) {
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
fn colorear(codigo: &str, fuente: &FontId, c: &Paleta) -> egui::text::LayoutJob {
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
            if let Tipo::Entrada(nombre, clase) = &n.tipo {
                let v = &mut self.entradas[k];
                let texto = format!("{nombre} = {}", *v as u8);
                match clase {
                    Clase::Interruptor => {
                        ui.toggle_value(v, texto);
                    }
                    Clase::Pulsador => {
                        let b = ui.add(egui::Button::new(texto).selected(*v));
                        if b.is_pointer_button_down_on() {
                            *v = true;
                            ui.ctx().request_repaint();
                        }
                    }
                    Clase::Reloj => {
                        ui.add_enabled(false, egui::Button::new(texto).selected(*v))
                            .on_disabled_hover_text("Lo mueven Ejecutar y Paso a paso");
                    }
                }
                k += 1;
            }
        }
        ui.add_space(24.0);
        titulo(ui, "Salidas");
        for (n, &v) in red.nodos.iter().zip(&self.estado.valores) {
            if let Tipo::Salida(nombre) = &n.tipo {
                ui.label(
                    RichText::new(format!("{nombre} = {}", v.simbolo()))
                        .monospace()
                        .color(c.de(v)),
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
        let mut area = egui::ScrollArea::vertical();
        // Click en Problemas: cursor al inicio de la línea y la línea a la vista
        if let Some((caracter, linea)) = self.ir_a.take() {
            let mut estado = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
            let cursor = egui::text::CCursor::new(caracter);
            estado
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(cursor)));
            estado.store(ui.ctx(), id);
            ui.memory_mut(|m| m.request_focus(id));
            let alto = ui.text_style_height(&egui::TextStyle::Monospace);
            area = area.vertical_scroll_offset((linea as f32 - 4.0).max(0.0) * alto);
        }
        let c = paleta(ui);
        let fuente = egui::TextStyle::Monospace.resolve(ui.style());
        // ponytail: colorea todo el archivo en cada cuadro; guardar el resultado si un archivo largo se pone lento
        let mut colores = |ui: &egui::Ui, texto: &dyn egui::TextBuffer, ancho: f32| {
            let mut trabajo = colorear(texto.as_str(), &fuente, c);
            trabajo.wrap.max_width = ancho;
            ui.painter().layout_job(trabajo)
        };
        area.show(ui, |ui| {
            ui.horizontal_top(|ui| {
                // Columna de números de línea: se llena después, con las filas del texto
                let lineas = self.proyecto.actual().codigo.matches('\n').count() + 1;
                let digito = ui
                    .painter()
                    .layout_no_wrap("0".into(), fuente.clone(), c.tenue)
                    .size()
                    .x;
                let ancho = lineas.to_string().len().max(2) as f32 * digito + 6.0;
                let (numeros, _) = ui.allocate_exact_size(vec2(ancho, 0.0), Sense::hover());

                let salida = egui::TextEdit::multiline(&mut self.proyecto.actual_mut().codigo)
                    .id(id)
                    .code_editor()
                    .frame(Frame::new())
                    .desired_width(f32::INFINITY)
                    .desired_rows(40)
                    .layouter(&mut colores)
                    .show(ui);
                if salida.response.changed() {
                    self.ultimo_lienzo = false;
                    self.compilar();
                }

                // La línea del cursor va más fuerte. Una línea larga ocupa varias filas
                // (se parte al ancho del editor): el número va en la primera.
                let cursor = salida.cursor_range.map(|r| {
                    let codigo = &self.proyecto.actual().codigo;
                    codigo
                        .chars()
                        .take(r.primary.index.into())
                        .filter(|&ch| ch == '\n')
                        .count()
                        + 1
                });
                let mut linea = 1;
                let mut empieza = true;
                for fila in &salida.galley.rows {
                    if empieza {
                        let color = if cursor == Some(linea) {
                            c.tinta
                        } else {
                            c.tenue
                        };
                        ui.painter().text(
                            pos2(numeros.right(), salida.galley_pos.y + fila.pos.y),
                            Align2::RIGHT_TOP,
                            linea.to_string(),
                            fuente.clone(),
                            color,
                        );
                    }
                    empieza = fila.ends_with_newline;
                    linea += fila.ends_with_newline as usize;
                }

                // Subrayado en zigzag bajo cada error de este archivo
                let actual = self.proyecto.actual();
                for (archivo, rango) in self.problemas.iter().filter_map(|p| p.lugar.as_ref()) {
                    if *archivo != actual.nombre {
                        continue;
                    }
                    let caracter = |byte: usize| {
                        let antes = actual.codigo.get(..byte).unwrap_or(&actual.codigo);
                        egui::text::CCursor::new(antes.chars().count())
                    };
                    let desde = salida.galley.pos_from_cursor(caracter(rango.start));
                    let hasta = salida.galley.pos_from_cursor(caracter(rango.end));
                    // Sin ancho (fin del archivo) o partido en dos filas: un carácter
                    let fin = if hasta.top() == desde.top() && hasta.left() > desde.left() {
                        hasta.left()
                    } else {
                        desde.left() + digito
                    };
                    let origen = salida.galley_pos.to_vec2();
                    zigzag(
                        ui.painter(),
                        desde.left() + origen.x,
                        fin + origen.x,
                        desde.bottom() + origen.y,
                        c.error,
                    );
                }
            });
        });
    }

    /// Click en Problemas: abre ese archivo con el cursor donde está el error
    fn ir_al_problema(&mut self, archivo: &str, byte: usize) {
        let Some(i) = self
            .proyecto
            .archivos
            .iter()
            .position(|a| a.nombre == archivo)
        else {
            return;
        };
        if i != self.proyecto.activo {
            self.abrir_archivo(i);
        }
        let codigo = &self.proyecto.actual().codigo;
        let antes = codigo.get(..byte).unwrap_or(codigo);
        let linea = antes.matches('\n').count() + 1;
        self.ir_a = Some((antes.chars().count(), linea));
    }

    fn lienzo(&mut self, ui: &mut egui::Ui) {
        let c = paleta(ui);
        cabecera(ui, "Circuito");
        // Un margen sin el lienzo alrededor: si no, arrastrar el fondo (mover la vista)
        // le gana al borde de los paneles vecinos y no se pueden agrandar
        let afuera = ui.available_rect_before_wrap().shrink(12.0);
        self.rect_lienzo = afuera;
        let Some(red) = &self.red else {
            ui.painter().text(
                afuera.center(),
                Align2::CENTER_CENTER,
                "Escribe un circuito en el editor",
                FontId::monospace(13.0),
                c.tenue,
            );
            return;
        };

        let archivo = self.proyecto.actual().nombre.clone();
        let cuadradas = self.cuadradas.entry(archivo.clone()).or_default();
        let posiciones = self.proyecto.posiciones.entry(archivo).or_default();
        // También la capa del dibujo y cuántas figuras son la cuadrícula (para el SVG)
        let (capa, cuadricula, dibujo) = ui
            .scope_builder(egui::UiBuilder::new().max_rect(afuera), |ui| {
                // Dos dedos (o la rueda) acercan y alejan: winit 0.30 no manda el pellizco en
                // Linux. Arrastrar mueve la vista.
                egui::Scene::new()
                    .zoom_range(0.2..=3.0)
                    .scroll_zooms(true)
                    .show(ui, &mut self.vista, |ui| {
                        puntos(ui);
                        let capa = ui.layer_id();
                        let cuadricula = ui
                            .ctx()
                            .graphics(|g| g.get(capa).map_or(0, |l| l.all_entries().len()));
                        let dibujo = dibujar(
                            ui,
                            red,
                            &self.estado.valores,
                            &mut self.entradas,
                            posiciones,
                            &self.ondas,
                            cuadradas,
                        );
                        (capa, cuadricula, dibujo)
                    })
                    .inner
            })
            .inner;
        if let Some(antes) = dibujo.antes {
            self.antes_de_mover(antes);
        }
        if self.ajustar && dibujo.contenido.is_positive() {
            self.ajustar = false;
            let r = dibujo.contenido.expand(50.0);
            // Sin agrandar un circuito chico: se ve a tamaño normal, centrado
            self.vista = Rect::from_center_size(r.center(), r.size().max(afuera.size()));
            ui.ctx().request_repaint();
        }

        // Exportando imágenes: se captura cuando ya se dibujó encuadrado
        if let Some((_, ruta)) = self.cola_png.front() {
            if !self.pedida && !self.ajustar && ruta.extension().is_some_and(|e| e == "svg") {
                self.pedida = true;
                let figuras: Vec<Shape> = ui.ctx().graphics(|g| {
                    g.get(capa).map_or(Vec::new(), |l| {
                        l.all_entries()
                            .skip(cuadricula)
                            .map(|f| f.shape.clone())
                            .collect()
                    })
                });
                let texto = svg(&figuras, c.fondo);
                let resultado = plataforma::escribir(ruta, texto.as_bytes())
                    .map(|()| format!("Exportado: {}", ruta.display()))
                    .map_err(|e| format!("No se pudo exportar {}: {e}", ruta.display()));
                let _ = self.captura.0.send(resultado);
                ui.ctx().request_repaint();
            } else if !self.pedida && !self.ajustar {
                self.pedida = true;
                capturar(
                    ui.ctx(),
                    self.rect_lienzo,
                    ruta.clone(),
                    self.captura.0.clone(),
                );
            }
            return;
        }
        // Controles del lienzo, abajo a la derecha, por encima del dibujo
        let mut zoom = None;
        egui::Area::new(egui::Id::unique("controles_lienzo"))
            .order(egui::Order::Middle)
            .fixed_pos(afuera.right_bottom() - vec2(12.0, 12.0))
            .pivot(Align2::RIGHT_BOTTOM)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    let control = |ui: &mut egui::Ui, texto: &str, ayuda: &str| {
                        ui.add(
                            egui::Button::new(RichText::new(texto).size(11.0).color(c.tenue))
                                .min_size(vec2(29.0, 27.0)),
                        )
                        .on_hover_text(ayuda)
                        .clicked()
                    };
                    if control(ui, "-", "Alejar (rueda o dos dedos)") {
                        zoom = Some(1.25);
                    }
                    if control(ui, "+", "Acercar (rueda o dos dedos)") {
                        zoom = Some(0.8);
                    }
                    if control(ui, "Ajustar", "Muestra todo el circuito") {
                        self.ajustar = true;
                    }
                    if control(ui, "Reordenar", "Devuelve las piezas movidas a su lugar") {
                        let archivo = self.proyecto.actual().nombre.clone();
                        if let Some(antes) = self.proyecto.posiciones.remove(&archivo) {
                            self.antes_de_mover(antes);
                        }
                        self.ajustar = true;
                    }
                });
            });
        if let Some(f) = zoom {
            self.vista = Rect::from_center_size(self.vista.center(), self.vista.size() * f);
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
        problemas.extend(self.avisos.iter().cloned().map(Problema::sin_lugar));
        if !self.estable {
            problemas.push(Problema::sin_lugar(
                "El circuito oscila: nunca llega a un valor estable.".into(),
            ));
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
        let mut ir_a = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.vertical(|ui| match self.pestana {
                    Pestana::Problemas => {
                        if problemas.is_empty() {
                            vacio(ui, "Sin problemas.");
                        }
                        for problema in &problemas {
                            let texto = RichText::new(&problema.texto).size(12.0).monospace();
                            let Some(lugar) = &problema.lugar else {
                                ui.label(texto);
                                continue;
                            };
                            let r = ui.add(egui::Label::new(texto).sense(Sense::click()));
                            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                ir_a = Some(lugar.clone());
                            }
                        }
                    }
                    Pestana::Tabla => self.tabla_de_verdad(ui),
                    Pestana::Ondas => vacio(ui, "Las ondas de cada probe se dibujan en el lienzo."),
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
        if let Some((archivo, rango)) = ir_a {
            self.ir_al_problema(&archivo, rango.start);
        }
    }

    /// Todas las combinaciones de entradas y lo que vale cada salida, con la columna
    /// "esperado" de cada salida: verifica el circuito, crea uno desde lo esperado y lo
    /// compara con otro archivo
    fn tabla_de_verdad(&mut self, ui: &mut egui::Ui) {
        let c = paleta(ui);
        let Some(red) = &self.red else {
            vacio(ui, "Escribe un circuito para ver su tabla de verdad.");
            return;
        };
        if contar_entradas(red) > MAX_ENTRADAS_TABLA {
            vacio(ui, "Con más de 8 entradas la tabla sería demasiado larga.");
            return;
        }
        // ponytail: se simula en cada cuadro; guardar la tabla si se nota lento
        let tabla = Tabla::de(red);
        let circuito = red.nombre.clone();
        let archivo = self.proyecto.actual().nombre.clone();
        let (filas, salidas) = (tabla.filas.len(), tabla.salidas.len());
        let esperado = self.esperado.entry(archivo.clone()).or_default();
        if esperado.len() != filas || esperado.first().is_some_and(|f| f.len() != salidas) {
            *esperado = vec![vec![None; salidas]; filas];
        }

        // Verificación: la primera celda marcada que no coincide
        let marcadas = esperado.iter().flatten().filter(|e| e.is_some()).count();
        let falla = (0..filas).find_map(|f| {
            (0..salidas).find_map(|k| {
                esperado[f][k]
                    .filter(|&e| tabla.filas[f][k] != Valor::de(e))
                    .map(|e| (f, k, e))
            })
        });
        let mensaje = match falla {
            Some((f, k, e)) => RichText::new(format!(
                "Con {} tu salida {} da {} y debería dar {}.",
                combinacion(&tabla, f),
                tabla.salidas[k].0,
                tabla.filas[f][k].simbolo(),
                e as u8
            ))
            .color(c.error),
            None if marcadas > 0 => {
                RichText::new(format!("Las {marcadas} celdas marcadas coinciden.")).color(c.uno)
            }
            None => RichText::new(
                "Llena la columna \"esperado\" (click en -) para verificar tu circuito o crear uno desde la tabla.",
            )
            .color(c.tenue),
        };
        ui.label(mensaje.size(12.0));

        let mut crear = false;
        ui.horizontal(|ui| {
            crear = ui
                .add_enabled(
                    marcadas > 0,
                    egui::Button::new("Crear circuito desde lo esperado"),
                )
                .on_hover_text(
                    "Escribe el circuito más simple que da lo esperado; las celdas en - dan igual",
                )
                .clicked();
            if ui
                .add_enabled(marcadas > 0, egui::Button::new("Borrar lo esperado"))
                .clicked()
            {
                esperado.iter_mut().for_each(|f| f.fill(None));
            }
            ui.separator();
            ui.label("Comparar con");
            egui::ComboBox::from_id_salt("comparar")
                .selected_text(self.comparar.as_deref().unwrap_or("-"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.comparar, None, "-");
                    for a in &self.proyecto.archivos {
                        if a.nombre != archivo {
                            ui.selectable_value(
                                &mut self.comparar,
                                Some(a.nombre.clone()),
                                &a.nombre,
                            );
                        }
                    }
                });
        });
        if let Some(otro) = &self.comparar {
            let (texto, color) = match comparar(&self.proyecto, &tabla, otro) {
                Ok(texto) => (texto, c.uno),
                Err(texto) => (texto, c.error),
            };
            ui.label(RichText::new(texto).size(12.0).color(color));
        }
        ui.add_space(6.0);

        let bit = |v: Valor| RichText::new(v.simbolo()).monospace();
        egui::Grid::new("tabla_de_verdad")
            .striped(true)
            .spacing(vec2(18.0, 3.0))
            .show(ui, |ui| {
                for nombre in &tabla.entradas {
                    ui.label(RichText::new(nombre).monospace().strong());
                }
                for (nombre, _) in &tabla.salidas {
                    ui.label(RichText::new(nombre).monospace().strong().color(c.lavanda));
                }
                for (nombre, _) in &tabla.salidas {
                    ui.label(
                        RichText::new(format!("{nombre} esperado"))
                            .monospace()
                            .color(c.tenue),
                    );
                }
                ui.end_row();
                for (f, celdas) in esperado.iter_mut().enumerate() {
                    for b in tabla.fila(f) {
                        ui.label(bit(Valor::de(b)));
                    }
                    for &v in &tabla.filas[f] {
                        ui.label(bit(v).color(c.de(v)));
                    }
                    for (e, &v) in celdas.iter_mut().zip(&tabla.filas[f]) {
                        let mal = e.is_some_and(|b| Valor::de(b) != v);
                        let (texto, color) = match *e {
                            None => ("-", c.tenue),
                            Some(b) => (
                                if b { "1" } else { "0" },
                                if mal { c.error } else { c.tinta },
                            ),
                        };
                        let boton =
                            egui::Button::new(RichText::new(texto).monospace().color(color))
                                .frame(false);
                        if ui.add(boton).on_hover_text("Click: -, 0, 1").clicked() {
                            *e = match *e {
                                None => Some(false),
                                Some(false) => Some(true),
                                Some(true) => None,
                            };
                        }
                    }
                    ui.end_row();
                }
            });

        if crear {
            let n = tabla.entradas.len();
            let productos: Vec<Vec<Producto>> = (0..salidas)
                .map(|k| {
                    let filas_con = |v: Option<bool>| {
                        (0..filas)
                            .filter(|&f| esperado[f][k] == v)
                            .collect::<Vec<_>>()
                    };
                    minimizar(n, &filas_con(Some(true)), &filas_con(None))
                })
                .collect();
            let nombres: Vec<String> = tabla.salidas.iter().map(|s| s.0.clone()).collect();
            self.crear_solucion(&circuito, &tabla.entradas, &nombres, &productos);
        }
    }

    /// Un archivo nuevo con el circuito que da lo esperado
    fn crear_solucion(
        &mut self,
        circuito: &str,
        entradas: &[String],
        salidas: &[String],
        productos: &[Vec<Producto>],
    ) {
        let base = if circuito == "main" {
            "Solucion".to_string()
        } else {
            format!("{circuito}Solucion")
        };
        let libre = (1..100)
            .map(|k| {
                if k == 1 {
                    base.clone()
                } else {
                    format!("{base}{k}")
                }
            })
            .find(|nombre| self.proyecto.crear(nombre).is_ok());
        let Some(nombre) = libre else {
            self.aviso(format!("No se pudo crear el archivo {base}."));
            return;
        };
        self.proyecto.actual_mut().codigo = escribir(&nombre, entradas, salidas, productos);
        self.recompilar_desde_cero();
        self.salida
            .push(format!("Se creó {nombre} desde la tabla de verdad."));
    }
}

/// Más entradas que esto hacen la tabla de verdad demasiado larga para mostrarla
const MAX_ENTRADAS_TABLA: usize = 8;

fn contar_entradas(red: &Netlist) -> usize {
    red.nodos
        .iter()
        .filter(|n| matches!(n.tipo, Tipo::Entrada(..)))
        .count()
}

/// "A=1, B=0": las entradas de la fila `f`
fn combinacion(tabla: &Tabla, f: usize) -> String {
    tabla
        .entradas
        .iter()
        .zip(tabla.fila(f))
        .map(|(e, v)| format!("{e}={}", v as u8))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Compara la tabla del archivo abierto con la del archivo `otro`: `Ok` si hacen lo mismo
fn comparar(proyecto: &Proyecto, tabla: &Tabla, otro: &str) -> Result<String, String> {
    let i = proyecto
        .archivos
        .iter()
        .position(|a| a.nombre == otro)
        .ok_or_else(|| format!("{otro} ya no existe."))?;
    let red = proyecto
        .compilar_archivo(i)
        .map_err(|_| format!("{otro} tiene errores: corrígelos para comparar."))?;
    if contar_entradas(&red) > MAX_ENTRADAS_TABLA {
        return Err(format!(
            "{otro} tiene más de {MAX_ENTRADAS_TABLA} entradas."
        ));
    }
    match logicsm_sim::diferencia(tabla, &Tabla::de(&red)) {
        Ok(None) => Ok(format!("Hace lo mismo que {otro}.")),
        Ok(Some((f, k, aqui, alla))) => Err(format!(
            "No hace lo mismo que {otro}: con {}, {} da {} aquí y {} en {otro}.",
            combinacion(tabla, f),
            tabla.salidas[k].0,
            aqui.simbolo(),
            alla.simbolo()
        )),
        Err(e) => Err(format!("No se puede comparar con {otro}: {e}.")),
    }
}

/// "Advertencia: la entrada `A` no se usa." por cada entrada que nada lee
fn entradas_sin_usar(red: &Netlist) -> Vec<String> {
    let mut leida = vec![false; red.nodos.len()];
    for n in &red.nodos {
        for &e in &n.entradas {
            leida[e] = true;
        }
    }
    red.nodos
        .iter()
        .zip(leida)
        .filter_map(|(n, leida)| match &n.tipo {
            Tipo::Entrada(x, _) if !leida => {
                Some(format!("Advertencia: la entrada `{x}` no se usa."))
            }
            _ => None,
        })
        .collect()
}

/// Guarda como PNG la parte `rect` del próximo cuadro (el lienzo) y manda el resultado
fn capturar(ctx: &egui::Context, rect: Rect, ruta: PathBuf, envio: mpsc::Sender<Exportado>) {
    let (ppp, ctx2) = (ctx.pixels_per_point(), ctx.clone());
    ctx.request_screenshot(move |imagen| {
        let recorte = imagen.region(&rect, Some(ppp));
        let [ancho, alto] = recorte.size.map(|n| n as u32);
        let resultado = image::RgbaImage::from_raw(ancho, alto, recorte.as_raw().to_vec())
            .ok_or_else(|| "la captura salió vacía".to_string())
            .and_then(|img| {
                let mut png = std::io::Cursor::new(Vec::new());
                img.write_to(&mut png, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
                plataforma::escribir(&ruta, png.get_ref()).map_err(|e| e.to_string())
            })
            .map(|()| format!("Exportado: {}", ruta.display()))
            .map_err(|e| format!("No se pudo exportar {}: {e}", ruta.display()));
        let _ = envio.send(resultado);
        ctx2.request_repaint();
    });
    ctx.request_repaint();
}

/// Espacio libre alrededor del dibujo en el SVG
const MARGEN_SVG: f32 = 10.0;

/// Agrega `circuito1 = Circuito(entradas);` en su propia línea antes de la última `}`
/// de `codigo`, con un nombre que el código no use. Devuelve la línea, o `None` si no
/// hay ningún circuito donde ponerla.
fn insertar_declaracion(codigo: &mut String, circuito: &str, entradas: &[&str]) -> Option<String> {
    use logicsm_core::lexer::{Tipo as Token, tokenizar};
    let fin = codigo.rfind('}')?;
    let usados: Vec<String> = tokenizar(codigo)
        .0
        .into_iter()
        .filter_map(|t| match t.tipo {
            Token::Ident(s) => Some(s),
            _ => None,
        })
        .collect();
    // FullAdder → fullAdder1, fullAdder2…
    let mut letras = circuito.chars();
    let base: String = letras
        .next()
        .into_iter()
        .flat_map(char::to_lowercase)
        .chain(letras)
        .collect();
    let nombre = (1..)
        .map(|n| format!("{base}{n}"))
        .find(|n| !usados.contains(n))
        .expect("siempre hay un número libre");
    let linea = format!("{nombre} = {circuito}({});", entradas.join(", "));
    let inicio = codigo[..fin].rfind('\n').map_or(0, |k| k + 1);
    if codigo[inicio..fin].trim().is_empty() {
        codigo.insert_str(inicio, &format!("    {linea}\n"));
    } else {
        codigo.insert_str(fin, &format!("\n    {linea}\n"));
    }
    Some(linea)
}

/// El circuito como SVG: las figuras que egui dibujó en el lienzo, en sus coordenadas.
/// El marco abarca todas (cables incluidos) más `MARGEN_SVG`; `fondo` es el color de atrás.
// ponytail: solo las figuras que usa el lienzo (rect, círculo, línea, trazo, texto de una fuente por fila)
fn svg(figuras: &[Shape], fondo: Color32) -> String {
    let marco = figuras
        .iter()
        .map(Shape::visual_bounding_rect)
        .fold(Rect::NOTHING, Rect::union)
        .expand(MARGEN_SVG);
    use std::fmt::Write;
    fn color(c: Color32) -> String {
        let [r, g, b, a] = c.to_srgba_unmultiplied();
        if a == 0 {
            "none".into()
        } else {
            format!("rgba({r},{g},{b},{:.3})", a as f32 / 255.0)
        }
    }
    fn trazo(c: Color32, ancho: f32) -> String {
        if ancho > 0.0 && c.a() > 0 {
            format!(r#" stroke="{}" stroke-width="{ancho}""#, color(c))
        } else {
            String::new()
        }
    }
    fn escapar(t: &str) -> String {
        t.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
    fn figura(s: &mut String, f: &Shape) {
        let _ = match f {
            Shape::Vec(v) => {
                v.iter().for_each(|f| figura(s, f));
                Ok(())
            }
            Shape::Rect(r) => {
                // El trazo de egui va adentro, afuera o en medio del borde
                let medio = r.stroke.width / 2.0;
                let q = match r.stroke_kind {
                    StrokeKind::Inside => r.rect.shrink(medio),
                    StrokeKind::Outside => r.rect.expand(medio),
                    StrokeKind::Middle => r.rect,
                };
                writeln!(
                    s,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}"{}/>"#,
                    q.left(),
                    q.top(),
                    q.width(),
                    q.height(),
                    r.corner_radius.nw,
                    color(r.fill),
                    trazo(r.stroke.color, r.stroke.width)
                )
            }
            Shape::Circle(c) => writeln!(
                s,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}"{}/>"#,
                c.center.x,
                c.center.y,
                c.radius,
                color(c.fill),
                trazo(c.stroke.color, c.stroke.width)
            ),
            Shape::LineSegment {
                points: [a, b],
                stroke,
            } => writeln!(
                s,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}/>"#,
                a.x,
                a.y,
                b.x,
                b.y,
                trazo(stroke.color, stroke.width)
            ),
            Shape::Path(p) => {
                let puntos: Vec<String> = p
                    .points
                    .iter()
                    .map(|q| format!("{},{}", q.x, q.y))
                    .collect();
                let linea = match p.stroke.color {
                    egui::epaint::ColorMode::Solid(c) => trazo(c, p.stroke.width),
                    _ => String::new(),
                };
                let (etiqueta, relleno) = if p.closed {
                    ("polygon", color(p.fill))
                } else {
                    ("polyline", "none".into())
                };
                writeln!(
                    s,
                    r#"<{etiqueta} points="{}" fill="{relleno}" stroke-linejoin="round"{linea}/>"#,
                    puntos.join(" ")
                )
            }
            Shape::Text(t) => {
                let Some(seccion) = t.galley.job.sections.first() else {
                    return;
                };
                let fuente = &seccion.format.font_id;
                let familia = match fuente.family {
                    egui::FontFamily::Monospace => "monospace",
                    _ => "sans-serif",
                };
                let tinta = t.override_text_color.unwrap_or(
                    if seccion.format.color == Color32::PLACEHOLDER {
                        t.fallback_color
                    } else {
                        seccion.format.color
                    },
                );
                for fila in &t.galley.rows {
                    let r = fila.rect().translate(t.pos.to_vec2());
                    let _ = writeln!(
                        s,
                        r#"<text x="{}" y="{}" font-family="{familia}" font-size="{}" fill="{}" dominant-baseline="central" xml:space="preserve">{}</text>"#,
                        r.left(),
                        r.center().y,
                        fuente.size,
                        color(tinta),
                        escapar(&fila.text())
                    );
                }
                Ok(())
            }
            _ => Ok(()),
        };
    }
    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}" width="{}" height="{}">"#,
        marco.left(),
        marco.top(),
        marco.width(),
        marco.height(),
        marco.width(),
        marco.height()
    );
    s.push('\n');
    let _ = writeln!(
        s,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
        marco.left(),
        marco.top(),
        marco.width(),
        marco.height(),
        color(fondo)
    );
    figuras.iter().for_each(|f| figura(&mut s, f));
    s.push_str("</svg>\n");
    s
}

/// Tabla de verdad en CSV: nombres de entradas y salidas, y una fila de 0 y 1 por combinación
fn tabla_csv(red: &Netlist) -> Result<String, String> {
    if contar_entradas(red) > MAX_ENTRADAS_CSV {
        return Err(format!(
            "tiene más de {MAX_ENTRADAS_CSV} entradas, la tabla sería demasiado grande"
        ));
    }
    let tabla = Tabla::de(red);
    let mut csv = tabla
        .entradas
        .iter()
        .chain(tabla.salidas.iter().map(|(nombre, _)| nombre))
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(",");
    csv.push('\n');
    for (f, salidas) in tabla.filas.iter().enumerate() {
        let bits: Vec<&str> = tabla
            .fila(f)
            .into_iter()
            .map(Valor::de)
            .chain(salidas.iter().copied())
            .map(Valor::simbolo)
            .collect();
        csv.push_str(&bits.join(","));
        csv.push('\n');
    }
    Ok(csv)
}

// ---------- Dibujo del circuito ----------

const ANCHO_COLUMNA: f32 = 130.0;
const ALTO_FILA: f32 = 64.0;

fn tamano(n: &Nodo) -> Vec2 {
    match n.tipo {
        Tipo::Compuerta(_) | Tipo::Memoria { .. } => {
            vec2(46.0, (n.entradas.len() as f32 * 14.0 + 8.0).max(36.0))
        }
        Tipo::Entrada(..) => vec2(30.0, 24.0),
        Tipo::Constante(_) => vec2(22.0, 22.0),
        Tipo::Salida(_) | Tipo::Visor(..) => vec2(24.0, 24.0),
        Tipo::Bus => vec2(10.0, 10.0),
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
        Compuerta::Not | Compuerta::Buffer | Compuerta::Tristate => {
            vec![pos2(l, t), pos2(l + w, cy), pos2(l, b)]
        }
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
                Tipo::Entrada(x, _) => format!("in {x}"),
                Tipo::Salida(x) => format!("out {x}"),
                Tipo::Visor(_, x) => format!("visor {x}"),
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
            match inst.split {
                Some(Split::Separa) => format!("split: separa {} bits", inst.salidas.len()),
                Some(Split::Junta) => format!("split: junta {} bits", inst.entradas.len()),
                None if matches!(visor_de(red, g), Some(Visor::Matrix(..))) => {
                    format!("{}: matrix (click para cambiar a cuadrados o círculos)", inst.nombre)
                }
                None => format!("{}: {}", inst.nombre, inst.circuito),
            }
        }
        Pieza::Nodo(i) => {
            let n = &red.nodos[i];
            match &n.tipo {
                Tipo::Entrada(x, Clase::Interruptor) => {
                    format!("Entrada {x} (click para cambiarla)")
                }
                Tipo::Entrada(x, Clase::Pulsador) => {
                    format!("Pulsador {x} (vale 1 mientras lo mantienes presionado)")
                }
                Tipo::Entrada(x, Clase::Reloj) => {
                    format!("Reloj {x} (lo mueven Ejecutar y Paso a paso)")
                }
                Tipo::Salida(x) => format!("Salida {x}"),
                Tipo::Constante(v) => format!("Constante {}", *v as u8),
                Tipo::Compuerta(g) if n.nombre.is_empty() => g.nombre().to_uppercase(),
                Tipo::Compuerta(g) => format!("{} = {}", n.nombre, g.nombre().to_uppercase()),
                Tipo::Bus => format!("{}: varios tristate", n.nombre),
                Tipo::Visor(v, x) => format!("{} {x}", v.nombre()),
                Tipo::Memoria { bit, .. } => format!("Memoria, bit {bit}"),
            }
        }
    }
}

/// Puntos de la cuadrícula en la parte visible del lienzo
fn puntos(ui: &egui::Ui) {
    const PASO: f32 = 18.0;
    let c = paleta(ui);
    let r = ui.clip_rect();
    // Muy alejado serían demasiados puntos: no se dibujan
    if r.width() * r.height() / (PASO * PASO) > 40_000.0 {
        return;
    }
    let p = ui.painter();
    let mut y = (r.top() / PASO).floor() * PASO + PASO / 2.0;
    while y < r.bottom() {
        let mut x = (r.left() / PASO).floor() * PASO + PASO / 2.0;
        while x < r.right() {
            p.circle_filled(pos2(x, y), 0.9, c.puntos);
            x += PASO;
        }
        y += PASO;
    }
}

/// Lo que pasó al dibujar
struct Dibujo {
    /// Rectángulo que ocupan las piezas
    contenido: Rect,
    /// Empezó a moverse una pieza: posiciones de antes, para deshacer
    antes: Option<Posiciones>,
}

/// Altura del pin `k` de `n` repartidos en el lado de `r`
fn altura_pin(r: Rect, k: usize, n: usize) -> f32 {
    r.top() + r.height() * (k + 1) as f32 / (n + 1) as f32
}

/// Dibuja el circuito en coordenadas del lienzo (dentro de un `Scene`)
fn dibujar(
    ui: &mut egui::Ui,
    red: &Netlist,
    valores: &[Valor],
    entradas: &mut [bool],
    posiciones: &mut Posiciones,
    ondas: &VecDeque<Vec<Valor>>,
    cuadradas: &mut HashSet<String>,
) -> Dibujo {
    let c = paleta(ui);
    let p = ui.painter().clone();
    let mut dibujo = Dibujo {
        contenido: Rect::NOTHING,
        antes: None,
    };

    // Piezas: los nodos sueltos y una caja por instancia
    let mut piezas: Vec<Pieza> = (0..red.nodos.len())
        .filter(|&i| red.nodos[i].instancia.is_none())
        .map(Pieza::Nodo)
        .collect();
    let primera_caja = piezas.len();
    piezas.extend((0..red.instancias.len()).map(Pieza::Caja));
    let total = piezas.len();
    // El visor de cada caja, buscado una sola vez por cuadro
    let mut visores: Vec<Option<Visor>> = vec![None; red.instancias.len()];
    for n in &red.nodos {
        if let (Tipo::Visor(v, _), Some(g)) = (&n.tipo, n.instancia) {
            visores[g].get_or_insert(*v);
        }
    }
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
            if inst.split.is_some() {
                return vec2(8.0, (pines as f32 * 16.0 + 8.0).max(30.0));
            }
            let ancho = (inst.circuito.len() as f32 * 7.0 + 20.0).max(96.0);
            let alto = (pines as f32 * 18.0 + 30.0).max(48.0);
            match visores[g] {
                Some(Visor::Probe) => vec2(200.0, alto),
                // Los LEDs llenan la caja; filas a la izquierda y columnas arriba
                Some(Visor::Matrix(f, c)) => vec2(
                    ETIQUETAS + CELDA * c as f32 + MARGEN,
                    ARRIBA + CELDA * f as f32 + MARGEN,
                ),
                _ => vec2(ancho, alto),
            }
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

    // "Ajustar" encuadra el resultado, así que el origen da igual
    let origen = pos2(70.0, 40.0);
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
        if let Some(&[x, y]) = posiciones.get(&clave) {
            centros[j] = pos2(x, y);
        }
        let r = ui.interact(
            Rect::from_center_size(centros[j], tam(pieza)),
            ui.scope_id().with(&clave),
            Sense::click_and_drag(),
        );
        // Click en una matriz: círculos o cuadrados
        if r.clicked()
            && matches!(pieza, Pieza::Caja(g) if matches!(visores[g], Some(Visor::Matrix(..))))
            && !cuadradas.remove(&clave)
        {
            cuadradas.insert(clave.clone());
        }
        if r.drag_started() {
            dibujo.antes = Some(posiciones.clone());
        }
        if r.dragged() {
            centros[j] += r.drag_delta();
            posiciones.insert(clave, [centros[j].x, centros[j].y]);
        }
        if let Pieza::Nodo(i) = pieza
            && let Tipo::Entrada(_, clase) = red.nodos[i].tipo
        {
            match clase {
                Clase::Interruptor if r.clicked() => entradas[k] = !entradas[k],
                Clase::Pulsador if r.is_pointer_button_down_on() => {
                    entradas[k] = true;
                    ui.ctx().request_repaint();
                }
                _ => {}
            }
            k += 1;
        }
        r.on_hover_text(descripcion(red, pieza))
            .on_hover_cursor(egui::CursorIcon::Grab);
    }

    let caja = |j: usize| Rect::from_center_size(centros[j], tam(piezas[j]));
    let valor = |i: usize| valores.get(i).copied().unwrap_or_default();
    let color = |v: Valor| c.de(v);
    // Relleno de entradas, salidas y LEDs: verde en 1, rojo en X
    let relleno = |v: Valor| match v {
        Valor::Uno => c.uno,
        Valor::X => c.error,
        _ => c.panel,
    };

    // Dónde sale la señal del nodo `f`: de su pieza o del pin de la caja que lo tiene
    let pin_salida = |f: usize| {
        let j = pieza_de[f];
        let r = caja(j);
        match piezas[j] {
            Pieza::Nodo(i) => {
                let burbuja = matches!(red.nodos[i].tipo, Tipo::Compuerta(g) if invertida(g));
                pos2(r.right() + if burbuja { 8.0 } else { 0.0 }, r.center().y)
            }
            // El bus de un split sale por un solo pin, al medio
            Pieza::Caja(g) if red.instancias[g].split == Some(Split::Junta) => r.right_center(),
            Pieza::Caja(g) => {
                let salidas = &red.instancias[g].salidas;
                let k = salidas.iter().position(|s| s.1 == f).unwrap_or(0);
                pos2(r.right(), altura_pin(r, k, salidas.len()))
            }
        }
    };
    // Matriz de la pieza `j`, si lo es: (filas, columnas, modo Individual)
    let matriz = |j: usize| match piezas[j] {
        Pieza::Caja(g) => match visores[g] {
            Some(Visor::Matrix(f, cols)) => {
                Some((f, cols, individual(f, cols, red.instancias[g].entradas.len())))
            }
            _ => None,
        },
        Pieza::Nodo(_) => None,
    };
    // Dónde llega la entrada `k` de la pieza `j`
    let pin_entrada = |j: usize, k: usize| {
        let r = caja(j);
        if matches!(piezas[j], Pieza::Caja(g) if red.instancias[g].split == Some(Split::Separa)) {
            return r.left_center();
        }
        // Como en LogicCircuit: cada fila llega por la izquierda y cada columna por
        // arriba; en modo Individual, los bits de cada fila llegan juntos a su pin
        if let Some((f, cols, individual)) = matriz(j) {
            let rej = rejilla_matriz(r, f, cols);
            let k = if individual { k / cols } else { k };
            return if k < f {
                pos2(r.left(), rej.top() + CELDA * (k as f32 + 0.5))
            } else {
                pos2(rej.left() + CELDA * ((k - f) as f32 + 0.5), r.top())
            };
        }
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

    dibujo.contenido = (0..total).map(caja).fold(Rect::NOTHING, Rect::union);

    // Cables (debajo de los componentes), en ángulo recto
    let abajo = dibujo.contenido.bottom();
    let mut lazos = 0;
    for (j, fs) in fuentes.iter().enumerate() {
        for (k, &fuente) in fs.iter().enumerate() {
            let a = pin_salida(fuente);
            let b = pin_entrada(j, k);
            let puntos = if matriz(j).is_some_and(|(f, _, ind)| !ind && k >= f) {
                // Columna de la matriz: sube por encima de la caja y baja al pin
                let x = (caja(j).left() - 14.0 - 5.0 * k as f32).max(a.x + 6.0);
                let y = b.y - 10.0 - 6.0 * k as f32;
                vec![a, pos2(x, a.y), pos2(x, y), pos2(b.x, y), b]
            } else if b.x > a.x + 12.0 {
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
            Pieza::Caja(g) if red.instancias[g].split.is_some() => {
                // Split: barra con el número de cada bit sobre su pin
                let inst = &red.instancias[g];
                p.rect_filled(r, 2.0, c.tinta);
                let (pines, x, alinear) = match inst.split {
                    Some(Split::Separa) => (&inst.salidas, r.right() + 3.0, Align2::LEFT_BOTTOM),
                    _ => (&inst.entradas, r.left() - 3.0, Align2::RIGHT_BOTTOM),
                };
                for (k, (nombre, _)) in pines.iter().enumerate() {
                    let y = altura_pin(r, k, pines.len());
                    p.text(
                        pos2(x, y - 1.0),
                        alinear,
                        nombre,
                        FontId::monospace(9.0),
                        c.tenue,
                    );
                }
                continue;
            }
            Pieza::Caja(g) => {
                // Caja con el nombre del circuito, la instancia arriba y los pines rotulados
                let inst = &red.instancias[g];
                let visor = visores[g];
                let pequeno = FontId::monospace(10.0);
                p.text(
                    r.center_top() - vec2(0.0, 4.0),
                    Align2::CENTER_BOTTOM,
                    &inst.nombre,
                    FontId::monospace(11.0),
                    c.tenue,
                );
                // La matriz se dibuja como un módulo de hardware, sin la caja de siempre
                if let Some(Visor::Matrix(filas, columnas)) = visor {
                    let luces: Vec<Valor> = inst.entradas.iter().map(|e| valor(e.1)).collect();
                    let cuadrada = cuadradas.contains(&clave(red, pieza));
                    dibujar_matriz(&p, r, filas, columnas, &luces, cuadrada);
                    continue;
                }
                p.rect(r, 6.0, c.panel, trazo, StrokeKind::Inside);
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
                if let Some(visor) = visor {
                    let bits: Vec<usize> = inst.entradas.iter().map(|e| e.1).collect();
                    dibujar_visor(&p, c, r, visor, &bits, &valor, ondas);
                }
                continue;
            }
        };
        let v = valor(i);
        match &red.nodos[i].tipo {
            Tipo::Entrada(nombre, _) => {
                p.rect(r, 5.0, relleno(v), trazo, StrokeKind::Inside);
                texto(r.center(), Align2::CENTER_CENTER, v.simbolo());
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
                p.circle(r.center(), r.width() / 2.0, relleno(v), trazo);
                texto(
                    r.right_center() + vec2(8.0, 0.0),
                    Align2::LEFT_CENTER,
                    nombre,
                );
            }
            // Solo el LED va suelto: los demás visores son cajas
            Tipo::Visor(_, nombre) => {
                let v = red.nodos[i]
                    .entradas
                    .first()
                    .map_or(Valor::X, |&e| valor(e));
                p.circle(r.center(), r.width() / 2.0, relleno(v), trazo);
                p.circle_stroke(r.center(), r.width() / 2.0 - 4.0, Stroke::new(1.0, c.tenue));
                texto(
                    r.right_center() + vec2(8.0, 0.0),
                    Align2::LEFT_CENTER,
                    nombre,
                );
            }
            Tipo::Bus => {
                p.circle_filled(r.center(), r.width() / 2.0, color(v));
            }
            // Siempre va dentro de su caja
            Tipo::Memoria { .. } => {
                p.rect(r, 3.0, c.panel, trazo, StrokeKind::Inside);
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
    dibujo
}

/// Lado de cada LED de una `matrix`
const CELDA: f32 = 22.0;
/// Espacio de los nombres de las filas (izquierda) y las columnas (arriba) de una
/// `matrix`, y borde alrededor de sus LEDs
const ETIQUETAS: f32 = 24.0;
const ARRIBA: f32 = 18.0;
const MARGEN: f32 = 3.0;

/// Dónde van los LEDs dentro de la caja `r` de una `matrix`
fn rejilla_matriz(r: Rect, filas: usize, columnas: usize) -> Rect {
    Rect::from_min_size(
        r.min + vec2(ETIQUETAS, ARRIBA),
        vec2(CELDA * columnas as f32, CELDA * filas as f32),
    )
}

/// Modo Individual de una `matrix` (un bit por LED) según cuántos bits recibe
fn individual(filas: usize, columnas: usize, bits: usize) -> bool {
    bits == filas * columnas && bits != filas + columnas
}

/// Una `matrix` como módulo de hardware: placa verde con pines dorados (filas a la
/// izquierda, columnas arriba) y los LEDs en un bloque negro. Los colores son fijos,
/// iguales en los dos temas. `luces` son sus pines: en modo Selector, primero las
/// filas y después las columnas; en modo Individual, un bit por LED, fila por fila.
fn dibujar_matriz(
    p: &egui::Painter,
    r: Rect,
    filas: usize,
    columnas: usize,
    luces: &[Valor],
    cuadrada: bool,
) {
    let placa = Color32::from_rgb(0x1f, 0x5c, 0x3c);
    let borde_placa = Color32::from_rgb(0x15, 0x42, 0x2b);
    let serigrafia = Color32::from_rgb(0xe8, 0xee, 0xe9);
    let oro = Color32::from_rgb(0xd4, 0xa8, 0x3a);
    let bloque = Color32::from_rgb(0x14, 0x15, 0x15);
    let apagado = Color32::from_rgb(0x2c, 0x2e, 0x2d);
    let encendido = Color32::from_rgb(0xff, 0x4a, 0x3a);
    let brillo = Color32::from_rgba_unmultiplied(0xff, 0x4a, 0x3a, 70);
    let reflejo = Color32::from_rgba_unmultiplied(255, 255, 255, 120);

    let individual = individual(filas, columnas, luces.len());
    let rejilla = rejilla_matriz(r, filas, columnas);

    // Placa con un agujero de montaje en la esquina libre
    p.rect(r, 3.0, placa, Stroke::new(1.5, borde_placa), StrokeKind::Inside);
    p.circle(
        r.left_top() + vec2(ETIQUETAS, ARRIBA) * 0.5,
        3.0,
        borde_placa,
        Stroke::new(1.2, oro),
    );

    // Pines dorados en el borde, donde llegan los cables, y su nombre serigrafiado.
    // En modo Individual solo hay pines de filas (cada uno con los bits de su fila)
    let pines = if individual { filas } else { filas + columnas };
    for k in 0..pines {
        let nombre = if k < filas {
            format!("F{k}")
        } else {
            format!("C{}", k - filas)
        };
        let (pin, texto, alinear) = if k < filas {
            let y = rejilla.top() + CELDA * (k as f32 + 0.5);
            (
                Rect::from_min_size(pos2(r.left(), y - 2.0), vec2(6.0, 4.0)),
                pos2(r.left() + 8.0, y),
                Align2::LEFT_CENTER,
            )
        } else {
            let x = rejilla.left() + CELDA * ((k - filas) as f32 + 0.5);
            (
                Rect::from_min_size(pos2(x - 2.0, r.top()), vec2(4.0, 5.0)),
                pos2(x, r.top() + 6.0),
                Align2::CENTER_TOP,
            )
        };
        p.rect_filled(pin, 1.0, oro);
        p.text(texto, alinear, nombre, FontId::monospace(8.0), serigrafia);
    }

    // Bloque negro de los LEDs. Como en LogicCircuit, solo cuenta el 1 (0, X y Z
    // apagan): en modo Selector se enciende con su fila y su columna en 1; en modo
    // Individual, con su propio bit
    p.rect_filled(rejilla.expand(2.0), 2.0, bloque);
    let uno = |i: usize| luces[i] == Valor::Uno;
    for f in 0..filas {
        for k in 0..columnas {
            let prendido = if individual {
                uno(f * columnas + k)
            } else {
                uno(f) && uno(filas + k)
            };
            let esquina = rejilla.min + vec2(k as f32, f as f32) * CELDA;
            let celda = Rect::from_min_size(esquina, Vec2::splat(CELDA)).shrink(2.5);
            let color = if prendido { encendido } else { apagado };
            if cuadrada {
                if prendido {
                    p.rect_filled(celda.expand(2.0), 4.0, brillo);
                }
                p.rect_filled(celda, 2.0, color);
            } else {
                let radio = celda.width() / 2.0;
                if prendido {
                    p.circle_filled(celda.center(), radio + 2.0, brillo);
                }
                p.circle_filled(celda.center(), radio, color);
            }
            if prendido {
                p.circle_filled(celda.min + Vec2::splat(celda.width() * 0.3), 1.6, reflejo);
            }
        }
    }
}

/// El visor (display, matriz o sonda) que se dibuja en la caja `g`
fn visor_de(red: &Netlist, g: usize) -> Option<Visor> {
    red.nodos.iter().find_map(|n| match n.tipo {
        Tipo::Visor(v, _) if n.instancia == Some(g) => Some(v),
        _ => None,
    })
}

/// El contenido de la caja de un display o una sonda: los segmentos o las ondas.
/// `bits` son los nodos de sus pines, en orden.
fn dibujar_visor(
    p: &egui::Painter,
    c: &Paleta,
    r: Rect,
    visor: Visor,
    bits: &[usize],
    valor: &dyn Fn(usize) -> Valor,
    ondas: &VecDeque<Vec<Valor>>,
) {
    // A la izquierda quedan los nombres de los pines y arriba el del visor
    let area = Rect::from_min_max(
        pos2(r.left() + 24.0, r.top() + 22.0),
        r.right_bottom() - vec2(8.0, 8.0),
    );
    // Encendido verde, apagado gris claro, X y Z con su color
    let luz = |v: Valor| match v {
        Valor::Uno => c.uno,
        Valor::Cero => c.suave,
        _ => c.de(v),
    };
    match visor {
        Visor::Display7 => {
            let alto = area.height().min(area.width() * 1.6);
            let d = Rect::from_center_size(area.center(), vec2(alto * 0.5, alto));
            let (tl, tr, ml, mr, bl, br) = (
                d.left_top(),
                d.right_top(),
                d.left_center(),
                d.right_center(),
                d.left_bottom(),
                d.right_bottom(),
            );
            // a, b, c, d, e, f, g
            let segmentos = [
                (tl, tr),
                (tr, mr),
                (mr, br),
                (bl, br),
                (ml, bl),
                (tl, ml),
                (ml, mr),
            ];
            for (k, (a, b)) in segmentos.into_iter().enumerate() {
                let corte = (b - a).normalized() * 3.5;
                let v = valor(bits[k]);
                p.line_segment([a + corte, b - corte], Stroke::new(5.0, luz(v)));
            }
            p.circle_filled(br + vec2(7.0, -2.0), 3.0, luz(valor(bits[7])));
        }
        // Una línea por señal, la muestra más nueva a la derecha
        Visor::Probe => {
            let ancho = area.width() / MUESTRAS as f32;
            let inicio = MUESTRAS - ondas.len();
            let alto = (r.height() / (bits.len() + 1) as f32 * 0.3).min(6.0);
            for (k, &e) in bits.iter().enumerate() {
                let y = altura_pin(r, k, bits.len());
                let mut antes: Option<f32> = None;
                for (s, muestra) in ondas.iter().enumerate() {
                    let v = muestra.get(e).copied().unwrap_or_default();
                    let x = area.left() + (inicio + s) as f32 * ancho;
                    let altura = match v {
                        Valor::Uno => y - alto,
                        Valor::Cero => y + alto,
                        _ => y,
                    };
                    if let Some(a) = antes.filter(|&a| a != altura) {
                        p.line_segment([pos2(x, a), pos2(x, altura)], Stroke::new(1.0, c.tenue));
                    }
                    p.line_segment(
                        [pos2(x, altura), pos2(x + ancho, altura)],
                        Stroke::new(1.5, c.de(v)),
                    );
                    antes = Some(altura);
                }
            }
        }
        // La matriz se dibuja aparte y el LED va suelto
        Visor::Matrix(..) | Visor::Led => {}
    }
}

/// Pone en `v` todas las entradas de la clase `clase`. Devuelve si alguna estaba en 1.
fn poner(red: &Netlist, entradas: &mut [bool], clase: Clase, v: bool) -> bool {
    let mut habia = false;
    let de_entrada = red.nodos.iter().filter_map(|n| match n.tipo {
        Tipo::Entrada(_, k) => Some(k),
        _ => None,
    });
    for (e, k) in entradas.iter_mut().zip(de_entrada) {
        if k == clase {
            habia |= *e;
            *e = v;
        }
    }
    habia
}

/// Guarda los valores de la red para las sondas: en cada medio ciclo del reloj
/// (`tic`) o cuando cambia una señal que miran
fn muestrear(red: &Netlist, valores: &[Valor], ondas: &mut VecDeque<Vec<Valor>>, tic: bool) {
    let mut sondas = red
        .nodos
        .iter()
        .filter(|n| matches!(n.tipo, Tipo::Visor(Visor::Probe, _)))
        .flat_map(|n| n.entradas.iter())
        .peekable();
    if sondas.peek().is_none() {
        return;
    }
    let cambio = ondas
        .back()
        .is_none_or(|u| sondas.any(|&e| u.get(e) != valores.get(e)));
    if tic || cambio {
        if ondas.len() == MUESTRAS {
            ondas.pop_front();
        }
        ondas.push_back(valores.to_vec());
    }
}

/// "0,5 Hz", "10 Hz"
fn hz(f: f64) -> String {
    format!("{f} Hz").replace('.', ",")
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

enum Lado {
    Izquierda,
    Abajo,
    Derecha,
}

/// Muestra u oculta un panel. Dibuja la ventana con ese panel marcado, como en VS Code:
/// relleno si se ve, solo el borde si está oculto.
fn boton_panel(ui: &mut egui::Ui, visible: bool, lado: Lado, ayuda: &str) -> bool {
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
enum Icono {
    Ejecutar,
    Pausa,
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

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn declarar_antes_de_la_ultima_llave() {
        let mut codigo = "main {\n    in A, B;\n    fullAdder1 = 0;\n}\n".to_string();
        let linea = insertar_declaracion(&mut codigo, "FullAdder", &["A", "B"]);
        assert_eq!(linea.as_deref(), Some("fullAdder2 = FullAdder(A, B);"));
        assert!(codigo.ends_with("    fullAdder2 = FullAdder(A, B);\n}\n"));
        // La llave en la misma línea que otra cosa
        let mut codigo = "main { in A; }".to_string();
        insertar_declaracion(&mut codigo, "Not", &["A"]);
        assert_eq!(codigo, "main { in A; \n    not1 = Not(A);\n}");
        assert_eq!(insertar_declaracion(&mut String::new(), "X", &[]), None);
    }

    #[test]
    fn svg_con_rect_linea_y_circulo() {
        let figuras = [
            Shape::rect_filled(
                Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 5.0)),
                2.0,
                Color32::RED,
            ),
            Shape::line_segment(
                [pos2(0.0, 0.0), pos2(4.0, 4.0)],
                Stroke::new(1.0, Color32::BLACK),
            ),
            Shape::circle_filled(pos2(1.0, 2.0), 3.0, Color32::TRANSPARENT),
        ];
        let s = svg(&figuras, Color32::WHITE);
        // Rect 0..10 × 0..5 y la línea (con medio trazo) desde -0.5, más 10 de margen;
        // el círculo transparente no cuenta
        assert!(s.starts_with(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10.5 -10.5 30.5 25.5""#
        ));
        assert!(s.contains(
            r#"<rect x="0" y="0" width="10" height="5" rx="2" fill="rgba(255,0,0,1.000)"/>"#
        ));
        assert!(s.contains(
            r#"<line x1="0" y1="0" x2="4" y2="4" stroke="rgba(0,0,0,1.000)" stroke-width="1"/>"#
        ));
        assert!(s.contains(r#"<circle cx="1" cy="2" r="3" fill="none"/>"#));
        assert!(s.ends_with("</svg>\n"));
    }
}
