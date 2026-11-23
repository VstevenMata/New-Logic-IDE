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

mod dibujo;
mod exportar;
mod tabla;
mod tema;
use dibujo::*;
use exportar::*;
use tabla::*;
pub use tema::estilo;
use tema::*;

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

/// En pantallas angostas (celular) se ve una parte a la vez, elegida abajo
#[derive(Clone, Copy, PartialEq)]
enum Vista {
    Proyecto,
    Codigo,
    Circuito,
    Senales,
    Abajo,
}

impl Vista {
    const TODAS: [(Vista, &str); 5] = [
        (Vista::Proyecto, "Proyecto"),
        (Vista::Codigo, "Código"),
        (Vista::Circuito, "Circuito"),
        (Vista::Senales, "Entradas"),
        (Vista::Abajo, "Salida"),
    ];
}

/// Más angosto que esto (en puntos) se usa la pantalla de celular
const ANCHO_MOVIL: f32 = 1024.0;

/// Ancho de una ventana emergente: `ancho`, o menos si la pantalla no alcanza
fn ancho_modal(ui: &egui::Ui, ancho: f32) -> f32 {
    ancho.min(ui.ctx().content_rect().width() - 48.0)
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
    /// Pantalla angosta: una parte a la vez (`parte`) en lugar de paneles
    movil: bool,
    parte: Vista,
    /// Cuánto está desplazado el código (para moverlo con el dedo en el celular)
    desplazado: f32,
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
            movil: false,
            parte: Vista::Codigo,
            desplazado: 0.0,
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
                ui.set_width(ancho_modal(ui, 340.0));
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
            ui.set_width(ancho_modal(ui, 340.0));
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
            ui.set_width(ancho_modal(ui, 560.0));
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
            let alto = ui.ctx().content_rect().height() - 140.0;
            egui::ScrollArea::vertical()
                .max_height(alto.clamp(160.0, 480.0))
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
        let movil = ui.available_width() < ANCHO_MOVIL;
        if movil != self.movil {
            // Con el dedo hacen falta botones más altos (en egui miden 20 por defecto)
            ctx.all_styles_mut(|s| s.spacing.interact_size.y = if movil { 40.0 } else { 20.0 });
        }
        self.movil = movil;
        egui::Panel::top("barra")
            .exact_size(48.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(c.panel)
                    .inner_margin(Margin::symmetric(if self.movil { 8 } else { 14 }, 0)),
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
        if self.movil {
            self.pantalla_movil(ui, c);
            self.ventanas(&ctx);
            self.confirmacion(&ctx);
            self.ventana_exportar(&ctx);
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

impl Interfaz {
    /// Celular: pestañas abajo y una sola parte en el resto de la pantalla
    fn pantalla_movil(&mut self, ui: &mut egui::Ui, c: &Paleta) {
        egui::Panel::bottom("vistas")
            .exact_size(52.0)
            .resizable(false)
            .frame(Frame::new().fill(c.panel))
            .show(ui, |ui| {
                ui.columns(Vista::TODAS.len(), |columnas| {
                    for (ui, (vista, texto)) in columnas.iter_mut().zip(Vista::TODAS) {
                        let elegida = self.parte == vista;
                        let color = if elegida { c.tinta } else { c.tenue };
                        let texto = RichText::new(texto).size(12.5).color(color);
                        let r = ui.add_sized(
                            vec2(ui.available_width(), 50.0),
                            // Muy angosto: "Proy…" en vez de partir la palabra en dos líneas
                            egui::Button::new(if elegida { texto.strong() } else { texto })
                                .frame(false)
                                .truncate(),
                        );
                        if elegida {
                            let linea = Rect::from_min_size(
                                r.rect.left_top() + vec2(8.0, 0.0),
                                vec2(r.rect.width() - 16.0, 2.5),
                            );
                            ui.painter().rect_filled(linea, 1.0, c.salvia_borde);
                        }
                        if r.clicked() {
                            self.parte = vista;
                        }
                    }
                });
            });
        // Exportar PNG captura el lienzo: tiene que estar a la vista
        let vista = if self.cola_png.is_empty() {
            self.parte
        } else {
            Vista::Circuito
        };
        let (fondo, margen) = match vista {
            Vista::Codigo => (c.codigo, Margin::ZERO),
            Vista::Circuito => (c.fondo, Margin::ZERO),
            Vista::Abajo => (c.panel, Margin::ZERO),
            Vista::Proyecto | Vista::Senales => (c.panel, Margin::symmetric(14, 15)),
        };
        let zona = egui::CentralPanel::default()
            .frame(Frame::new().fill(fondo).inner_margin(margen))
            .show(ui, |ui| match vista {
                Vista::Proyecto => self.panel_proyecto(ui),
                Vista::Codigo => self.editor(ui),
                Vista::Circuito => self.lienzo(ui),
                Vista::Senales => self.senales(ui),
                Vista::Abajo => self.pestanas(ui),
            })
            .response
            .rect;
        let ctx = ui.ctx().clone();
        self.soltar_componente(&ctx, zona);
    }

    fn barra_superior(&mut self, ui: &mut egui::Ui) {
        // En el celular: los menús dentro de uno solo y los botones sin texto
        let movil = self.movil;
        let rotulo = |texto: &'static str| if movil { "" } else { texto };
        ui.horizontal_centered(|ui| {
            ui.image((self.logo.id(), vec2(22.0, 22.0)));
            if movil {
                ui.menu_button("Menú", |ui| self.menu(ui));
            } else {
                ui.label(RichText::new("LogicSM").size(14.0).strong());
                ui.add_space(6.0);
                self.menu(ui);
            }
            ui.add_space(if movil { 2.0 } else { 6.0 });
            ui.separator();
            ui.add_space(if movil { 2.0 } else { 6.0 });
            let hay = self.hay_reloj();
            let (texto, icono) = if self.corriendo {
                (rotulo("Pausa"), Icono::Pausa)
            } else {
                (rotulo("Ejecutar"), Icono::Ejecutar)
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
                .add_enabled_ui(hay, |ui| {
                    boton(ui, rotulo("Paso a paso"), Icono::Paso, false)
                })
                .inner
                .on_hover_text(paso)
                .on_disabled_hover_text(paso)
                .clicked()
            {
                self.paso();
            }
            egui::ComboBox::from_id_salt("frecuencia")
                .width(if movil { 58.0 } else { 70.0 })
                .selected_text(hz(self.frecuencia))
                .show_ui(ui, |ui| {
                    for f in FRECUENCIAS {
                        ui.selectable_value(&mut self.frecuencia, f, hz(f));
                    }
                })
                .response
                .on_hover_text("Frecuencia del reloj");
            if boton(ui, rotulo("Reiniciar"), Icono::Reiniciar, false)
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
                if boton(ui, rotulo(texto), icono, false).clicked() {
                    ui.ctx().set_theme(otro);
                }
                // En el celular no hay paneles: las partes se eligen abajo
                if movil {
                    return;
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
        // En el celular no hay teclado: los atajos solo ocupan lugar
        let movil = self.movil;
        let atajo = move |a: &'static str| if movil { "" } else { a };
        let opcion = |ui: &mut egui::Ui, texto: &str, a: &'static str| {
            ui.add(egui::Button::new(texto).shortcut_text(atajo(a)))
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
                self.parte = Vista::Proyecto;
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
                .add_enabled(hay, egui::Button::new(texto).shortcut_text(atajo("F5")))
                .on_disabled_hover_text(SIN_RELOJ)
                .clicked()
            {
                self.ejecutar(&ctx);
            }
            if ui
                .add_enabled(
                    hay,
                    egui::Button::new("Paso a paso").shortcut_text(atajo("F10")),
                )
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
                self.parte = Vista::Abajo;
            }
            if opcion(ui, "Ondas", "") {
                self.pestana = Pestana::Ondas;
                self.ver_inferior = true;
                self.parte = Vista::Abajo;
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
                let mas = if self.movil {
                    ui.button("+")
                } else {
                    ui.small_button("+")
                };
                if mas.on_hover_text("Nuevo archivo").clicked() {
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
        let movil = self.movil;
        let opcion = |ui: &mut egui::Ui, texto: &str, atajo: &str| {
            ui.add(
                egui::Button::new(RichText::new(texto).size(13.0))
                    .shortcut_text(if movil { "" } else { atajo })
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
                if !movil {
                    ui.add_space(26.0);
                    vacio(ui, "También puedes arrastrar un archivo .SA a la ventana.");
                }
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
        // En el celular, arrastrar el dedo desplaza el código; si no, el editor selecciona texto
        let arrastrando =
            self.movil && ui.input(|i| i.any_touches() && i.pointer.is_decidedly_dragging());
        if arrastrando {
            let dy = ui.input(|i| i.pointer.delta().y);
            area = area.vertical_scroll_offset((self.desplazado - dy).max(0.0));
        }
        let c = paleta(ui);
        let fuente = egui::TextStyle::Monospace.resolve(ui.style());
        // ponytail: colorea todo el archivo en cada cuadro; guardar el resultado si un archivo largo se pone lento
        let mut colores = |ui: &egui::Ui, texto: &dyn egui::TextBuffer, ancho: f32| {
            let mut trabajo = colorear(texto.as_str(), &fuente, c);
            trabajo.wrap.max_width = ancho;
            ui.painter().layout_job(trabajo)
        };
        self.desplazado = area
            .show(ui, |ui| {
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
                        .interactive(!arrastrando)
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
            })
            .state
            .clamped_offset()
            .y;
    }

    /// Click en Problemas: abre ese archivo con el cursor donde está el error
    fn ir_al_problema(&mut self, archivo: &str, byte: usize) {
        self.parte = Vista::Codigo;
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
            // Más a los lados: ahí van los nombres de entradas y salidas, fuera de las cajas
            let r = dibujo.contenido.expand2(vec2(80.0, 50.0));
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
            (
                Pestana::Tabla,
                if self.movil {
                    "Tabla"
                } else {
                    "Tabla de verdad"
                },
            ),
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
}

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
