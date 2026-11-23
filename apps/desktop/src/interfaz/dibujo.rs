//! Dibujo del circuito en el lienzo: columnas, compuertas, cajas, cables y visores.

use super::*;

pub(super) const ANCHO_COLUMNA: f32 = 130.0;
pub(super) const ALTO_FILA: f32 = 64.0;

pub(super) fn tamano(n: &Nodo) -> Vec2 {
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

pub(super) fn invertida(c: Compuerta) -> bool {
    matches!(
        c,
        Compuerta::Not | Compuerta::Nand | Compuerta::Nor | Compuerta::Xnor
    )
}

/// Columna de cada pieza, al estilo de una red neuronal: lo que no tiene entradas a
/// la izquierda, cada pieza una columna después de la más lejana que la alimenta y
/// las salidas al final. `fuentes[i]` son las piezas que alimentan a `i`.
// ponytail: dentro de cada columna van en orden de aparición, sin minimizar cruces
pub(super) fn columnas(fuentes: &[Vec<usize>], salida: &[bool]) -> Vec<usize> {
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
pub(super) fn curva(a: Pos2, control: Pos2, b: Pos2) -> impl Iterator<Item = Pos2> {
    (0..=12).map(move |i| {
        let s = i as f32 / 12.0;
        let u = 1.0 - s;
        (a.to_vec2() * u * u + control.to_vec2() * 2.0 * u * s + b.to_vec2() * s * s).to_pos2()
    })
}

/// Contorno de la compuerta dentro de `r` (sin la burbuja de negación)
pub(super) fn contorno(c: Compuerta, r: Rect) -> Vec<Pos2> {
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
pub(super) enum Pieza {
    Nodo(usize),
    Caja(usize),
}

/// Para recordar dónde la dejó el usuario aunque el código cambie
pub(super) fn clave(red: &Netlist, pieza: Pieza) -> String {
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
pub(super) fn descripcion(red: &Netlist, pieza: Pieza) -> String {
    match pieza {
        Pieza::Caja(g) => {
            let inst = &red.instancias[g];
            match inst.split {
                Some(Split::Separa) => format!("split: separa {} bits", inst.salidas.len()),
                Some(Split::Junta) => format!("split: junta {} bits", inst.entradas.len()),
                None if matches!(visor_de(red, g), Some(Visor::Matrix(..))) => {
                    format!(
                        "{}: matrix (click para cambiar a cuadrados o círculos)",
                        inst.nombre
                    )
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
pub(super) fn puntos(ui: &egui::Ui) {
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
pub(super) struct Dibujo {
    /// Rectángulo que ocupan las piezas
    pub(super) contenido: Rect,
    /// Empezó a moverse una pieza: posiciones de antes, para deshacer
    pub(super) antes: Option<Posiciones>,
}

/// Altura del pin `k` de `n` repartidos en el lado de `r`
pub(super) fn altura_pin(r: Rect, k: usize, n: usize) -> f32 {
    r.top() + r.height() * (k + 1) as f32 / (n + 1) as f32
}

/// Dibuja el circuito en coordenadas del lienzo (dentro de un `Scene`)
pub(super) fn dibujar(
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
            Some(Visor::Matrix(f, cols)) => Some((
                f,
                cols,
                individual(f, cols, red.instancias[g].entradas.len()),
            )),
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
pub(super) const CELDA: f32 = 22.0;
/// Espacio de los nombres de las filas (izquierda) y las columnas (arriba) de una
/// `matrix`, y borde alrededor de sus LEDs
pub(super) const ETIQUETAS: f32 = 24.0;
pub(super) const ARRIBA: f32 = 18.0;
pub(super) const MARGEN: f32 = 3.0;

/// Dónde van los LEDs dentro de la caja `r` de una `matrix`
pub(super) fn rejilla_matriz(r: Rect, filas: usize, columnas: usize) -> Rect {
    Rect::from_min_size(
        r.min + vec2(ETIQUETAS, ARRIBA),
        vec2(CELDA * columnas as f32, CELDA * filas as f32),
    )
}

/// Modo Individual de una `matrix` (un bit por LED) según cuántos bits recibe
pub(super) fn individual(filas: usize, columnas: usize, bits: usize) -> bool {
    bits == filas * columnas && bits != filas + columnas
}

/// Una `matrix` como módulo de hardware: placa verde con pines dorados (filas a la
/// izquierda, columnas arriba) y los LEDs en un bloque negro. Los colores son fijos,
/// iguales en los dos temas. `luces` son sus pines: en modo Selector, primero las
/// filas y después las columnas; en modo Individual, un bit por LED, fila por fila.
pub(super) fn dibujar_matriz(
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
    p.rect(
        r,
        3.0,
        placa,
        Stroke::new(1.5, borde_placa),
        StrokeKind::Inside,
    );
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
pub(super) fn visor_de(red: &Netlist, g: usize) -> Option<Visor> {
    red.nodos.iter().find_map(|n| match n.tipo {
        Tipo::Visor(v, _) if n.instancia == Some(g) => Some(v),
        _ => None,
    })
}

/// El contenido de la caja de un display o una sonda: los segmentos o las ondas.
/// `bits` son los nodos de sus pines, en orden.
pub(super) fn dibujar_visor(
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
pub(super) fn poner(red: &Netlist, entradas: &mut [bool], clase: Clase, v: bool) -> bool {
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
pub(super) fn muestrear(
    red: &Netlist,
    valores: &[Valor],
    ondas: &mut VecDeque<Vec<Valor>>,
    tic: bool,
) {
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
