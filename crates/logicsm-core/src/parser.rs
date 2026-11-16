//! Parser: tokens → AST. Descenso recursivo escrito a mano.
//!
//! ponytail: por ahora `circuit`/`main`, `in`, `out`, compuertas (`X = and(A, not(B));`),
//! circuitos propios (`m = MedioSumador(A, B);`, `m.Suma`), buses (`A[8]`, `A[i]`,
//! `A[0..3]`, `split(...)`), arreglos de instancias (`FullAdder fa[8];`) y `for`.
//! Dispositivos y memorias dan error "todavía no".

use crate::lexer::{Reservada, Tipo, Token, tokenizar};
use crate::{Compuerta, Error};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Circuito {
    pub nombre: Nombre,
    pub entradas: Vec<Puerto>,
    pub salidas: Vec<Puerto>,
    pub sentencias: Vec<Sentencia>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nombre {
    pub texto: String,
    pub rango: Range<usize>,
}

/// `A` (un bit, `ancho` = `None`) o `A[8]` (bus de 8 bits)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puerto {
    pub nombre: Nombre,
    pub ancho: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sentencia {
    Asignar(Acceso, Expr),
    /// `FullAdder fa[8];`: varias instancias, sus entradas se conectan con `fa[i].A = ...`
    Arreglo {
        circuito: Nombre,
        nombre: Nombre,
        cantidad: IExpr,
    },
    /// `for (i = 0; i < 8; i++) { ... }`: se desenrolla al compilar
    For {
        variable: Nombre,
        desde: IExpr,
        hasta: IExpr,
        cuerpo: Vec<Sentencia>,
    },
}

/// `Q`, `S[3]`, `A[0..3]`, `m.Suma`, `fa[i].Cin`, `c.Q[0]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acceso {
    pub nombre: Nombre,
    pub indice: Option<Indice>,
    /// `.Salida` con su propio índice
    pub miembro: Option<(Nombre, Option<Indice>)>,
}

/// `[i]` o `[a..b]` (inclusivo, en cualquier sentido)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Indice {
    pub desde: IExpr,
    pub hasta: Option<IExpr>,
    pub rango: Range<usize>,
}

/// Índice entero: `3`, `i`, `i-1`. Cada término con su signo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IExpr {
    pub terminos: Vec<(bool, Termino)>,
    pub rango: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Termino {
    Numero(u64),
    Variable(Nombre),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Senal(Acceso),
    Constante(bool),
    Compuerta(Compuerta, Vec<Expr>),
    /// `MedioSumador(A, B)`: una copia nueva de un circuito propio
    Instancia(Nombre, Vec<Expr>),
    /// `split.derecha(A)` separa un bus en pines (salen por la derecha);
    /// `split.izquierda(a, b, c)` junta pines en un bus (entran por la izquierda).
    /// El primer pin es el bit 0 (como en LogicCircuit). El `bool` es `true` si separa.
    Split(bool, Vec<Expr>, Range<usize>),
}

