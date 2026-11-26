//! Partes centrales: entradas y salidas, editor de código, lienzo y pestañas inferiores.

use super::*;

impl Interfaz {
    /// Interruptores de las entradas y valor de las salidas
    pub(super) fn senales(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn editor(&mut self, ui: &mut egui::Ui) {
        cabecera(ui, &self.nombre_archivo());
        // Un id por archivo: cada uno tiene su propio historial de deshacer
        let id = egui::Id::unique(("editor", self.proyecto.actual().nombre.as_str()));
        // En el celular no hay teclado ni click derecho: Editar va a la vista, sobre el código.
        // Con "Seleccionar" el dedo marca texto en vez de desplazar.
        if self.movil {
            ui.horizontal_wrapped(|ui| {
                filas_chicas(ui);
                ui.toggle_value(&mut self.seleccionar, "Seleccionar");
                self.editar(ui);
            });
            ui.separator();
        }
        match plataforma::pegado(ui.ctx()) {
            Some(Ok(texto)) => self.para_editor = Some(egui::Event::Paste(texto)),
            Some(Err(e)) => self.aviso(e),
            None => {}
        }
        // Con el dedo, egui solo deja seleccionar arrastrando si el editor tiene el foco
        if self.movil && self.seleccionar {
            ui.memory_mut(|m| m.request_focus(id));
        }
        // Lo que pidió el menú Editar: se le da el foco al editor y se le pasa el evento
        if let Some(evento) = self.para_editor.take() {
            // Al tocar el botón el editor perdió el foco y egui dejó la selección en solo
            // el cursor (Copiar no copiaba nada): se le devuelve la última que tuvo
            if let Some((de, rango)) = self.seleccion
                && de == id
            {
                let mut estado =
                    egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
                estado.cursor.set_char_range(Some(rango));
                estado.store(ui.ctx(), id);
            }
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
        // En el celular, arrastrar el dedo desplaza el código salvo con "Seleccionar"
        let arrastrando = self.movil
            && !self.seleccionar
            && ui.input(|i| i.any_touches() && i.pointer.is_decidedly_dragging());
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
                    // Solo hay rango mientras tiene el foco: se guarda para el menú Editar
                    if let Some(rango) = salida.cursor_range {
                        self.seleccion = Some((id, rango));
                    }
                    if salida.response.changed() {
                        self.ultimo_lienzo = false;
                        self.compilar_en = Some(ui.input(|i| i.time) + ESPERA_COMPILAR);
                        ui.ctx()
                            .request_repaint_after(Duration::from_secs_f64(ESPERA_COMPILAR));
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
    pub(super) fn ir_al_problema(&mut self, archivo: &str, byte: usize) {
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

    pub(super) fn lienzo(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn pestanas(&mut self, ui: &mut egui::Ui) {
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
        } else if self.estado.valores.contains(&Valor::X) {
            // Un lazo que arranca en X (flip-flop, oscilador) se queda en X y parece estable
            problemas.push(Problema::sin_lugar(
                "Hay señales en X (sin valor): un lazo de realimentación sin valor inicial o \
                 tristate en conflicto. Reiniciar pone todo en 0."
                    .into(),
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
