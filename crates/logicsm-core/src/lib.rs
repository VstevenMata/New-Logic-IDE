//! Lexer, parser, análisis semántico y elaboración: código `.SA` → Circuit IR.

pub mod compuerta;
pub mod lexer;
pub mod netlist;
pub mod parser;

pub use compuerta::Compuerta;
pub use netlist::Netlist;

use parser::{Circuito, Expr};
use std::collections::HashSet;

/// Un error del compilador, con su posición en bytes dentro del código.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub mensaje: String,
    pub rango: std::ops::Range<usize>,
}

/// Compila el circuito principal del código (ver [`principal`]).
pub fn compilar(fuente: &str) -> Result<Netlist, Vec<Error>> {
    let circuitos = parser::parsear(fuente)?;
    let Some(c) = principal(&circuitos) else {
        return Err(vec![Error {
            mensaje: "no hay ningún circuito: empieza con `circuit Nombre { ... }`".into(),
            rango: 0..0,
        }]);
    };
    netlist::elaborar(c, &circuitos)
}

/// Compila el circuito llamado `nombre`. La app junta todos los archivos del
/// proyecto en `fuente` y elige el circuito del archivo abierto.
pub fn compilar_circuito(fuente: &str, nombre: &str) -> Result<Netlist, Vec<Error>> {
    let circuitos = parser::parsear(fuente)?;
    match circuitos.iter().find(|c| c.nombre.texto == nombre) {
        Some(c) => netlist::elaborar(c, &circuitos),
        None => Err(vec![Error {
            mensaje: format!("no existe el circuito `{nombre}`"),
            rango: 0..0,
        }]),
    }
}

/// `main` si existe; si no, el último circuito que ningún otro usa (el de más
/// arriba en la jerarquía, por ejemplo el sumador completo y no el medio sumador).
pub fn principal(circuitos: &[Circuito]) -> Option<&Circuito> {
    if let Some(c) = circuitos.iter().find(|c| c.nombre.texto == "main") {
        return Some(c);
    }
    let usados: HashSet<&str> = circuitos
        .iter()
        .flat_map(|c| &c.asignaciones)
        .filter_map(|(_, e)| match e {
            Expr::Instancia(n, _) => Some(n.texto.as_str()),
            _ => None,
        })
        .collect();
    circuitos
        .iter()
        .rev()
        .find(|c| !usados.contains(c.nombre.texto.as_str()))
        .or(circuitos.last())
}
