//! Parser: tokens → AST. Descenso recursivo escrito a mano.
//!
//! ponytail: por ahora `circuit`/`main`, `in`, `out`, compuertas (`X = and(A, not(B));`)
//! y circuitos propios (`m = MedioSumador(A, B);`, `m.Suma`). Buses, `for`,
//! dispositivos y memorias dan error "todavía no".

use crate::lexer::{Reservada, Tipo, Token, tokenizar};
use crate::{Compuerta, Error};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Circuito {
    pub nombre: Nombre,
    pub entradas: Vec<Nombre>,
    pub salidas: Vec<Nombre>,
    pub asignaciones: Vec<(Nombre, Expr)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nombre {
    pub texto: String,
    pub rango: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Senal(Nombre),
    Constante(bool),
    Compuerta(Compuerta, Vec<Expr>),
    /// `MedioSumador(A, B)`: una copia nueva de un circuito propio
    Instancia(Nombre, Vec<Expr>),
    /// `m.Suma`: una salida de una instancia
    Miembro(Nombre, Nombre),
}

/// Devuelve todos los errores del lexer, o el primero de sintaxis.
pub fn parsear(fuente: &str) -> Result<Vec<Circuito>, Vec<Error>> {
    let (tokens, errores) = tokenizar(fuente);
    if !errores.is_empty() {
        return Err(errores);
    }
    let mut p = Parser {
        fuente,
        tokens,
        i: 0,
    };
    let mut circuitos = Vec::new();
    while p.actual().tipo != Tipo::Fin {
        circuitos.push(p.circuito().map_err(|e| vec![e])?);
    }
    Ok(circuitos)
}

struct Parser<'a> {
    fuente: &'a str,
    tokens: Vec<Token>,
    i: usize,
}

