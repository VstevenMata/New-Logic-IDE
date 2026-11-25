//! Pestaña Tabla de verdad: verificar con lo esperado, crear circuito y comparar.

use super::*;

impl Interfaz {
    /// Todas las combinaciones de entradas y lo que vale cada salida, con la columna
    /// "esperado" de cada salida: verifica el circuito, crea uno desde lo esperado y lo
    /// compara con otro archivo
    pub(super) fn tabla_de_verdad(&mut self, ui: &mut egui::Ui) {
        let c = paleta(ui);
        let Some(red) = &self.red else {
            vacio(ui, "Escribe un circuito para ver su tabla de verdad.");
            return;
        };
        if contar_entradas(red) > MAX_ENTRADAS_TABLA {
            vacio(ui, "Con más de 8 entradas la tabla sería demasiado larga.");
            return;
        }
        // Simular las 2^n filas en cada cuadro trababa la app con circuitos grandes
        let tabla = match &self.tabla {
            Some((desde_reset, tabla)) if *desde_reset == self.desde_reset => tabla.clone(),
            _ => {
                let tabla = Tabla::de(red, self.desde_reset);
                self.tabla = Some((self.desde_reset, tabla.clone()));
                tabla
            }
        };
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
        ui.horizontal_wrapped(|ui| {
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
            // Solo hace falta si hay memoria: sin Reset una fila arranca en X
            if self.desde_reset || tabla.filas.iter().flatten().any(|&v| v == Valor::X) {
                ui.separator();
                ui.checkbox(&mut self.desde_reset, "Partir desde Reset")
                    .on_hover_text("Cada fila arranca con todo en 0, como después del botón Reset");
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
            // Compila el otro archivo y simula su tabla: una vez, no en cada cuadro
            let resultado = match &self.comparacion {
                Some((desde_reset, archivo, r))
                    if *desde_reset == self.desde_reset && archivo == otro =>
                {
                    r.clone()
                }
                _ => {
                    let r = comparar(&self.proyecto, &tabla, otro, self.desde_reset);
                    self.comparacion = Some((self.desde_reset, otro.clone(), r.clone()));
                    r
                }
            };
            let (texto, color) = match resultado {
                Ok(texto) => (texto, c.uno),
                Err(texto) => (texto, c.error),
            };
            ui.label(RichText::new(texto).size(12.0).color(color));
        }
        ui.add_space(6.0);

        let bit = |v: Valor| RichText::new(v.simbolo()).monospace();
        egui::ScrollArea::horizontal().show(ui, |ui| {
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
    pub(super) fn crear_solucion(
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
pub(super) const MAX_ENTRADAS_TABLA: usize = 8;

pub(super) fn contar_entradas(red: &Netlist) -> usize {
    red.nodos
        .iter()
        .filter(|n| matches!(n.tipo, Tipo::Entrada(..)))
        .count()
}

/// "A=1, B=0": las entradas de la fila `f`
pub(super) fn combinacion(tabla: &Tabla, f: usize) -> String {
    tabla
        .entradas
        .iter()
        .zip(tabla.fila(f))
        .map(|(e, v)| format!("{e}={}", v as u8))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Compara la tabla del archivo abierto con la del archivo `otro`: `Ok` si hacen lo mismo
pub(super) fn comparar(
    proyecto: &Proyecto,
    tabla: &Tabla,
    otro: &str,
    desde_reset: bool,
) -> Result<String, String> {
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
    match logicsm_sim::diferencia(tabla, &Tabla::de(&red, desde_reset)) {
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
pub(super) fn entradas_sin_usar(red: &Netlist) -> Vec<String> {
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
