//! Barra superior, menú principal, pantalla de celular, panel Proyecto y bienvenida.

use super::*;

impl Interfaz {
    /// Celular: pestañas abajo y una sola parte en el resto de la pantalla
    pub(super) fn pantalla_movil(&mut self, ui: &mut egui::Ui, c: &Paleta) {
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

    pub(super) fn barra_superior(&mut self, ui: &mut egui::Ui) {
        // En el celular: los menús dentro de uno solo y los botones sin texto
        let movil = self.movil;
        let rotulo = |texto: &'static str| if movil { "" } else { texto };
        ui.horizontal_centered(|ui| {
            ui.image((self.logo.id(), vec2(22.0, 22.0)));
            if movil {
                ui.menu_button("Menú", |ui| {
                    filas_chicas(ui);
                    self.menu(ui);
                });
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
                .height(400.0)
                .show_ui(ui, |ui| {
                    filas_chicas(ui);
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
    pub(super) fn menu(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        // En el celular no hay teclado: los atajos solo ocupan lugar
        let movil = self.movil;
        let atajo = move |a: &'static str| if movil { "" } else { a };
        let opcion = |ui: &mut egui::Ui, texto: &str, a: &'static str| {
            ui.add(egui::Button::new(texto).shortcut_text(atajo(a)))
                .clicked()
        };
        ui.menu_button("Archivo", |ui| {
            filas_chicas(ui);
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
            ui.menu_button("Exportar", |ui| {
                filas_chicas(ui);
                self.exportar(ui);
            });
            ui.separator();
            if !self.sin_proyecto && opcion(ui, "Cerrar proyecto", "") {
                self.pedir(&ctx, Accion::Cerrar);
            }
            if !WEB && opcion(ui, "Salir", "Ctrl+Q") {
                self.pedir(&ctx, Accion::Salir);
            }
        });
        ui.menu_button("Editar", |ui| {
            filas_chicas(ui);
            self.editar(ui);
        });
        ui.menu_button("Ver", |ui| {
            filas_chicas(ui);
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
            filas_chicas(ui);
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
            filas_chicas(ui);
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
    pub(super) fn lista_de_ejemplos(&mut self, ui: &mut egui::Ui) {
        filas_chicas(ui);
        ui.add_enabled(false, egui::Label::new("Se agregan al proyecto:"));
        // Los menús no se deslizan solos: en un celular acostado la lista no cabe
        let alto = ui.ctx().content_rect().height() * 0.6;
        egui::ScrollArea::vertical().max_height(alto).show(ui, |ui| {
            for (titulo, archivos) in ayuda::EJEMPLOS {
                if ui.button(*titulo).clicked() {
                    self.agregar_ejemplo(archivos);
                }
            }
        });
    }

    /// Acciones de Editar: en el menú y, en el celular, en la barra sobre el código
    pub(super) fn editar(&mut self, ui: &mut egui::Ui) {
        let movil = self.movil;
        let opcion = |ui: &mut egui::Ui, texto: &str, a: &str| {
            ui.add(egui::Button::new(texto).shortcut_text(if movil { "" } else { a }))
                .clicked()
        };
        let tecla = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        };
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
            // El texto llega después (el navegador lo lee aparte): lo recoge el editor
            plataforma::pegar(ui.ctx());
        }
        ui.separator();
        if opcion(ui, "Seleccionar todo", "Ctrl+A") {
            self.para_editor = Some(tecla(egui::Key::A));
        }
    }

    pub(super) fn panel_proyecto(&mut self, ui: &mut egui::Ui) {
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
    pub(super) fn bienvenida(&mut self, ui: &mut egui::Ui) {
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
    pub(super) fn campo_nombre(&mut self, ui: &mut egui::Ui) {
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
    pub(super) fn nombre_archivo(&self) -> String {
        let a = self.proyecto.actual();
        let marca = if a.sin_guardar() { " •" } else { "" };
        format!("Código — {}{marca}", a.nombre)
    }
}