impl Parser<'_> {
    fn actual(&self) -> &Token {
        &self.tokens[self.i]
    }

    /// Consume el token actual (el `Fin` nunca se consume).
    fn avanzar(&mut self) -> Token {
        let t = self.tokens[self.i].clone();
        if t.tipo != Tipo::Fin {
            self.i += 1;
        }
        t
    }

    /// Error en el token actual: "se esperaba X, pero hay `Y`".
    fn error(&self, esperado: &str) -> Error {
        let t = self.actual();
        let hay = if t.tipo == Tipo::Fin {
            "el final del archivo".to_string()
        } else {
            format!("`{}`", &self.fuente[t.rango.clone()])
        };
        Error {
            mensaje: format!("se esperaba {esperado}, pero hay {hay}"),
            rango: t.rango.clone(),
        }
    }

    fn todavia_no(&self) -> Error {
        let t = self.actual();
        Error {
            mensaje: format!(
                "`{}` todavía no está disponible en esta versión",
                &self.fuente[t.rango.clone()]
            ),
            rango: t.rango.clone(),
        }
    }

    fn esperar(&mut self, tipo: Tipo, esperado: &str) -> Result<Token, Error> {
        if self.actual().tipo == tipo {
            Ok(self.avanzar())
        } else {
            Err(self.error(esperado))
        }
    }

    fn nombre(&mut self) -> Result<Nombre, Error> {
        match &self.actual().tipo {
            Tipo::Ident(texto) => {
                let texto = texto.clone();
                let rango = self.avanzar().rango;
                Ok(Nombre { texto, rango })
            }
            _ => Err(self.error("un nombre")),
        }
    }

    /// `circuit Nombre { ... }` o `main { ... }`
    fn circuito(&mut self) -> Result<Circuito, Error> {
        let nombre = match self.actual().tipo {
            Tipo::Reservada(Reservada::Circuit) => {
                self.avanzar();
                self.nombre()?
            }
            Tipo::Reservada(Reservada::Main) => Nombre {
                texto: "main".into(),
                rango: self.avanzar().rango,
            },
            _ => return Err(self.error("`circuit` o `main`")),
        };
        let mut c = Circuito {
            nombre,
            entradas: Vec::new(),
            salidas: Vec::new(),
            asignaciones: Vec::new(),
        };
        self.esperar(Tipo::LlaveAbre, "`{`")?;
        loop {
            match self.actual().tipo {
                Tipo::LlaveCierra => {
                    self.avanzar();
                    return Ok(c);
                }
                Tipo::Reservada(Reservada::In) => {
                    self.avanzar();
                    c.entradas.extend(self.lista_de_nombres()?);
                }
                Tipo::Reservada(Reservada::Out) => {
                    self.avanzar();
                    c.salidas.extend(self.lista_de_nombres()?);
                }
                Tipo::Ident(_) => {
                    let destino = self.nombre()?;
                    self.esperar(Tipo::Igual, "`=`")?;
                    let valor = self.expr()?;
                    self.esperar(Tipo::PuntoComa, "`;`")?;
                    c.asignaciones.push((destino, valor));
                }
                Tipo::Reservada(_) => return Err(self.todavia_no()),
                _ => return Err(self.error("una declaración o `}`")),
            }
        }
    }

    /// `A, B, C;`
    fn lista_de_nombres(&mut self) -> Result<Vec<Nombre>, Error> {
        let mut nombres = vec![self.nombre()?];
        while self.actual().tipo == Tipo::Coma {
            self.avanzar();
            nombres.push(self.nombre()?);
        }
        self.esperar(Tipo::PuntoComa, "`,` o `;`")?;
        Ok(nombres)
    }

    /// `A`, `0`, `1` o `and(expr, expr, ...)`
    fn expr(&mut self) -> Result<Expr, Error> {
        let t = self.actual().clone();
        match t.tipo {
            Tipo::Ident(_) => {
                let n = self.nombre()?;
                match self.actual().tipo {
                    Tipo::ParAbre => {
                        let (args, _) = self.argumentos()?;
                        Ok(Expr::Instancia(n, args))
                    }
                    Tipo::Punto => {
                        self.avanzar();
                        Ok(Expr::Miembro(n, self.nombre()?))
                    }
                    _ => Ok(Expr::Senal(n)),
                }
            }
            Tipo::Entero(v @ (0 | 1)) => {
                self.avanzar();
                Ok(Expr::Constante(v == 1))
            }
            Tipo::Entero(_) => Err(Error {
                mensaje: "como señal solo se puede usar `0` o `1`".into(),
                rango: t.rango,
            }),
            Tipo::Reservada(_) => {
                let fuente = self.fuente;
                let texto = &fuente[t.rango.clone()];
                let Some(compuerta) = Compuerta::desde_nombre(texto) else {
                    return Err(self.todavia_no());
                };
                self.avanzar();
                let (args, cierre) = self.argumentos()?;
                if !compuerta.entradas_validas(args.len()) {
                    let cuantas = match compuerta {
                        Compuerta::Not | Compuerta::Buffer => "1 entrada",
                        _ => "2 o más entradas",
                    };
                    return Err(Error {
                        mensaje: format!("`{texto}` lleva {cuantas}, pero tiene {}", args.len()),
                        rango: t.rango.start..cierre.rango.end,
                    });
                }
                Ok(Expr::Compuerta(compuerta, args))
            }
            _ => Err(self.error("una señal, `0`, `1` o una compuerta")),
        }
    }

    /// `(expr, expr, ...)`; devuelve también el `)` para marcar errores
    fn argumentos(&mut self) -> Result<(Vec<Expr>, Token), Error> {
        self.esperar(Tipo::ParAbre, "`(`")?;
        let mut args = vec![self.expr()?];
        while self.actual().tipo == Tipo::Coma {
            self.avanzar();
            args.push(self.expr()?);
        }
        let cierre = self.esperar(Tipo::ParCierra, "`,` o `)`")?;
        Ok((args, cierre))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(fuente: &str) -> String {
        parsear(fuente).unwrap_err()[0].mensaje.clone()
    }

    #[test]
    fn circuito_con_compuertas_anidadas() {
        let c = &parsear("circuit Demo { in A, B; out X; X = and(A, not(B), 1); }").unwrap()[0];
        assert_eq!(c.nombre.texto, "Demo");
        assert_eq!(c.entradas.len(), 2);
        assert_eq!(c.salidas[0].texto, "X");
        let Expr::Compuerta(Compuerta::And, args) = &c.asignaciones[0].1 else {
            panic!("se esperaba un and");
        };
        assert!(matches!(args[1], Expr::Compuerta(Compuerta::Not, _)));
        assert_eq!(args[2], Expr::Constante(true));
    }

    #[test]
    fn instancias_y_salidas() {
        let c = &parsear("main { m = MedioSumador(A, 1); X = m.Suma; }").unwrap()[0];
        let Expr::Instancia(circuito, args) = &c.asignaciones[0].1 else {
            panic!("se esperaba una instancia");
        };
        assert_eq!(circuito.texto, "MedioSumador");
        assert_eq!(args.len(), 2);
        let Expr::Miembro(m, salida) = &c.asignaciones[1].1 else {
            panic!("se esperaba m.Suma");
        };
        assert_eq!((m.texto.as_str(), salida.texto.as_str()), ("m", "Suma"));
    }

    #[test]
    fn errores() {
        assert_eq!(error("circuit { }"), "se esperaba un nombre, pero hay `{`");
        assert_eq!(error("main { X = A }"), "se esperaba `;`, pero hay `}`");
        assert_eq!(
            error("main { in A"),
            "se esperaba `,` o `;`, pero hay el final del archivo"
        );
        assert_eq!(
            error("main { X = not(A, B); }"),
            "`not` lleva 1 entrada, pero tiene 2"
        );
        assert_eq!(
            error("main { X = and(A); }"),
            "`and` lleva 2 o más entradas, pero tiene 1"
        );
        assert_eq!(
            error("main { X = 2; }"),
            "como señal solo se puede usar `0` o `1`"
        );
        assert_eq!(
            error("main { led L(A); }"),
            "`led` todavía no está disponible en esta versión"
        );
    }
}
