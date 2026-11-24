//! Exportar: ventana de formatos, PNG (captura del lienzo), SVG y CSV.

use super::*;

/// Formatos de exportación que faltan
// ponytail: deshabilitados hasta que exista cada exportador
pub(super) const FORMATOS: [&str; 2] = ["LogicCircuit (.CircuitProject)", "Verilog (.v)"];

/// Mensaje para Salida, o el error para avisar
pub(super) type Exportado = Result<String, String>;

/// Más entradas que esto hacen una tabla de verdad demasiado grande para exportar
pub(super) const MAX_ENTRADAS_CSV: usize = 16;

impl Interfaz {
    /// Menú Archivo → Exportar
    pub(super) fn exportar(&mut self, ui: &mut egui::Ui) {
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
    pub(super) fn ventana_exportar(&mut self, ctx: &egui::Context) {
        if let Some((formato, marcados)) = &mut self.elegir {
            let formato = *formato;
            let mut listo = None;
            let r = egui::Modal::new(egui::Id::unique("exportar")).show(ctx, |ui| {
                ui.set_width(ancho_modal(ui, 340.0));
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
            ui.set_width(ancho_modal(ui, 340.0));
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
    pub(super) fn exportar_a(
        &mut self,
        ctx: &egui::Context,
        formato: Formato,
        indices: Vec<usize>,
    ) {
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

    pub(super) fn escribir(&mut self, formato: Formato, destinos: Vec<(usize, PathBuf)>) {
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
    pub(super) fn preparar_imagen(&mut self) {
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

    pub(super) fn informar(&mut self, resultado: Exportado) {
        match resultado {
            Ok(m) => self.salida.push(m),
            Err(e) => self.aviso(e),
        }
    }
}

/// Guarda como PNG la parte `rect` del próximo cuadro (el lienzo) y manda el resultado
pub(super) fn capturar(
    ctx: &egui::Context,
    rect: Rect,
    ruta: PathBuf,
    envio: mpsc::Sender<Exportado>,
) {
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
pub(super) const MARGEN_SVG: f32 = 10.0;

/// El circuito como SVG: las figuras que egui dibujó en el lienzo, en sus coordenadas.
/// El marco abarca todas (cables incluidos) más `MARGEN_SVG`; `fondo` es el color de atrás.
// ponytail: solo las figuras que usa el lienzo (rect, círculo, línea, trazo, texto de una fuente por fila)
pub(super) fn svg(figuras: &[Shape], fondo: Color32) -> String {
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
pub(super) fn tabla_csv(red: &Netlist) -> Result<String, String> {
    if contar_entradas(red) > MAX_ENTRADAS_CSV {
        return Err(format!(
            "tiene más de {MAX_ENTRADAS_CSV} entradas, la tabla sería demasiado grande"
        ));
    }
    let tabla = Tabla::de(red, false);
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
