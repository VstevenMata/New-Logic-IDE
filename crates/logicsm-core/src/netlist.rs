//! Elaboración: AST → netlist (nodos conectados, sin nombres por resolver).
//!
//! Los circuitos propios se aplanan: cada instancia copia las compuertas de su
//! circuito en un ámbito de nombres propio.

use crate::parser::{Circuito, Expr, Nombre};
use crate::{Compuerta, Error};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tipo {
    Entrada(String),
    Salida(String),
    Constante(bool),
    Compuerta(Compuerta),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nodo {
    pub tipo: Tipo,
    /// Nodos que alimentan sus entradas, en orden.
    pub entradas: Vec<usize>,
    /// La señal que da (`X` en `X = and(A, B);`, o el nombre de la entrada o
    /// salida); vacío en las compuertas anidadas y en las constantes sueltas.
    pub nombre: String,
    /// Instancia del circuito principal que lo contiene (en `Netlist::instancias`)
    pub instancia: Option<usize>,
}

/// `m1 = MedioSumador(A, B);` en el circuito principal, para dibujarlo como caja
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instancia {
    pub nombre: String,
    pub circuito: String,
    /// Cada entrada con el nodo que la alimenta
    pub entradas: Vec<(String, usize)>,
    /// Cada salida con el nodo que la da
    pub salidas: Vec<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Netlist {
    pub nombre: String,
    pub nodos: Vec<Nodo>,
    pub instancias: Vec<Instancia>,
}

/// De dónde sale una señal mientras se elabora. Los nombres se resuelven al
/// final, así el orden no importa y la realimentación funciona.
#[derive(Clone, Copy)]
enum Ref<'a> {
    Nodo(usize),
    /// Un nombre dentro de un ámbito
    Senal(usize, &'a Nombre),
    /// `m.Suma` dentro de un ámbito
    Miembro(usize, &'a Nombre, &'a Nombre),
}

/// Los nombres de una copia de un circuito (el principal o una instancia)
struct Ambito<'a> {
    circuito: &'a Circuito,
    senales: HashMap<&'a str, Ref<'a>>,
    /// Nombre de la instancia → su ámbito
    instancias: HashMap<&'a str, usize>,
}

struct Elaborador<'a> {
    circuitos: HashMap<&'a str, &'a Circuito>,
    ambitos: Vec<Ambito<'a>>,
    nodos: Vec<Nodo>,
    /// Paralelo a `nodos`: de dónde sale cada una de sus entradas
    pendientes: Vec<Vec<Ref<'a>>>,
    errores: Vec<Error>,
    /// Circuitos que se están copiando, para detectar uno que se contiene a sí mismo
    pila: Vec<&'a str>,
    /// Instancia del principal que se está copiando
    actual: Option<usize>,
    /// Instancias del principal: nombre, circuito y sus entradas y salidas sin resolver
    instancias: Vec<(&'a Nombre, &'a Circuito, Vec<Ref<'a>>)>,
}

fn error(mensaje: String, n: &Nombre) -> Error {
    Error {
        mensaje,
        rango: n.rango.clone(),
    }
}

/// Aplana `principal` usando los demás circuitos del proyecto.
pub fn elaborar(principal: &Circuito, circuitos: &[Circuito]) -> Result<Netlist, Vec<Error>> {
    let mut e = Elaborador {
        circuitos: HashMap::new(),
        ambitos: Vec::new(),
        nodos: Vec::new(),
        pendientes: Vec::new(),
        errores: Vec::new(),
        pila: Vec::new(),
        actual: None,
        instancias: Vec::new(),
    };
    for c in circuitos {
        if e.circuitos.insert(&c.nombre.texto, c).is_some() {
            e.error(
                format!("el circuito `{}` está definido dos veces", c.nombre.texto),
                &c.nombre,
            );
        }
    }

    let raiz = e.ambito(principal, None);
    for n in &principal.salidas {
        if e.ambitos[raiz].senales.contains_key(n.texto.as_str()) {
            e.nodo(Tipo::Salida(n.texto.clone()), vec![Ref::Senal(raiz, n)]);
        }
    }

    // Aquí se cierran los lazos de realimentación (Q = nor(R, Qn); Qn = nor(S, Q);)
    let pendientes = std::mem::take(&mut e.pendientes);
    for (i, fuentes) in pendientes.into_iter().enumerate() {
        for f in fuentes {
            match e.resolver(f) {
                Ok(id) => e.nodos[i].entradas.push(id),
                Err(err) => e.agregar(err),
            }
        }
    }

    let mut instancias = Vec::new();
    for (nombre, c, fuentes) in std::mem::take(&mut e.instancias) {
        let mut entradas = Vec::new();
        for (n, f) in c.entradas.iter().zip(fuentes) {
            match e.resolver(f) {
                Ok(id) => entradas.push((n.texto.clone(), id)),
                Err(err) => e.agregar(err),
            }
        }
        // Una salida sin valor ya tiene su error ("nunca recibe un valor")
        let hijo = e.ambitos[raiz].instancias[nombre.texto.as_str()];
        let salidas = c
            .salidas
            .iter()
            .filter_map(|s| Some((s.texto.clone(), e.resolver(Ref::Senal(hijo, s)).ok()?)))
            .collect();
        instancias.push(Instancia {
            nombre: nombre.texto.clone(),
            circuito: c.nombre.texto.clone(),
            entradas,
            salidas,
        });
    }

    if e.errores.is_empty() {
        Ok(Netlist {
            nombre: principal.nombre.texto.clone(),
            nodos: e.nodos,
            instancias,
        })
    } else {
        Err(e.errores)
    }
}

impl<'a> Elaborador<'a> {
    /// Un circuito usado varias veces repetiría sus errores: se guardan una sola vez.
    fn agregar(&mut self, err: Error) {
        if !self.errores.contains(&err) {
            self.errores.push(err);
        }
    }

    fn error(&mut self, mensaje: String, n: &Nombre) {
        self.agregar(error(mensaje, n));
    }

    fn nodo(&mut self, tipo: Tipo, fuentes: Vec<Ref<'a>>) -> usize {
        let nombre = match &tipo {
            Tipo::Entrada(n) | Tipo::Salida(n) => n.clone(),
            _ => String::new(),
        };
        self.nodos.push(Nodo {
            tipo,
            entradas: Vec::new(),
            nombre,
            instancia: self.actual,
        });
        self.pendientes.push(fuentes);
        self.nodos.len() - 1
    }

    /// Crea una copia de `c`. `entradas` son las señales que le llegan; `None` es
    /// el circuito principal, cuyas entradas son interruptores.
    fn ambito(&mut self, c: &'a Circuito, entradas: Option<Vec<Ref<'a>>>) -> usize {
        let id = self.ambitos.len();
        self.ambitos.push(Ambito {
            circuito: c,
            senales: HashMap::new(),
            instancias: HashMap::new(),
        });
        self.pila.push(&c.nombre.texto);

        for (k, n) in c.entradas.iter().enumerate() {
            if self.ambitos[id].senales.contains_key(n.texto.as_str()) {
                self.error(format!("`{}` está declarada dos veces", n.texto), n);
                continue;
            }
            let r = match &entradas {
                Some(refs) => refs[k],
                None => Ref::Nodo(self.nodo(Tipo::Entrada(n.texto.clone()), Vec::new())),
            };
            self.ambitos[id].senales.insert(&n.texto, r);
        }

        for (destino, valor) in &c.asignaciones {
            let a = &self.ambitos[id];
            let mensaje = if c.entradas.iter().any(|e| e.texto == destino.texto) {
                "es una entrada: no se le puede asignar un valor"
            } else if a.senales.contains_key(destino.texto.as_str())
                || a.instancias.contains_key(destino.texto.as_str())
            {
                "ya tiene un valor: cada señal se asigna una sola vez"
            } else {
                match valor {
                    Expr::Instancia(circuito, args) => {
                        if let Some(hijo) = self.instanciar(id, destino, circuito, args) {
                            self.ambitos[id].instancias.insert(&destino.texto, hijo);
                        }
                    }
                    _ => {
                        let antes = self.nodos.len();
                        let r = self.construir(id, valor);
                        if let Ref::Nodo(i) = r
                            && i >= antes
                        {
                            self.nodos[i].nombre = destino.texto.clone();
                        }
                        self.ambitos[id].senales.insert(&destino.texto, r);
                    }
                }
                continue;
            };
            self.error(format!("`{}` {mensaje}", destino.texto), destino);
        }

        for n in &c.salidas {
            if !self.ambitos[id].senales.contains_key(n.texto.as_str()) {
                self.error(format!("la salida `{}` nunca recibe un valor", n.texto), n);
            }
        }
        self.pila.pop();
        id
    }

    /// `m = MedioSumador(A, B);` dentro del ámbito `padre`
    fn instanciar(
        &mut self,
        padre: usize,
        destino: &'a Nombre,
        circuito: &'a Nombre,
        args: &'a [Expr],
    ) -> Option<usize> {
        let Some(&c) = self.circuitos.get(circuito.texto.as_str()) else {
            self.error(
                format!("`{}` no es un circuito del proyecto", circuito.texto),
                circuito,
            );
            return None;
        };
        if self.pila.contains(&c.nombre.texto.as_str()) {
            self.error(
                format!("`{}` se usa dentro de sí mismo", circuito.texto),
                circuito,
            );
            return None;
        }
        if args.len() != c.entradas.len() {
            let plural = if c.entradas.len() == 1 {
                "entrada"
            } else {
                "entradas"
            };
            self.error(
                format!(
                    "`{}` lleva {} {plural}, pero tiene {}",
                    circuito.texto,
                    c.entradas.len(),
                    args.len()
                ),
                circuito,
            );
            return None;
        }
        let entradas: Vec<Ref> = args.iter().map(|a| self.construir(padre, a)).collect();
        // Lo que se copia de una instancia del principal (con sus instancias
        // internas) se dibuja dentro de su caja
        let arriba = padre == 0;
        if arriba {
            self.actual = Some(self.instancias.len());
            self.instancias.push((destino, c, entradas.clone()));
        }
        let hijo = self.ambito(c, Some(entradas));
        if arriba {
            self.actual = None;
        }
        Some(hijo)
    }

    /// Crea los nodos de una expresión (las compuertas anidadas también).
    fn construir(&mut self, ambito: usize, e: &'a Expr) -> Ref<'a> {
        match e {
            Expr::Senal(n) => Ref::Senal(ambito, n),
            Expr::Miembro(m, salida) => Ref::Miembro(ambito, m, salida),
            Expr::Constante(v) => Ref::Nodo(self.nodo(Tipo::Constante(*v), Vec::new())),
            Expr::Compuerta(c, args) => {
                let fuentes = args.iter().map(|a| self.construir(ambito, a)).collect();
                Ref::Nodo(self.nodo(Tipo::Compuerta(*c), fuentes))
            }
            Expr::Instancia(circuito, _) => {
                self.error(
                    format!(
                        "`{0}(...)` crea un circuito: asígnalo a un nombre (`m = {0}(...);`) y usa sus salidas con `m.Salida`",
                        circuito.texto
                    ),
                    circuito,
                );
                Ref::Nodo(self.nodo(Tipo::Constante(false), Vec::new()))
            }
        }
    }

    /// Sigue los nombres (`X = Y;`, entradas de instancias, `m.Suma`) hasta llegar a un nodo.
    fn resolver(&self, mut r: Ref<'a>) -> Result<usize, Error> {
        let mut primero = None;
        // Sin ciclos se pasa por cada nombre una vez, más un `m.Salida` por cada uno
        let limite: usize = 2 * self
            .ambitos
            .iter()
            .map(|a| a.senales.len() + 1)
            .sum::<usize>();
        for _ in 0..=limite {
            r = match r {
                Ref::Nodo(i) => return Ok(i),
                Ref::Senal(a, n) => {
                    primero.get_or_insert(n);
                    let ambito = &self.ambitos[a];
                    match ambito.senales.get(n.texto.as_str()) {
                        Some(&siguiente) => siguiente,
                        None if ambito.instancias.contains_key(n.texto.as_str()) => {
                            return Err(error(
                                format!(
                                    "`{0}` es un circuito, no una señal: usa una de sus salidas, como `{0}.Salida`",
                                    n.texto
                                ),
                                n,
                            ));
                        }
                        None => {
                            return Err(error(
                                format!(
                                    "`{}` no existe: declárala con `in` o asígnale un valor",
                                    n.texto
                                ),
                                n,
                            ));
                        }
                    }
                }
                Ref::Miembro(a, m, salida) => {
                    let Some(&hijo) = self.ambitos[a].instancias.get(m.texto.as_str()) else {
                        return Err(error(format!("`{}` no es un circuito", m.texto), m));
                    };
                    let c = self.ambitos[hijo].circuito;
                    if !c.salidas.iter().any(|s| s.texto == salida.texto) {
                        return Err(error(
                            format!("`{}` no tiene la salida `{}`", c.nombre.texto, salida.texto),
                            salida,
                        ));
                    }
                    Ref::Senal(hijo, salida)
                }
            };
        }
        let n = primero.expect("el lazo pasó por al menos un nombre");
        Err(error(
            format!(
                "`{}` se define a sí misma sin pasar por ninguna compuerta",
                n.texto
            ),
            n,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::compilar;

    fn errores(fuente: &str) -> Vec<String> {
        compilar(fuente)
            .unwrap_err()
            .into_iter()
            .map(|e| e.mensaje)
            .collect()
    }

    const MEDIO_SUMADOR: &str = "circuit MedioSumador { in A, B; out Suma, Acarreo; Suma = xor(A, B); Acarreo = and(A, B); }";

    #[test]
    fn conecta_en_cualquier_orden_y_con_realimentacion() {
        let red =
            compilar("circuit FF { in S, R; out Q; Q = nor(R, Qn); Qn = nor(S, Q); }").unwrap();
        // S, R, nor de Q, nor de Qn, salida Q
        assert_eq!(red.nodos.len(), 5);
        assert_eq!(red.nodos[2].entradas, [1, 3]); // Q = nor(R, Qn)
        assert_eq!(red.nodos[3].entradas, [0, 2]); // Qn = nor(S, Q)
        assert_eq!(red.nodos[4].entradas, [2]);
    }

    #[test]
    fn errores_de_conexion() {
        assert_eq!(
            errores("main { in A, A; out X, Y; X = and(A, B); A = 1; X = 0; Z = W; W = Z; }"),
            [
                "`A` está declarada dos veces",
                "`A` es una entrada: no se le puede asignar un valor",
                "`X` ya tiene un valor: cada señal se asigna una sola vez",
                "la salida `Y` nunca recibe un valor",
                "`B` no existe: declárala con `in` o asígnale un valor",
            ]
        );
        assert_eq!(
            errores("main { out X; X = Y; Y = X; }"),
            ["`X` se define a sí misma sin pasar por ninguna compuerta"]
        );
    }

    #[test]
    fn instancias_se_aplanan() {
        let fuente = format!(
            "{MEDIO_SUMADOR}
            circuit SumadorCompleto {{
                in A, B, Cin; out S, Cout;
                m1 = MedioSumador(A, B);
                m2 = MedioSumador(m1.Suma, Cin);
                S = m2.Suma;
                Cout = or(m1.Acarreo, m2.Acarreo);
            }}"
        );
        let red = compilar(&fuente).unwrap();
        // Se compila el que nadie usa: el sumador completo
        assert_eq!(red.nombre, "SumadorCompleto");
        // 3 entradas + 2 copias × (xor + and) + or + 2 salidas
        assert_eq!(red.nodos.len(), 3 + 4 + 1 + 2);

        // Para el dibujo: cada instancia es una caja con sus pines
        assert_eq!(red.instancias.len(), 2);
        let m2 = &red.instancias[1];
        assert_eq!(
            (m2.nombre.as_str(), m2.circuito.as_str()),
            ("m2", "MedioSumador")
        );
        assert_eq!(m2.entradas, [("A".into(), 3), ("B".into(), 2)]); // m1.Suma, Cin
        assert_eq!(m2.salidas, [("Suma".into(), 5), ("Acarreo".into(), 6)]);
        assert_eq!(red.nodos[3].instancia, Some(0));
        assert_eq!(red.nodos[3].nombre, "Suma");
        assert_eq!(
            (red.nodos[7].nombre.as_str(), red.nodos[7].instancia),
            ("Cout", None)
        );
    }

    #[test]
    fn errores_de_instancias() {
        let con = |cuerpo: &str| errores(&format!("{MEDIO_SUMADOR} main {{ {cuerpo} }}"));
        assert_eq!(
            con("out X; m = MedioSumador(1); X = m.Suma;"),
            [
                "`MedioSumador` lleva 2 entradas, pero tiene 1",
                "`m` no es un circuito"
            ]
        );
        assert_eq!(
            con("out X; m = Nada(1, 0); X = m.Suma;"),
            [
                "`Nada` no es un circuito del proyecto",
                "`m` no es un circuito"
            ]
        );
        assert_eq!(
            con("out X; m = MedioSumador(1, 0); X = m.Resta;"),
            ["`MedioSumador` no tiene la salida `Resta`"]
        );
        assert_eq!(
            con("out X; m = MedioSumador(1, 0); X = m;"),
            ["`m` es un circuito, no una señal: usa una de sus salidas, como `m.Salida`"]
        );
        assert_eq!(
            con("out X; X = and(MedioSumador(1, 0), 1);"),
            [
                "`MedioSumador(...)` crea un circuito: asígnalo a un nombre (`m = MedioSumador(...);`) y usa sus salidas con `m.Salida`"
            ]
        );
        assert_eq!(
            errores(
                "circuit Bucle { in A; out X; b = Bucle(A); X = b.X; } main { out Y; b = Bucle(1); Y = b.X; }"
            ),
            ["`Bucle` se usa dentro de sí mismo", "`b` no es un circuito"]
        );
    }
}
