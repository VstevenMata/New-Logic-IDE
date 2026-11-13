//! Lexer, parser, análisis semántico y elaboración: código `.SA` → Circuit IR.

pub mod compuerta;
pub mod lexer;
pub mod netlist;
pub mod parser;

pub use compuerta::Compuerta;
pub use netlist::Netlist;

/// Un error del compilador, con su posición en bytes dentro del código.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub mensaje: String,
    pub rango: std::ops::Range<usize>,
}

/// Compila el primer circuito del código.
// ponytail: un solo circuito; elegir cuál cuando haya proyectos con varios archivos
pub fn compilar(fuente: &str) -> Result<Netlist, Vec<Error>> {
    let circuitos = parser::parsear(fuente)?;
    let Some(circuito) = circuitos.first() else {
        return Err(vec![Error {
            mensaje: "no hay ningún circuito: empieza con `circuit Nombre { ... }`".into(),
            rango: 0..0,
        }]);
    };
    netlist::elaborar(circuito)
}
