//! Parser: tokens → AST. Descenso recursivo escrito a mano.
//!
//! `circuit`/`main`, `in`, `out`, `button`, `clock`, compuertas (`X = and(A, not(B));`),
//! circuitos propios (`m = MedioSumador(A, B);`, `m.Suma`), buses (`A[8]`, `A[i]`,
//! `A[0..3]`, `split(...)`), arreglos de instancias (`FullAdder fa[8];`), `for`,
//! dispositivos de salida (`led L(Q);`) y moldes de memoria (`R = rom.size(4, 8, "...");`).

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
    /// Solo en las entradas: `in`, `button` o `clock`
    pub clase: Clase,
}

/// Cómo se maneja una entrada del circuito que se simula. Dentro de otro
/// circuito las tres son pines normales.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Clase {
    /// `in A;`: un click la cambia y se queda
    #[default]
    Interruptor,
    /// `button B;`: vale 1 mientras se mantiene presionado
    Pulsador,
    /// `clock CLK;`: alterna sola con la frecuencia del sistema
    Reloj,
}

/// Dispositivos que muestran señales: `led L(Q);`, `display7 D(...);`,
/// `matrix M(filas, columnas, ...);`, `probe P(...);`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visor {
    Led,
    Display7,
    /// Filas y columnas
    Matrix(usize, usize),
    Probe,
}

impl Visor {
    pub fn nombre(self) -> &'static str {
        match self {
            Visor::Led => "led",
            Visor::Display7 => "display7",
            Visor::Matrix(..) => "matrix",
            Visor::Probe => "probe",
        }
    }
}

/// `rom.size(4, 8, "00 1F ...")` o `ram.size(4, 8)`: bits de dirección y de dato.
/// Los datos de la ROM van en hexadecimal desde la dirección 0; lo que falta vale 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Molde {
    pub ram: bool,
    pub direccion: usize,
    pub dato: usize,
    pub datos: Vec<u64>,
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
    /// `led L(Q);`
    Visor {
        tipo: Visor,
        nombre: Nombre,
        senales: Vec<Expr>,
    },
}

