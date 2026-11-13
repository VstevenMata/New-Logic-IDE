//! Elaboración: AST → netlist (nodos conectados, sin nombres por resolver).

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Netlist {
    pub nombre: String,
    pub nodos: Vec<Nodo>,
}

/// De dónde sale una señal mientras se elabora: un nodo ya creado o el nombre
/// de otra señal (se resuelve al final, así el orden no importa).
enum Fuente<'a> {
    Nodo(usize),
    Senal(&'a Nombre),
}

fn error(mensaje: String, n: &Nombre) -> Error {
    Error {
        mensaje,
        rango: n.rango.clone(),
    }
}

pub fn elaborar(c: &Circuito) -> Result<Netlist, Vec<Error>> {
    let mut nodos = Vec::new();
    // Paralelo a `nodos`: de dónde sale cada una de sus entradas
    let mut pendientes: Vec<Vec<Fuente>> = Vec::new();
    let mut senales: HashMap<&str, Fuente> = HashMap::new();
    let mut errores = Vec::new();

    for n in &c.entradas {
        if senales.contains_key(n.texto.as_str()) {
            errores.push(error(format!("`{}` está declarada dos veces", n.texto), n));
            continue;
        }
        senales.insert(&n.texto, Fuente::Nodo(nodos.len()));
        nodos.push(Nodo {
            tipo: Tipo::Entrada(n.texto.clone()),
            entradas: Vec::new(),
        });
        pendientes.push(Vec::new());
    }

    for (destino, valor) in &c.asignaciones {
        let mensaje = if c.entradas.iter().any(|e| e.texto == destino.texto) {
            "es una entrada: no se le puede asignar un valor"
        } else if senales.contains_key(destino.texto.as_str()) {
            "ya tiene un valor: cada señal se asigna una sola vez"
        } else {
            let f = construir(valor, &mut nodos, &mut pendientes);
            senales.insert(&destino.texto, f);
            continue;
        };
        errores.push(error(format!("`{}` {mensaje}", destino.texto), destino));
    }

    for n in &c.salidas {
        if !senales.contains_key(n.texto.as_str()) {
            errores.push(error(
                format!("la salida `{}` nunca recibe un valor", n.texto),
                n,
            ));
            continue;
        }
        nodos.push(Nodo {
            tipo: Tipo::Salida(n.texto.clone()),
            entradas: Vec::new(),
        });
        pendientes.push(vec![Fuente::Senal(n)]);
    }

    // Aquí se cierran los lazos de realimentación (Q = nor(R, Qn); Qn = nor(S, Q);)
    for (i, fuentes) in pendientes.iter().enumerate() {
        for f in fuentes {
            match resolver(f, &senales) {
                Ok(id) => nodos[i].entradas.push(id),
                Err(e) => errores.push(e),
            }
        }
    }

    if errores.is_empty() {
        Ok(Netlist {
            nombre: c.nombre.clone(),
            nodos,
        })
    } else {
        Err(errores)
    }
}

/// Crea los nodos de una expresión (las compuertas anidadas también).
fn construir<'a>(
    e: &'a Expr,
    nodos: &mut Vec<Nodo>,
    pendientes: &mut Vec<Vec<Fuente<'a>>>,
) -> Fuente<'a> {
    let (tipo, fuentes) = match e {
        Expr::Senal(n) => return Fuente::Senal(n),
        Expr::Constante(v) => (Tipo::Constante(*v), Vec::new()),
        Expr::Compuerta(c, args) => (
            Tipo::Compuerta(*c),
            args.iter()
                .map(|a| construir(a, nodos, pendientes))
                .collect(),
        ),
    };
    nodos.push(Nodo {
        tipo,
        entradas: Vec::new(),
    });
    pendientes.push(fuentes);
    Fuente::Nodo(nodos.len() - 1)
}

/// Sigue los nombres (`X = Y;`) hasta llegar a un nodo.
fn resolver<'b>(
    mut f: &'b Fuente<'b>,
    senales: &'b HashMap<&'b str, Fuente<'b>>,
) -> Result<usize, Error> {
    let mut ultimo = None;
    for _ in 0..=senales.len() {
        match f {
            Fuente::Nodo(i) => return Ok(*i),
            Fuente::Senal(n) => match senales.get(n.texto.as_str()) {
                Some(siguiente) => {
                    ultimo = Some(*n);
                    f = siguiente;
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
            },
        }
    }
    let n = ultimo.expect("el lazo pasó por al menos un nombre");
    Err(error(
        format!(
            "`{}` se define a sí misma sin pasar por ninguna compuerta",
            n.texto
        ),
        n,
    ))
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
}