impl Circuito {
    /// Nombres de los circuitos propios que usa (también dentro de los `for`).
    pub fn usados(&self) -> Vec<&str> {
        fn juntar<'a>(sentencias: &'a [Sentencia], v: &mut Vec<&'a str>) {
            for s in sentencias {
                match s {
                    Sentencia::Asignar(_, Expr::Instancia(n, _)) => v.push(&n.texto),
                    Sentencia::Arreglo { circuito, .. } => v.push(&circuito.texto),
                    Sentencia::For { cuerpo, .. } => juntar(cuerpo, v),
                    Sentencia::Asignar(..) => {}
                }
            }
        }
        let mut v = Vec::new();
        juntar(&self.sentencias, &mut v);
        v
    }
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

    /// Consume el token si es de ese tipo.
    fn tomar(&mut self, tipo: Tipo) -> bool {
        let si = self.actual().tipo == tipo;
        if si {
            self.avanzar();
        }
        si
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
            sentencias: Vec::new(),
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
                    c.entradas.extend(self.puertos()?);
                }
                Tipo::Reservada(Reservada::Out) => {
                    self.avanzar();
                    c.salidas.extend(self.puertos()?);
                }
                _ => c.sentencias.push(self.sentencia()?),
            }
        }
    }

    /// Una asignación, un arreglo de instancias o un `for`
    fn sentencia(&mut self) -> Result<Sentencia, Error> {
        match self.actual().tipo {
            Tipo::Reservada(Reservada::For) => self.para(),
            Tipo::Ident(_) => {
                // `FullAdder fa[8];`: dos nombres seguidos
                if matches!(self.tokens[self.i + 1].tipo, Tipo::Ident(_)) {
                    let circuito = self.nombre()?;
                    let nombre = self.nombre()?;
                    self.esperar(Tipo::CorAbre, "`[` con la cantidad de copias")?;
                    let cantidad = self.iexpr()?;
                    self.esperar(Tipo::CorCierra, "`]`")?;
                    self.esperar(Tipo::PuntoComa, "`;`")?;
                    return Ok(Sentencia::Arreglo {
                        circuito,
                        nombre,
                        cantidad,
                    });
                }
                let destino = self.acceso()?;
                self.esperar(Tipo::Igual, "`=`")?;
                let valor = self.expr()?;
                self.esperar(Tipo::PuntoComa, "`;`")?;
                Ok(Sentencia::Asignar(destino, valor))
            }
            Tipo::Reservada(_) => Err(self.todavia_no()),
            _ => Err(self.error("una declaración o `}`")),
        }
    }

    /// `for (i = 0; i < 8; i++) { ... }`
    fn para(&mut self) -> Result<Sentencia, Error> {
        self.avanzar();
        self.esperar(Tipo::ParAbre, "`(`")?;
        let variable = self.nombre()?;
        self.esperar(Tipo::Igual, "`=`")?;
        let desde = self.iexpr()?;
        self.esperar(Tipo::PuntoComa, "`;`")?;
        self.misma_variable(&variable)?;
        self.esperar(Tipo::Menor, "`<`")?;
        let hasta = self.iexpr()?;
        self.esperar(Tipo::PuntoComa, "`;`")?;
        self.misma_variable(&variable)?;
        self.esperar(Tipo::MasMas, "`++`")?;
        self.esperar(Tipo::ParCierra, "`)`")?;
        self.esperar(Tipo::LlaveAbre, "`{`")?;
        let mut cuerpo = Vec::new();
        while !self.tomar(Tipo::LlaveCierra) {
            if matches!(self.actual().tipo, Tipo::Reservada(Reservada::In | Reservada::Out)) {
                return Err(Error {
                    mensaje: "`in` y `out` no van dentro de un `for`".into(),
                    rango: self.actual().rango.clone(),
                });
            }
            cuerpo.push(self.sentencia()?);
        }
        Ok(Sentencia::For {
            variable,
            desde,
            hasta,
            cuerpo,
        })
    }

    fn misma_variable(&mut self, variable: &Nombre) -> Result<(), Error> {
        let n = self.nombre()?;
        if n.texto == variable.texto {
            Ok(())
        } else {
            Err(Error {
                mensaje: format!("el `for` usa `{}`: aquí va `{}`", variable.texto, variable.texto),
                rango: n.rango,
            })
        }
    }

    /// `A, B[8], C;`
    fn puertos(&mut self) -> Result<Vec<Puerto>, Error> {
        let mut puertos = Vec::new();
        loop {
            let nombre = self.nombre()?;
            let mut ancho = None;
            if self.tomar(Tipo::CorAbre) {
                let t = self.actual().clone();
                match t.tipo {
                    Tipo::Entero(n @ 1..=64) => {
                        self.avanzar();
                        ancho = Some(n as usize);
                    }
                    Tipo::Entero(_) => {
                        return Err(Error {
                            mensaje: "un bus tiene de 1 a 64 bits".into(),
                            rango: t.rango,
                        });
                    }
                    _ => return Err(self.error("la cantidad de bits, como `[8]`")),
                }
                self.esperar(Tipo::CorCierra, "`]`")?;
            }
            puertos.push(Puerto { nombre, ancho });
            if !self.tomar(Tipo::Coma) {
                break;
            }
        }
        self.esperar(Tipo::PuntoComa, "`,` o `;`")?;
        Ok(puertos)
    }

    /// `[i]` o `[a..b]`, si hay
    fn indice(&mut self) -> Result<Option<Indice>, Error> {
        let Tipo::CorAbre = self.actual().tipo else {
            return Ok(None);
        };
        let inicio = self.avanzar().rango.start;
        let desde = self.iexpr()?;
        let hasta = if self.tomar(Tipo::PuntoPunto) {
            Some(self.iexpr()?)
        } else {
            None
        };
        let fin = self.esperar(Tipo::CorCierra, "`]` o `..`")?.rango.end;
        Ok(Some(Indice {
            desde,
            hasta,
            rango: inicio..fin,
        }))
    }

    /// `3`, `i`, `i - 1 + 2`
    fn iexpr(&mut self) -> Result<IExpr, Error> {
        let inicio = self.actual().rango.start;
        let mut terminos = Vec::new();
        let mut positivo = true;
        loop {
            let t = self.actual().clone();
            let termino = match t.tipo {
                Tipo::Entero(n) => {
                    self.avanzar();
                    Termino::Numero(n)
                }
                Tipo::Ident(_) => Termino::Variable(self.nombre()?),
                _ => return Err(self.error("un número o la variable del `for`")),
            };
            terminos.push((positivo, termino));
            positivo = match self.actual().tipo {
                Tipo::Mas => true,
                Tipo::Menos => false,
                _ => break,
            };
            self.avanzar();
        }
        let fin = self.tokens[self.i - 1].rango.end;
        Ok(IExpr {
            terminos,
            rango: inicio..fin,
        })
    }

    /// `A`, `A[3]`, `m.Suma`, `fa[i].Cout[0..3]`
    fn acceso(&mut self) -> Result<Acceso, Error> {
        let nombre = self.nombre()?;
        let indice = self.indice()?;
        let miembro = if self.tomar(Tipo::Punto) {
            Some((self.nombre()?, self.indice()?))
        } else {
            None
        };
        Ok(Acceso {
            nombre,
            indice,
            miembro,
        })
    }

    /// `A[3]`, `0`, `1`, `split(...)`, `and(expr, ...)` o `Circuito(expr, ...)`
    fn expr(&mut self) -> Result<Expr, Error> {
        let t = self.actual().clone();
        match t.tipo {
            Tipo::Ident(_) => {
                if self.tokens[self.i + 1].tipo == Tipo::ParAbre {
                    let n = self.nombre()?;
                    let (args, _) = self.argumentos()?;
                    return Ok(Expr::Instancia(n, args));
                }
                Ok(Expr::Senal(self.acceso()?))
            }
            Tipo::Reservada(Reservada::Split) => {
                self.avanzar();
                let pista = "`.derecha` (separa un bus) o `.izquierda` (junta señales)";
                if !self.tomar(Tipo::Punto) {
                    return Err(self.error(pista));
                }
                let lado = self.nombre().map_err(|_| self.error(pista))?;
                let separa = match lado.texto.as_str() {
                    "derecha" => true,
                    "izquierda" => false,
                    _ => {
                        return Err(Error {
                            mensaje: format!("se esperaba {pista}, pero hay `{}`", lado.texto),
                            rango: lado.rango,
                        });
                    }
                };
                let (partes, cierre) = self.argumentos()?;
                let rango = t.rango.start..cierre.rango.end;
                if separa && partes.len() != 1 {
                    return Err(Error {
                        mensaje: "`split.derecha` separa un solo bus: `split.derecha(A)`".into(),
                        rango,
                    });
                }
                if !separa && partes.len() < 2 {
                    return Err(Error {
                        mensaje: "`split.izquierda` junta 2 o más señales: `split.izquierda(a, b)`".into(),
                        rango,
                    });
                }
                Ok(Expr::Split(separa, partes, rango))
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
        while self.tomar(Tipo::Coma) {
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

    fn asignacion(s: &Sentencia) -> (&Acceso, &Expr) {
        match s {
            Sentencia::Asignar(a, e) => (a, e),
            _ => panic!("se esperaba una asignación"),
        }
    }

    #[test]
    fn circuito_con_compuertas_anidadas() {
        let c = &parsear("circuit Demo { in A, B; out X; X = and(A, not(B), 1); }").unwrap()[0];
        assert_eq!(c.nombre.texto, "Demo");
        assert_eq!(c.entradas.len(), 2);
        assert_eq!(c.salidas[0].nombre.texto, "X");
        let Expr::Compuerta(Compuerta::And, args) = asignacion(&c.sentencias[0]).1 else {
            panic!("se esperaba un and");
        };
        assert!(matches!(args[1], Expr::Compuerta(Compuerta::Not, _)));
        assert_eq!(args[2], Expr::Constante(true));
    }

    #[test]
    fn instancias_y_salidas() {
        let c = &parsear("main { m = MedioSumador(A, 1); X = m.Suma; }").unwrap()[0];
        let Expr::Instancia(circuito, args) = asignacion(&c.sentencias[0]).1 else {
            panic!("se esperaba una instancia");
        };
        assert_eq!(circuito.texto, "MedioSumador");
        assert_eq!(args.len(), 2);
        let Expr::Senal(a) = asignacion(&c.sentencias[1]).1 else {
            panic!("se esperaba m.Suma");
        };
        assert_eq!(a.nombre.texto, "m");
        assert_eq!(a.miembro.as_ref().unwrap().0.texto, "Suma");
        assert_eq!(c.usados(), ["MedioSumador"]);
    }

    #[test]
    fn buses_arreglos_y_for() {
        let c = &parsear(
            "circuit S { in A[8], C; out S[8];
             FullAdder fa[8];
             for (i = 1; i < 8; i++) { fa[i].Cin = fa[i-1].Cout; S[i] = split.izquierda(A[0..3], C); }
             }",
        )
        .unwrap()[0];
        assert_eq!(c.entradas[0].ancho, Some(8));
        assert_eq!(c.entradas[1].ancho, None);
        assert!(matches!(&c.sentencias[0], Sentencia::Arreglo { nombre, .. } if nombre.texto == "fa"));
        let Sentencia::For { variable, cuerpo, .. } = &c.sentencias[1] else {
            panic!("se esperaba un for");
        };
        assert_eq!(variable.texto, "i");
        let (destino, valor) = asignacion(&cuerpo[0]);
        assert!(destino.indice.is_some() && destino.miembro.is_some());
        let Expr::Senal(a) = valor else { panic!() };
        assert_eq!(a.indice.as_ref().unwrap().desde.terminos.len(), 2); // i-1
        assert!(matches!(asignacion(&cuerpo[1]).1, Expr::Split(false, p, _) if p.len() == 2));
        assert_eq!(c.usados(), ["FullAdder"]);
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
        assert_eq!(error("main { in A[0]; }"), "un bus tiene de 1 a 64 bits");
        assert_eq!(
            error("main { X = split(A); }"),
            "se esperaba `.derecha` (separa un bus) o `.izquierda` (junta señales), pero hay `(`"
        );
        assert_eq!(
            error("main { X = split.izquierda(A); }"),
            "`split.izquierda` junta 2 o más señales: `split.izquierda(a, b)`"
        );

        assert_eq!(
            error("main { for (i = 0; j < 8; i++) { } }"),
            "el `for` usa `i`: aquí va `i`"
        );
        assert_eq!(
            error("main { for (i = 0; i < 8; i++) { in A; } }"),
            "`in` y `out` no van dentro de un `for`"
        );
    }
}