/// `Q`, `S[3]`, `A[0..3]`, `m.Suma`, `fa[i].Cin`, `c.Q[0]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acceso {
    pub nombre: Nombre,
    /// En caja: sin ella cada `Acceso` (y cada `Expr`) pesaría ~270 bytes
    pub indice: Option<Box<Indice>>,
    /// `.Salida` con su propio índice
    pub miembro: Option<(Nombre, Option<Box<Indice>>)>,
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
    /// `rom.size(...)` / `ram.size(...)`: se guarda en un nombre y se usa con `R(dir)`
    Molde(Molde, Range<usize>),
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
                    Sentencia::Asignar(..) | Sentencia::Visor { .. } => {}
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
    let mut moldes = Vec::new();
    while p.actual().tipo != Tipo::Fin {
        if matches!(p.actual().tipo, Tipo::Ident(_)) {
            moldes.push(p.molde_global().map_err(|e| vec![e])?);
        } else {
            circuitos.push(p.circuito().map_err(|e| vec![e])?);
        }
    }
    // Un molde afuera se ve en todos los circuitos del proyecto, salvo donde
    // un nombre propio del circuito lo tapa
    for c in &mut circuitos {
        let propios: Vec<String> = c
            .sentencias
            .iter()
            .filter_map(|s| match s {
                Sentencia::Asignar(d, _) => Some(d.nombre.texto.clone()),
                _ => None,
            })
            .collect();
        let visibles = moldes.iter().filter(|m| match m {
            Sentencia::Asignar(d, _) => !propios.contains(&d.nombre.texto),
            _ => false,
        });
        let mut todas: Vec<Sentencia> = visibles.cloned().collect();
        todas.append(&mut c.sentencias);
        c.sentencias = todas;
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

    /// `R = rom.size(...);` fuera de los circuitos
    fn molde_global(&mut self) -> Result<Sentencia, Error> {
        let inicio = self.actual().rango.clone();
        let s = self.sentencia()?;
        let molde = matches!(&s, Sentencia::Asignar(d, Expr::Molde(..)) if d.indice.is_none() && d.miembro.is_none());
        if molde {
            return Ok(s);
        }
        Err(Error {
            mensaje: "fuera de un circuito solo van moldes de memoria, como `R = ram.size(4, 8);`; lo demás va dentro de `circuit Nombre { ... }`".into(),
            rango: inicio,
        })
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
                    c.entradas.extend(self.puertos(Clase::Interruptor)?);
                }
                Tipo::Reservada(Reservada::Button) => {
                    self.avanzar();
                    c.entradas.extend(self.puertos(Clase::Pulsador)?);
                }
                Tipo::Reservada(Reservada::Clock) => {
                    self.avanzar();
                    c.entradas.extend(self.puertos(Clase::Reloj)?);
                }
                Tipo::Reservada(Reservada::Out) => {
                    self.avanzar();
                    c.salidas.extend(self.puertos(Clase::Interruptor)?);
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
            Tipo::Reservada(
                Reservada::Led | Reservada::Display7 | Reservada::Matrix | Reservada::Probe,
            ) => self.visor(),
            _ => Err(self.error("una declaración o `}`")),
        }
    }

    /// `led L(Q);`, `display7 D(...);`, `matrix M(8, 8, ...);`, `probe P(...);`
    fn visor(&mut self) -> Result<Sentencia, Error> {
        let palabra = self.avanzar();
        let nombre = self.nombre()?;
        let (tipo, senales) = match palabra.tipo {
            Tipo::Reservada(Reservada::Matrix) => {
                self.esperar(Tipo::ParAbre, "`(` con las filas y columnas")?;
                let filas = self.entero(1, 64, "la cantidad de filas")?;
                self.esperar(Tipo::Coma, "`,`")?;
                let columnas = self.entero(1, 64, "la cantidad de columnas")?;
                self.esperar(Tipo::Coma, "`,` y las señales de las filas y columnas")?;
                let mut senales = vec![self.expr()?];
                while self.tomar(Tipo::Coma) {
                    senales.push(self.expr()?);
                }
                self.esperar(Tipo::ParCierra, "`,` o `)`")?;
                (Visor::Matrix(filas, columnas), senales)
            }
            Tipo::Reservada(r) => {
                let tipo = match r {
                    Reservada::Led => Visor::Led,
                    Reservada::Display7 => Visor::Display7,
                    _ => Visor::Probe,
                };
                let (senales, cierre) = self.argumentos()?;
                if tipo == Visor::Led && senales.len() != 1 {
                    return Err(Error {
                        mensaje: "`led` muestra una señal: `led L(Q);`".into(),
                        rango: palabra.rango.start..cierre.rango.end,
                    });
                }
                (tipo, senales)
            }
            _ => unreachable!("visor() se llama con un dispositivo"),
        };
        self.esperar(Tipo::PuntoComa, "`;`")?;
        Ok(Sentencia::Visor {
            tipo,
            nombre,
            senales,
        })
    }

    /// Un número entero entre `min` y `max`
    fn entero(&mut self, min: usize, max: usize, que: &str) -> Result<usize, Error> {
        let t = self.actual().clone();
        match t.tipo {
            Tipo::Entero(n) if (min as u64..=max as u64).contains(&n) => {
                self.avanzar();
                Ok(n as usize)
            }
            Tipo::Entero(n) => Err(Error {
                mensaje: format!("se esperaba {que} de {min} a {max}, pero hay `{n}`"),
                rango: t.rango,
            }),
            _ => Err(self.error(que)),
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
            if let Tipo::Reservada(
                Reservada::In
                | Reservada::Out
                | Reservada::Button
                | Reservada::Clock
                | Reservada::Led
                | Reservada::Display7
                | Reservada::Matrix
                | Reservada::Probe,
            ) = self.actual().tipo
            {
                let t = self.actual();
                return Err(Error {
                    mensaje: format!(
                        "`{}` no va dentro de un `for`",
                        &self.fuente[t.rango.clone()]
                    ),
                    rango: t.rango.clone(),
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
                mensaje: format!(
                    "el `for` usa `{}`: aquí va `{}`",
                    variable.texto, variable.texto
                ),
                rango: n.rango,
            })
        }
    }

    /// `A, B[8], C;`
    fn puertos(&mut self, clase: Clase) -> Result<Vec<Puerto>, Error> {
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
            puertos.push(Puerto {
                nombre,
                ancho,
                clase,
            });
            if !self.tomar(Tipo::Coma) {
                break;
            }
        }
        self.esperar(Tipo::PuntoComa, "`,` o `;`")?;
        Ok(puertos)
    }

    /// `[i]` o `[a..b]`, si hay
    fn indice(&mut self) -> Result<Option<Box<Indice>>, Error> {
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
        Ok(Some(Box::new(Indice {
            desde,
            hasta,
            rango: inicio..fin,
        })))
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
                        mensaje: "`split.izquierda` junta 2 o más señales: `split.izquierda(a, b)`"
                            .into(),
                        rango,
                    });
                }
                Ok(Expr::Split(separa, partes, rango))
            }
            Tipo::Reservada(r @ (Reservada::Rom | Reservada::Ram)) => {
                self.avanzar();
                let ram = r == Reservada::Ram;
                let pista = if ram {
                    "`.size(bits de dirección, bits de dato)`"
                } else {
                    "`.size(bits de dirección, bits de dato, \"datos en hexadecimal\")`"
                };
                if !self.tomar(Tipo::Punto) {
                    return Err(self.error(pista));
                }
                let size = self.nombre().map_err(|_| self.error(pista))?;
                if size.texto != "size" {
                    return Err(Error {
                        mensaje: format!("se esperaba {pista}, pero hay `{}`", size.texto),
                        rango: size.rango,
                    });
                }
                self.esperar(Tipo::ParAbre, "`(`")?;
                let direccion = self.entero(1, 16, "los bits de dirección")?;
                self.esperar(Tipo::Coma, "`,`")?;
                let dato = self.entero(1, 64, "los bits de dato")?;
                let mut datos = Vec::new();
                if !ram && self.tomar(Tipo::Coma) {
                    let d = self.actual().clone();
                    let Tipo::Texto(texto) = &d.tipo else {
                        return Err(self.error("los datos entre comillas, como `\"00 1F A3\"`"));
                    };
                    self.avanzar();
                    for palabra in texto.split_whitespace() {
                        let v = u64::from_str_radix(palabra, 16)
                            .ok()
                            .filter(|&v| dato == 64 || v >> dato == 0);
                        let Some(v) = v else {
                            return Err(Error {
                                mensaje: format!(
                                    "`{palabra}` no es un dato de {dato} bits en hexadecimal"
                                ),
                                rango: d.rango.clone(),
                            });
                        };
                        datos.push(v);
                    }
                    if datos.len() > 1 << direccion {
                        return Err(Error {
                            mensaje: format!(
                                "la ROM tiene {} direcciones, pero hay {} datos",
                                1 << direccion,
                                datos.len()
                            ),
                            rango: d.rango.clone(),
                        });
                    }
                }
                let cierre = self.esperar(Tipo::ParCierra, "`)`")?;
                let molde = Molde {
                    ram,
                    direccion,
                    dato,
                    datos,
                };
                Ok(Expr::Molde(molde, t.rango.start..cierre.rango.end))
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
                    return Err(self.error("una señal, `0`, `1` o una compuerta"));
                };
                self.avanzar();
                let (args, cierre) = self.argumentos()?;
                if !compuerta.entradas_validas(args.len()) {
                    let cuantas = match compuerta {
                        Compuerta::Not | Compuerta::Buffer => "1 entrada",
                        Compuerta::Tristate => "2 entradas (dato, habilitar)",
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
        assert!(
            matches!(&c.sentencias[0], Sentencia::Arreglo { nombre, .. } if nombre.texto == "fa")
        );
        let Sentencia::For {
            variable, cuerpo, ..
        } = &c.sentencias[1]
        else {
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
            error("main { led L(A, B); }"),
            "`led` muestra una señal: `led L(Q);`"
        );
        assert_eq!(
            error("main { X = tristate(A); }"),
            "`tristate` lleva 2 entradas (dato, habilitar), pero tiene 1"
        );
        assert_eq!(
            error("main { R = rom.size(4, 4, \"1F\"); }"),
            "`1F` no es un dato de 4 bits en hexadecimal"
        );
        assert_eq!(
            error("main { R = rom.size(1, 8, \"1 2 3\"); }"),
            "la ROM tiene 2 direcciones, pero hay 3 datos"
        );
        assert_eq!(
            error("main { M = ram.size(20, 8); }"),
            "se esperaba los bits de dirección de 1 a 16, pero hay `20`"
        );
        assert_eq!(
            error("main { M = ram.tamano(2, 8); }"),
            "se esperaba `.size(bits de dirección, bits de dato)`, pero hay `tamano`"
        );
        assert_eq!(
            error("X = and(A, B); main { }"),
            "fuera de un circuito solo van moldes de memoria, como `R = ram.size(4, 8);`; lo demás va dentro de `circuit Nombre { ... }`"
        );
        assert_eq!(
            error("main { for (i = 0; i < 2; i++) { led L(A); } }"),
            "`led` no va dentro de un `for`"
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
            "`in` no va dentro de un `for`"
        );
    }

    #[test]
    fn dispositivos_y_memorias() {
        let c = &parsear(
            "R = rom.size(2, 4, \"0 1 F\");
             main { in A; button B; clock CLK; out X;
                 led L(A); matrix M(2, 3, F, C); display7 D(S);
                 M2 = ram.size(3, 8); X = tristate(A, B); }
             circuit Otro { R = 1; }",
        )
        .unwrap();
        let main = &c[0];
        let clases: Vec<Clase> = main.entradas.iter().map(|p| p.clase).collect();
        assert_eq!(clases, [Clase::Interruptor, Clase::Pulsador, Clase::Reloj]);
        // El molde de afuera queda al principio de cada circuito que no lo tapa
        let Expr::Molde(rom, _) = asignacion(&main.sentencias[0]).1 else {
            panic!("se esperaba el molde de la ROM");
        };
        assert_eq!((rom.ram, rom.direccion, rom.dato), (false, 2, 4));
        assert_eq!(rom.datos, [0, 1, 15]);
        assert!(matches!(
            &main.sentencias[2],
            Sentencia::Visor { tipo: Visor::Matrix(2, 3), senales, .. } if senales.len() == 2
        ));
        assert!(matches!(
            asignacion(&main.sentencias[4]).1,
            Expr::Molde(
                Molde {
                    ram: true,
                    direccion: 3,
                    dato: 8,
                    ..
                },
                _
            )
        ));
        assert!(matches!(
            asignacion(&main.sentencias[5]).1,
            Expr::Compuerta(Compuerta::Tristate, _)
        ));
        assert_eq!(c[1].sentencias.len(), 1);
    }
}
