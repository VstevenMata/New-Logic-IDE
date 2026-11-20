//! Elaboración: AST → netlist (nodos conectados, sin nombres por resolver).
//!
//! Los circuitos propios se aplanan: cada instancia copia las compuertas de su
//! circuito en un ámbito de nombres propio. Los buses se separan en bits con
//! nombre (`A[0]`, `A[1]`, ...) y los `for` se desenrollan, así la red solo tiene
//! señales de un bit.

use crate::parser::{Acceso, Circuito, Expr, IExpr, Indice, Nombre, Puerto, Sentencia, Termino};
pub use crate::parser::{Clase, Molde, Visor};
use crate::{Compuerta, Error};
use std::collections::{HashMap, HashSet};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tipo {
    Entrada(String, Clase),
    Salida(String),
    Constante(bool),
    Compuerta(Compuerta),
    /// Varios `tristate` asignados a la misma señal: vale lo que dé el que no está en Z
    Bus,
    /// `led L(Q);` y los demás: muestran sus entradas y no dan ninguna señal
    Visor(Visor, String),
    /// Un bit del dato que lee una memoria (en `Netlist::memorias`). Sus entradas son
    /// la dirección y, si es RAM, el dato, `escribir` y el reloj.
    Memoria {
        memoria: usize,
        bit: usize,
    },
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
    /// Cada entrada (un pin por bit) con el nodo que la alimenta
    pub entradas: Vec<(String, usize)>,
    /// Cada salida (un pin por bit) con el nodo que la da
    pub salidas: Vec<(String, usize)>,
    /// Si es un `split`: de qué lado está el bus
    pub split: Option<Split>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    /// `split(A)`: entra un bus, sale un pin por bit
    Separa,
    /// `split(a, b, c)`: entra un pin por bit, sale un bus
    Junta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Netlist {
    pub nombre: String,
    pub nodos: Vec<Nodo>,
    pub instancias: Vec<Instancia>,
    /// Cada `R(dir)` es una memoria nueva, con su copia del molde
    pub memorias: Vec<Molde>,
}

/// De dónde sale un bit mientras se elabora. Los nombres se resuelven al
/// final, así el orden no importa y la realimentación funciona.
#[derive(Debug, Clone)]
enum Ref {
    Nodo(usize),
    /// Un bit con nombre (`A`, `A[3]`, `fa[2].Cin`) dentro de un ámbito
    Senal(usize, String, Range<usize>),
    /// Un bit de una salida de una instancia: ámbito, instancia (`m`, `fa[2]`) y bit (`Suma`, `S[0]`)
    Miembro(usize, String, String, Range<usize>),
}

/// Valores de las variables de los `for` que encierran una sentencia
type Variables<'a> = Vec<(&'a str, i64)>;

/// Los nombres de una copia de un circuito (el principal o una instancia)
struct Ambito<'a> {
    circuito: &'a Circuito,
    /// Cada bit con nombre → de dónde sale
    senales: HashMap<String, Ref>,
    /// Ancho de cada señal: `None` = un bit, `Some(n)` = bus de n bits
    anchos: HashMap<String, Option<usize>>,
    /// Instancia (`m`, `fa[2]`) → su circuito, conocido antes de copiarla
    tipos: HashMap<String, &'a Circuito>,
    /// Arreglo de instancias (`fa`) → cantidad
    arreglos: HashMap<String, usize>,
    /// Instancia ya copiada → su ámbito
    instancias: HashMap<String, usize>,
    /// `R = rom.size(...);` → el molde
    moldes: HashMap<String, &'a Molde>,
    /// Nombres de los `led`, `display7`...
    visores: HashSet<String>,
}

struct Elaborador<'a> {
    circuitos: HashMap<&'a str, &'a Circuito>,
    ambitos: Vec<Ambito<'a>>,
    nodos: Vec<Nodo>,
    /// Paralelo a `nodos`: de dónde sale cada una de sus entradas
    pendientes: Vec<Vec<Ref>>,
    errores: Vec<Error>,
    /// Circuitos que se están copiando, para detectar uno que se contiene a sí mismo
    pila: Vec<&'a str>,
    /// Instancia del principal que se está copiando
    actual: Option<usize>,
    /// Cajas del principal (instancias, `split`, memorias y visores), con sus pines sin resolver
    instancias: Vec<Caja>,
    memorias: Vec<Molde>,
    /// Instancias (ámbito, nombre) que no se pudieron crear: ya tienen su error y
    /// sus usos (`m.Suma`) no lo repiten
    fallidas: HashSet<(usize, String)>,
}

struct Caja {
    nombre: String,
    circuito: String,
    entradas: Vec<(String, Ref)>,
    salidas: Vec<(String, Ref)>,
    split: Option<Split>,
}

/// Más vueltas de `for` o copias en un arreglo seguramente es un error de tipeo
const MAXIMO: i64 = 1024;

fn error(mensaje: String, rango: Range<usize>) -> Error {
    Error { mensaje, rango }
}

/// "1 bit", "8 bits"
fn en_bits(n: usize) -> String {
    if n == 1 {
        "1 bit".into()
    } else {
        format!("{n} bits")
    }
}

/// Nombres de los bits de una señal: `A`, o `A[0]`, `A[1]`, ... si es un bus.
fn bits(base: &str, ancho: Option<usize>) -> Vec<String> {
    match ancho {
        None => vec![base.to_string()],
        Some(n) => (0..n).map(|k| format!("{base}[{k}]")).collect(),
    }
}

/// Los bits de todos los puertos, en orden (como se pasan a una instancia)
fn bits_de(puertos: &[Puerto]) -> Vec<String> {
    puertos
        .iter()
        .flat_map(|p| bits(&p.nombre.texto, p.ancho))
        .collect()
}

/// El nombre más parecido a `n` (a 2 letras o menos, sin contar mayúsculas), para
/// "¿quisiste decir ...?"
fn parecido<'n>(n: &str, nombres: impl Iterator<Item = &'n str>) -> Option<&'n str> {
    let distancia = |a: &str, b: &str| {
        let (a, b): (Vec<char>, Vec<char>) = (
            a.to_lowercase().chars().collect(),
            b.to_lowercase().chars().collect(),
        );
        // Levenshtein con una sola fila
        let mut fila: Vec<usize> = (0..=b.len()).collect();
        for (i, ca) in a.iter().enumerate() {
            let mut diagonal = fila[0];
            fila[0] = i + 1;
            for (j, cb) in b.iter().enumerate() {
                let arriba = fila[j + 1];
                fila[j + 1] = (diagonal + usize::from(ca != cb))
                    .min(arriba + 1)
                    .min(fila[j] + 1);
                diagonal = arriba;
            }
        }
        fila[b.len()]
    };
    nombres
        .filter(|&x| x != n)
        .map(|x| (distancia(n, x), x))
        .filter(|&(d, x)| d <= 2 && d < n.len().min(x.len()))
        .min()
        .map(|(_, x)| x)
}

/// "`X` no existe", con "¿quisiste decir ...?" si alguno de `nombres` se parece
fn no_existe<'n>(n: &str, nombres: impl Iterator<Item = &'n str>) -> String {
    match parecido(n, nombres) {
        Some(x) => format!("`{n}` no existe; ¿quisiste decir `{x}`?"),
        None => format!("`{n}` no existe: declárala con `in` o asígnale un valor"),
    }
}

fn puerto<'c>(puertos: &'c [Puerto], nombre: &str) -> Option<&'c Puerto> {
    puertos.iter().find(|p| p.nombre.texto == nombre)
}

fn evaluar(e: &IExpr, vars: &Variables) -> Result<i64, Error> {
    let mut total: i64 = 0;
    for (positivo, t) in &e.terminos {
        let v = match t {
            Termino::Numero(n) => i64::try_from(*n).unwrap_or(i64::MAX),
            Termino::Variable(n) => match vars.iter().rev().find(|(v, _)| *v == n.texto) {
                Some(&(_, v)) => v,
                None => {
                    return Err(error(
                        format!(
                            "`{}` no es una variable de `for`: en los índices solo van números y la variable del `for`",
                            n.texto
                        ),
                        n.rango.clone(),
                    ));
                }
            },
        };
        total = if *positivo {
            total.saturating_add(v)
        } else {
            total.saturating_sub(v)
        };
    }
    Ok(total)
}

/// Los bits que elige un índice (`A`, `A[3]`, `A[0..3]`), del más bajo al más alto.
fn elegir(
    base: &str,
    ancho: Option<usize>,
    indice: Option<&Indice>,
    vars: &Variables,
) -> Result<Vec<String>, Error> {
    let Some(ix) = indice else {
        return Ok(bits(base, ancho));
    };
    let Some(n) = ancho else {
        return Err(error(
            format!("`{base}` es de un bit: no lleva índice"),
            ix.rango.clone(),
        ));
    };
    let desde = evaluar(&ix.desde, vars)?;
    let hasta = match &ix.hasta {
        Some(h) => evaluar(h, vars)?,
        None => desde,
    };
    for k in [desde, hasta] {
        if !(0..n as i64).contains(&k) {
            return Err(error(
                format!(
                    "`{base}` tiene {}: el índice {k} no existe (van de 0 a {})",
                    en_bits(n),
                    n - 1
                ),
                ix.rango.clone(),
            ));
        }
    }
    let posiciones: Vec<i64> = if desde <= hasta {
        (desde..=hasta).collect()
    } else {
        (hasta..=desde).rev().collect()
    };
    Ok(posiciones
        .into_iter()
        .map(|k| format!("{base}[{k}]"))
        .collect())
}

/// Cómo se usa una memoria: `R(dirección)` o `M(dirección, dato, escribir, reloj)`
fn uso(m: &Molde) -> &'static str {
    if m.ram {
        "dirección, dato, escribir, reloj"
    } else {
        "dirección"
    }
}

/// Algún nombre dentro de la expresión, para ubicar un error
fn rango_de(e: &Expr) -> Range<usize> {
    match e {
        Expr::Senal(a) => a.nombre.rango.clone(),
        Expr::Instancia(n, _) => n.rango.clone(),
        Expr::Split(_, _, rango) | Expr::Molde(_, rango) => rango.clone(),
        Expr::Compuerta(_, partes) => partes
            .iter()
            .map(rango_de)
            .find(|r| !r.is_empty())
            .unwrap_or(0..0),
        Expr::Constante(_) => 0..0,
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
        memorias: Vec::new(),
        fallidas: HashSet::new(),
    };
    for c in circuitos {
        if e.circuitos.insert(&c.nombre.texto, c).is_some() {
            e.agregar(error(
                format!("el circuito `{}` está definido dos veces", c.nombre.texto),
                c.nombre.rango.clone(),
            ));
        }
    }

    let raiz = e.ambito(principal, None);
    for p in &principal.salidas {
        for b in bits(&p.nombre.texto, p.ancho) {
            if e.ambitos[raiz].senales.contains_key(&b) {
                let r = Ref::Senal(raiz, b.clone(), p.nombre.rango.clone());
                e.nodo(Tipo::Salida(b), vec![r]);
            }
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
    for caja in std::mem::take(&mut e.instancias) {
        let mut entradas = Vec::new();
        for (pin, f) in caja.entradas {
            match e.resolver(f) {
                Ok(id) => entradas.push((pin, id)),
                Err(err) => e.agregar(err),
            }
        }
        // Una salida sin valor ya tiene su error ("nunca recibe un valor")
        let salidas = caja
            .salidas
            .into_iter()
            .filter_map(|(pin, f)| Some((pin, e.resolver(f).ok()?)))
            .collect();
        instancias.push(Instancia {
            nombre: caja.nombre,
            circuito: caja.circuito,
            entradas,
            salidas,
            split: caja.split,
        });
    }

    if e.errores.is_empty() {
        Ok(Netlist {
            nombre: principal.nombre.texto.clone(),
            nodos: e.nodos,
            instancias,
            memorias: e.memorias,
        })
    } else {
        Err(e.errores)
    }
}

impl<'a> Elaborador<'a> {
    /// Un circuito usado varias veces (o un `for`) repetiría sus errores: se guardan una sola vez.
    /// Los errores sin mensaje son usos de una instancia que falló: no se muestran.
    fn agregar(&mut self, err: Error) {
        if !err.mensaje.is_empty() && !self.errores.contains(&err) {
            self.errores.push(err);
        }
    }

    fn nodo(&mut self, tipo: Tipo, fuentes: Vec<Ref>) -> usize {
        let nombre = match &tipo {
            Tipo::Entrada(n, _) | Tipo::Salida(n) | Tipo::Visor(_, n) => n.clone(),
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

    /// El circuito `n` del proyecto, si existe y no se está copiando ya.
    fn circuito(&mut self, n: &Nombre) -> Option<&'a Circuito> {
        let Some(&c) = self.circuitos.get(n.texto.as_str()) else {
            let mensaje = match parecido(&n.texto, self.circuitos.keys().copied()) {
                Some(x) => format!(
                    "`{}` no es un circuito del proyecto; ¿quisiste decir `{x}`?",
                    n.texto
                ),
                None => format!("`{}` no es un circuito del proyecto", n.texto),
            };
            self.agregar(error(mensaje, n.rango.clone()));
            return None;
        };
        if self.pila.contains(&c.nombre.texto.as_str()) {
            self.agregar(error(
                format!("`{}` se usa dentro de sí mismo", n.texto),
                n.rango.clone(),
            ));
            return None;
        }
        Some(c)
    }

    /// Crea una copia de `c`. `entradas` son los bits que le llegan; `None` es
    /// el circuito principal, cuyas entradas son interruptores.
    fn ambito(&mut self, c: &'a Circuito, entradas: Option<Vec<Ref>>) -> usize {
        let id = self.ambitos.len();
        self.ambitos.push(Ambito {
            circuito: c,
            senales: HashMap::new(),
            anchos: HashMap::new(),
            tipos: HashMap::new(),
            arreglos: HashMap::new(),
            instancias: HashMap::new(),
            moldes: HashMap::new(),
            visores: HashSet::new(),
        });
        self.pila.push(&c.nombre.texto);

        let mut k = 0;
        for p in &c.entradas {
            let n = &p.nombre.texto;
            let bs = bits(n, p.ancho);
            if self.ambitos[id].anchos.contains_key(n) {
                k += bs.len();
                self.agregar(error(
                    format!("`{n}` está declarada dos veces"),
                    p.nombre.rango.clone(),
                ));
                continue;
            }
            for b in bs {
                let r = match &entradas {
                    Some(refs) => refs[k].clone(),
                    None => Ref::Nodo(self.nodo(Tipo::Entrada(b.clone(), p.clase), Vec::new())),
                };
                k += 1;
                self.ambitos[id].senales.insert(b, r);
            }
            self.ambitos[id].anchos.insert(n.clone(), p.ancho);
        }
        for p in &c.salidas {
            let n = &p.nombre.texto;
            if self.ambitos[id].anchos.insert(n.clone(), p.ancho).is_some() {
                self.agregar(error(
                    format!("`{n}` está declarada dos veces"),
                    p.nombre.rango.clone(),
                ));
            }
        }

        let mut planas = Vec::new();
        self.desenrollar(&c.sentencias, &mut Vec::new(), &mut planas);
        self.declarar_moldes(id, &planas);
        self.declarar_instancias(id, &planas);
        self.inferir_anchos(id, &planas);
        // Una asignación con error no repite el aviso de "nunca recibe un valor"
        let mut fallidas = HashSet::new();
        for (s, vars) in &planas {
            match *s {
                Sentencia::Asignar(destino, valor) => {
                    if let Err(err) = self.asignar(id, destino, valor, vars) {
                        self.agregar(err);
                        fallidas.insert(destino.nombre.texto.as_str());
                    }
                }
                Sentencia::Visor {
                    tipo,
                    nombre,
                    senales,
                } => {
                    if let Err(err) = self.visor(id, *tipo, nombre, senales, vars) {
                        self.agregar(err);
                    }
                }
                _ => {}
            }
        }

        for p in &c.salidas {
            let bs = bits(&p.nombre.texto, p.ancho);
            let senales = &self.ambitos[id].senales;
            let faltan: Vec<String> = bs
                .iter()
                .filter(|b| !senales.contains_key(*b))
                .cloned()
                .collect();
            let Some(primera) = faltan.first() else {
                continue;
            };
            if !fallidas.contains(p.nombre.texto.as_str()) {
                let falta = if faltan.len() == bs.len() {
                    &p.nombre.texto
                } else {
                    primera
                };
                let mensaje = format!("la salida `{falta}` nunca recibe un valor");
                self.agregar(error(mensaje, p.nombre.rango.clone()));
            }
            // Relleno para que quien la lea no repita el error (igual no compila)
            for b in faltan {
                let r = Ref::Nodo(self.nodo(Tipo::Constante(false), Vec::new()));
                self.ambitos[id].senales.insert(b, r);
            }
        }
        self.pila.pop();
        id
    }

    /// Saca los `for`: cada sentencia queda con los valores de sus variables.
    fn desenrollar(
        &mut self,
        sentencias: &'a [Sentencia],
        vars: &mut Variables<'a>,
        planas: &mut Vec<(&'a Sentencia, Variables<'a>)>,
    ) {
        for s in sentencias {
            let Sentencia::For {
                variable,
                desde,
                hasta,
                cuerpo,
            } = s
            else {
                planas.push((s, vars.clone()));
                continue;
            };
            let (desde, hasta) = match (evaluar(desde, vars), evaluar(hasta, vars)) {
                (Ok(d), Ok(h)) => (d, h),
                (Err(err), _) | (_, Err(err)) => {
                    self.agregar(err);
                    continue;
                }
            };
            if hasta.saturating_sub(desde) > MAXIMO {
                self.agregar(error(
                    format!("el `for` da demasiadas vueltas (máximo {MAXIMO})"),
                    variable.rango.clone(),
                ));
                continue;
            }
            for v in desde..hasta {
                vars.push((&variable.texto, v));
                self.desenrollar(cuerpo, vars, planas);
                vars.pop();
            }
        }
    }

    /// Anota los moldes de memoria (`R = rom.size(...);`) antes que nada: `R(dir)` se
    /// puede usar en cualquier orden.
    fn declarar_moldes(&mut self, id: usize, planas: &[(&'a Sentencia, Variables<'a>)]) {
        for (s, _) in planas {
            let Sentencia::Asignar(d, Expr::Molde(m, _)) = *s else {
                continue;
            };
            let n = &d.nombre.texto;
            let a = &self.ambitos[id];
            let mensaje = if d.indice.is_some() || d.miembro.is_some() {
                "un molde se guarda en un nombre simple, como `R = rom.size(...);`".to_string()
            } else if a.moldes.contains_key(n) || a.anchos.contains_key(n) {
                format!("`{n}` ya tiene un valor: cada nombre se usa una sola vez")
            } else {
                self.ambitos[id].moldes.insert(n.clone(), m);
                continue;
            };
            self.agregar(error(mensaje, d.nombre.rango.clone()));
        }
    }

    /// Anota las instancias antes que nada, así se puede leer `m.Salida` en
    /// cualquier orden. Los arreglos se copian aquí mismo: sus entradas llegan
    /// después con `fa[i].A = ...`.
    fn declarar_instancias(&mut self, id: usize, planas: &[(&'a Sentencia, Variables<'a>)]) {
        for (s, vars) in planas {
            let (circuito, nombre, cantidad) = match *s {
                Sentencia::Arreglo {
                    circuito,
                    nombre,
                    cantidad,
                } => (circuito, nombre, Some(cantidad)),
                Sentencia::Asignar(d, Expr::Instancia(circuito, _))
                    if d.indice.is_none()
                        && d.miembro.is_none()
                        && !self.ambitos[id].moldes.contains_key(&circuito.texto) =>
                {
                    (circuito, &d.nombre, None)
                }
                _ => continue,
            };
            let a = &self.ambitos[id];
            let n = &nombre.texto;
            if a.anchos.contains_key(n)
                || a.tipos.contains_key(n)
                || a.arreglos.contains_key(n)
                || a.moldes.contains_key(n)
            {
                self.agregar(error(
                    format!("`{n}` ya tiene un valor: cada nombre se usa una sola vez"),
                    nombre.rango.clone(),
                ));
                continue;
            }
            let Some(c) = self.circuito(circuito) else {
                self.fallidas.insert((id, n.clone()));
                continue;
            };
            let Some(cantidad) = cantidad else {
                self.ambitos[id].tipos.insert(n.clone(), c);
                continue;
            };
            let cantidad = match evaluar(cantidad, vars) {
                Ok(k) if (1..=MAXIMO).contains(&k) => k as usize,
                Ok(_) => {
                    self.agregar(error(
                        format!("un arreglo tiene de 1 a {MAXIMO} circuitos"),
                        cantidad.rango.clone(),
                    ));
                    continue;
                }
                Err(err) => {
                    self.agregar(err);
                    continue;
                }
            };
            self.ambitos[id].arreglos.insert(n.clone(), cantidad);
            for k in 0..cantidad {
                let clave = format!("{n}[{k}]");
                let entradas = bits_de(&c.entradas)
                    .into_iter()
                    .map(|b| Ref::Senal(id, format!("{clave}.{b}"), nombre.rango.clone()))
                    .collect();
                self.ambitos[id].tipos.insert(clave.clone(), c);
                let hijo = self.copiar(id, clave.clone(), c, entradas);
                self.ambitos[id].instancias.insert(clave, hijo);
            }
        }
    }

    /// El ancho de las señales internas sale de lo que se les asigna:
    /// `t[3] = ...` hace un bus de 4 bits; `x = and(A, B)` toma el ancho de A y B.
    fn inferir_anchos(&mut self, id: usize, planas: &[(&'a Sentencia, Variables<'a>)]) {
        let mut pendientes = Vec::new();
        for (s, vars) in planas {
            let Sentencia::Asignar(d, valor) = s else {
                continue;
            };
            let a = &self.ambitos[id];
            let n = &d.nombre.texto;
            if d.miembro.is_some()
                || matches!(valor, Expr::Molde(..))
                || matches!(valor, Expr::Instancia(c, _) if !a.moldes.contains_key(&c.texto))
                || a.tipos.contains_key(n)
                || a.arreglos.contains_key(n)
                || puerto(&a.circuito.entradas, n).is_some()
                || puerto(&a.circuito.salidas, n).is_some()
            {
                continue;
            }
            match &d.indice {
                None => pendientes.push((n, valor, vars)),
                Some(ix) => {
                    let alto = [Some(&ix.desde), ix.hasta.as_ref()]
                        .into_iter()
                        .flatten()
                        .filter_map(|e| evaluar(e, vars).ok())
                        .max()
                        .unwrap_or(-1);
                    if (0..64).contains(&alto) {
                        let ancho = self.ambitos[id].anchos.entry(n.clone()).or_insert(Some(1));
                        *ancho = Some(ancho.unwrap_or(1).max(alto as usize + 1));
                    }
                }
            }
        }
        pendientes.retain(|(n, ..)| !self.ambitos[id].anchos.contains_key(*n));

        // Se repite hasta que todo tenga ancho. Lo que depende de sí mismo
        // (realimentación) se supone de 1 bit.
        // ponytail: un lazo de buses sin ningún ancho conocido queda de 1 bit.
        let mut suponer = false;
        while !pendientes.is_empty() {
            let antes = pendientes.len();
            let mut i = 0;
            while i < pendientes.len() {
                let (n, valor, vars) = pendientes[i];
                match self.ancho(id, valor, vars, suponer) {
                    Some(w) => {
                        let ancho = if w == 1 { None } else { Some(w) };
                        self.ambitos[id].anchos.entry(n.clone()).or_insert(ancho);
                        pendientes.remove(i);
                    }
                    None => i += 1,
                }
            }
            suponer = pendientes.len() == antes;
        }
    }

    /// Cuántos bits da una expresión; `None` si usa una señal de ancho desconocido.
    fn ancho(&self, id: usize, e: &Expr, vars: &Variables, suponer: bool) -> Option<usize> {
        match e {
            Expr::Constante(_) | Expr::Molde(..) => Some(1),
            Expr::Instancia(c, _) => {
                Some(self.ambitos[id].moldes.get(&c.texto).map_or(1, |m| m.dato))
            }
            Expr::Senal(a) => {
                let ambito = &self.ambitos[id];
                if a.miembro.is_none() && !ambito.anchos.contains_key(&a.nombre.texto) {
                    return suponer.then_some(1);
                }
                // Si tiene un error se avisará al construirla
                Some(self.leer(id, a, vars).map_or(1, |r| r.len()))
            }
            Expr::Compuerta(_, partes) => partes
                .iter()
                .map(|p| self.ancho(id, p, vars, suponer))
                .try_fold(1, |m, w| Some(m.max(w?))),
            Expr::Split(_, partes, _) => partes
                .iter()
                .map(|p| self.ancho(id, p, vars, suponer))
                .sum(),
        }
    }

    /// `m` o `fa[2]`: el nombre de la instancia y su circuito
    fn instancia(
        &self,
        id: usize,
        nombre: &Nombre,
        indice: Option<&Indice>,
        vars: &Variables,
    ) -> Result<(String, &'a Circuito), Error> {
        let a = &self.ambitos[id];
        let n = &nombre.texto;
        let clave = match (a.arreglos.get(n), indice) {
            (Some(&cantidad), None) => {
                return Err(error(
                    format!(
                        "`{n}` es un arreglo de {cantidad} circuitos: elige uno, como `{n}[0]`"
                    ),
                    nombre.rango.clone(),
                ));
            }
            (Some(&cantidad), Some(ix)) => {
                if ix.hasta.is_some() {
                    return Err(error(
                        format!("elige un solo circuito del arreglo, como `{n}[0]`"),
                        ix.rango.clone(),
                    ));
                }
                let k = evaluar(&ix.desde, vars)?;
                if !(0..cantidad as i64).contains(&k) {
                    return Err(error(
                        format!(
                            "`{n}` tiene {cantidad} circuitos: el índice {k} no existe (van de 0 a {})",
                            cantidad - 1
                        ),
                        ix.rango.clone(),
                    ));
                }
                format!("{n}[{k}]")
            }
            (None, Some(ix)) if a.tipos.contains_key(n) => {
                return Err(error(
                    format!("`{n}` es un solo circuito: no lleva índice"),
                    ix.rango.clone(),
                ));
            }
            (None, _) => n.clone(),
        };
        match a.tipos.get(&clave) {
            Some(&c) => Ok((clave, c)),
            None if a.anchos.contains_key(n) => Err(error(
                format!(
                    "`{n}` es una señal, no un circuito: úsala sin punto, como `{n}` o `{n}[0]`"
                ),
                nombre.rango.clone(),
            )),
            // Ya tiene su error donde se declaró
            None if self.fallidas.contains(&(id, n.clone())) => {
                Err(error(String::new(), nombre.rango.clone()))
            }
            None => Err(error(
                format!("`{n}` no es un circuito"),
                nombre.rango.clone(),
            )),
        }
    }

    /// Los bits que se leen con `A`, `A[3]`, `m.Suma` o `fa[i].S[0..3]`
    fn leer(&self, id: usize, a: &Acceso, vars: &Variables) -> Result<Vec<Ref>, Error> {
        let n = &a.nombre.texto;
        let rango = a.nombre.rango.clone();
        if let Some((salida, indice)) = &a.miembro {
            let (clave, c) = self.instancia(id, &a.nombre, a.indice.as_deref(), vars)?;
            let Some(p) = puerto(&c.salidas, &salida.texto) else {
                let mensaje = if puerto(&c.entradas, &salida.texto).is_some() {
                    format!(
                        "`{}` es una entrada de `{}`: solo se leen sus salidas",
                        salida.texto, c.nombre.texto
                    )
                } else {
                    let nombres = c.salidas.iter().map(|p| p.nombre.texto.as_str());
                    match parecido(&salida.texto, nombres) {
                        Some(x) => format!(
                            "`{}` no tiene la salida `{}`; ¿quisiste decir `{x}`?",
                            c.nombre.texto, salida.texto
                        ),
                        None => {
                            format!("`{}` no tiene la salida `{}`", c.nombre.texto, salida.texto)
                        }
                    }
                };
                return Err(error(mensaje, salida.rango.clone()));
            };
            let bs = elegir(&salida.texto, p.ancho, indice.as_deref(), vars)?;
            return Ok(bs
                .into_iter()
                .map(|b| Ref::Miembro(id, clave.clone(), b, rango.clone()))
                .collect());
        }
        let ambito = &self.ambitos[id];
        if let Some(&ancho) = ambito.anchos.get(n) {
            let bs = elegir(n, ancho, a.indice.as_deref(), vars)?;
            return Ok(bs
                .into_iter()
                .map(|b| Ref::Senal(id, b, rango.clone()))
                .collect());
        }
        let mensaje = if ambito.tipos.contains_key(n) || ambito.arreglos.contains_key(n) {
            format!("`{n}` es un circuito, no una señal: usa una de sus salidas, como `{n}.Salida`")
        } else if let Some(m) = ambito.moldes.get(n) {
            format!("`{n}` es un molde de memoria: úsalo con `{n}({})`", uso(m))
        } else {
            no_existe(n, ambito.anchos.keys().map(String::as_str))
        };
        Err(error(mensaje, rango))
    }

    /// Los bits a los que se asigna: `X`, `S[3]`, `S[0..3]` o `fa[i].Cin`
    fn destinos(&self, id: usize, d: &Acceso, vars: &Variables) -> Result<Vec<String>, Error> {
        let a = &self.ambitos[id];
        let n = &d.nombre.texto;
        if let Some((entrada, indice)) = &d.miembro {
            let (clave, c) = self.instancia(id, &d.nombre, d.indice.as_deref(), vars)?;
            if !a.arreglos.contains_key(n) {
                return Err(error(
                    format!(
                        "las entradas de `{n}` ya se conectaron al crearlo con `{n} = {}(...)`",
                        c.nombre.texto
                    ),
                    entrada.rango.clone(),
                ));
            }
            let Some(p) = puerto(&c.entradas, &entrada.texto) else {
                let mensaje = if puerto(&c.salidas, &entrada.texto).is_some() {
                    format!(
                        "`{}` es una salida de `{}`: no se le asigna un valor",
                        entrada.texto, c.nombre.texto
                    )
                } else {
                    format!(
                        "`{}` no tiene la entrada `{}`",
                        c.nombre.texto, entrada.texto
                    )
                };
                return Err(error(mensaje, entrada.rango.clone()));
            };
            let bs = elegir(&entrada.texto, p.ancho, indice.as_deref(), vars)?;
            return Ok(bs.into_iter().map(|b| format!("{clave}.{b}")).collect());
        }
        let mensaje = if puerto(&a.circuito.entradas, n).is_some() {
            "es una entrada: no se le puede asignar un valor"
        } else if a.tipos.contains_key(n) || a.arreglos.contains_key(n) || a.moldes.contains_key(n)
        {
            "ya tiene un valor: cada nombre se usa una sola vez"
        } else {
            let ancho = a.anchos.get(n).copied().flatten();
            return elegir(n, ancho, d.indice.as_deref(), vars);
        };
        Err(error(format!("`{n}` {mensaje}"), d.nombre.rango.clone()))
    }

    fn asignar(
        &mut self,
        id: usize,
        d: &'a Acceso,
        valor: &'a Expr,
        vars: &Variables<'a>,
    ) -> Result<(), Error> {
        // Ya se anotó en `declarar_moldes`
        if let Expr::Molde(..) = valor {
            return Ok(());
        }
        if let Expr::Instancia(circuito, args) = valor
            && !self.ambitos[id].moldes.contains_key(&circuito.texto)
        {
            if d.indice.is_some() || d.miembro.is_some() {
                return Err(error(
                    format!(
                        "un circuito se guarda en un nombre simple, como `m = {0}(...);`; para varias copias usa `{0} m[8];`",
                        circuito.texto
                    ),
                    circuito.rango.clone(),
                ));
            }
            let n = &d.nombre.texto;
            if self.ambitos[id].instancias.contains_key(n) {
                return Err(error(
                    format!("`{n}` ya tiene un valor: cada nombre se usa una sola vez"),
                    d.nombre.rango.clone(),
                ));
            }
            // Si no está anotada, ya se avisó por qué
            let Some(&c) = self.ambitos[id].tipos.get(n) else {
                self.fallidas.insert((id, n.clone()));
                return Ok(());
            };
            let creada = self.instanciar(id, n.clone(), c, circuito, args, vars);
            if creada.is_err() {
                self.fallidas.insert((id, n.clone()));
            }
            return creada;
        }

        let antes = self.nodos.len();
        let fuentes = self.construir(id, valor, vars)?;
        let destinos = self.destinos(id, d, vars)?;
        if fuentes.len() != destinos.len() {
            return Err(error(
                format!(
                    "`{}` es de {}, pero recibe {}",
                    d.nombre.texto,
                    en_bits(destinos.len()),
                    en_bits(fuentes.len())
                ),
                d.nombre.rango.clone(),
            ));
        }
        for (b, r) in destinos.into_iter().zip(fuentes) {
            if let Some(viejo) = self.ambitos[id].senales.get(&b).cloned() {
                self.otro_conductor(id, b, viejo, r, &d.nombre.rango)?;
                continue;
            }
            if let Ref::Nodo(i) = r
                && i >= antes
                && self.nodos[i].nombre.is_empty()
            {
                self.nodos[i].nombre = b.clone();
            }
            self.ambitos[id].senales.insert(b, r);
        }
        Ok(())
    }

    /// Una señal que ya tiene valor solo admite más `tristate`: se juntan en un bus.
    fn otro_conductor(
        &mut self,
        id: usize,
        b: String,
        viejo: Ref,
        nuevo: Ref,
        rango: &Range<usize>,
    ) -> Result<(), Error> {
        let tristate = |t: &Tipo| matches!(t, Tipo::Compuerta(Compuerta::Tristate));
        let par = match (&viejo, &nuevo) {
            (Ref::Nodo(v), Ref::Nodo(n)) => Some((*v, *n)),
            _ => None,
        };
        let Some((v, _)) = par.filter(|&(_, n)| tristate(&self.nodos[n].tipo)) else {
            return Err(error(
                format!("`{b}` ya tiene un valor: cada señal se asigna una sola vez"),
                rango.clone(),
            ));
        };
        if self.nodos[v].tipo == Tipo::Bus {
            self.pendientes[v].push(nuevo);
        } else if tristate(&self.nodos[v].tipo) {
            let bus = self.nodo(Tipo::Bus, vec![Ref::Nodo(v), nuevo]);
            self.nodos[bus].nombre = std::mem::take(&mut self.nodos[v].nombre);
            self.ambitos[id].senales.insert(b, Ref::Nodo(bus));
        } else {
            return Err(error(
                format!(
                    "`{b}` ya tiene un valor: solo varios `tristate` pueden manejar la misma señal"
                ),
                rango.clone(),
            ));
        }
        Ok(())
    }

    /// `led L(Q);`, `display7 D(...);`, `matrix M(f, c, ...);` o `probe P(...);`.
    /// En el principal (menos el LED) se dibujan como una caja con sus pines.
    fn visor(
        &mut self,
        id: usize,
        tipo: Visor,
        nombre: &Nombre,
        senales: &'a [Expr],
        vars: &Variables,
    ) -> Result<(), Error> {
        let n = &nombre.texto;
        let a = &self.ambitos[id];
        if a.anchos.contains_key(n)
            || a.tipos.contains_key(n)
            || a.arreglos.contains_key(n)
            || a.moldes.contains_key(n)
            || a.visores.contains(n)
        {
            return Err(error(
                format!("`{n}` ya tiene un valor: cada nombre se usa una sola vez"),
                nombre.rango.clone(),
            ));
        }
        self.ambitos[id].visores.insert(n.clone());
        let mut bits = Vec::new();
        for s in senales {
            bits.extend(self.construir(id, s, vars)?);
        }
        let (esperados, pines): (usize, Vec<String>) = match tipo {
            Visor::Led => (1, vec![String::new()]),
            Visor::Display7 => (
                8,
                ["a", "b", "c", "d", "e", "f", "g", "dp"]
                    .map(String::from)
                    .to_vec(),
            ),
            Visor::Matrix(f, c) => (
                f + c,
                (0..f)
                    .map(|k| format!("F{k}"))
                    .chain((0..c).map(|k| format!("C{k}")))
                    .collect(),
            ),
            Visor::Probe => (bits.len(), (0..bits.len()).map(|k| k.to_string()).collect()),
        };
        if bits.len() != esperados {
            let detalle = match tipo {
                Visor::Display7 => ": a, b, c, d, e, f, g y dp".to_string(),
                Visor::Matrix(f, c) => format!(": {f} filas y {c} columnas"),
                _ => String::new(),
            };
            return Err(error(
                format!(
                    "`{}` lleva {}{detalle}, pero recibe {}",
                    tipo.nombre(),
                    en_bits(esperados),
                    en_bits(bits.len())
                ),
                nombre.rango.clone(),
            ));
        }
        let caja = id == 0 && tipo != Visor::Led;
        let g = self.instancias.len();
        if caja {
            self.actual = Some(g);
        }
        self.nodo(Tipo::Visor(tipo, n.clone()), bits.clone());
        if caja {
            self.actual = None;
            self.instancias.push(Caja {
                nombre: n.clone(),
                circuito: tipo.nombre().into(),
                entradas: pines.into_iter().zip(bits).collect(),
                salidas: Vec::new(),
                split: None,
            });
        }
        Ok(())
    }

    /// `R(dir)` o `M(dir, dato, escribir, reloj)`: una memoria nueva con el molde `m`.
    /// Da los bits del dato leído.
    fn memoria(
        &mut self,
        id: usize,
        nombre: &Nombre,
        m: &Molde,
        args: &'a [Expr],
        vars: &Variables,
    ) -> Result<Vec<Ref>, Error> {
        let mut pines = vec![("A", m.direccion, "la dirección")];
        if m.ram {
            pines.extend([
                ("D", m.dato, "el dato"),
                ("W", 1, "`escribir`"),
                ("C", 1, "el reloj"),
            ]);
        }
        let n = &nombre.texto;
        if args.len() != pines.len() {
            return Err(error(
                format!("`{n}` se usa con `{n}({})`", uso(m)),
                nombre.rango.clone(),
            ));
        }
        let mut entradas = Vec::new();
        for (&(pin, ancho, que), arg) in pines.iter().zip(args) {
            let bits = self.construir(id, arg, vars)?;
            if bits.len() != ancho {
                return Err(error(
                    format!(
                        "{que} de `{n}` es de {}, pero recibe {}",
                        en_bits(ancho),
                        en_bits(bits.len())
                    ),
                    rango_de(arg),
                ));
            }
            for (k, b) in bits.into_iter().enumerate() {
                let pin = if ancho == 1 {
                    pin.to_string()
                } else {
                    format!("{pin}{k}")
                };
                entradas.push((pin, b));
            }
        }
        let memoria = self.memorias.len();
        self.memorias.push(m.clone());
        let fuentes: Vec<Ref> = entradas.iter().map(|e| e.1.clone()).collect();
        // En el principal se dibuja como una caja con los bits del dato a la derecha
        let caja = id == 0;
        let g = self.instancias.len();
        if caja {
            self.actual = Some(g);
        }
        let salidas: Vec<Ref> = (0..m.dato)
            .map(|bit| Ref::Nodo(self.nodo(Tipo::Memoria { memoria, bit }, fuentes.clone())))
            .collect();
        if caja {
            self.actual = None;
            self.instancias.push(Caja {
                // ponytail: se ubica por orden, como el split
                nombre: format!("{n}{g}"),
                circuito: String::from(if m.ram { "ram" } else { "rom" }),
                entradas,
                salidas: (0..m.dato)
                    .map(|k| format!("D{k}"))
                    .zip(salidas.clone())
                    .collect(),
                split: None,
            });
        }
        Ok(salidas)
    }

    /// `m = MedioSumador(A, B);` dentro del ámbito `padre`
    fn instanciar(
        &mut self,
        padre: usize,
        nombre: String,
        c: &'a Circuito,
        circuito: &Nombre,
        args: &'a [Expr],
        vars: &Variables<'a>,
    ) -> Result<(), Error> {
        if args.len() != c.entradas.len() {
            let plural = if c.entradas.len() == 1 {
                "entrada"
            } else {
                "entradas"
            };
            // `in Q[4]` es una sola entrada: se le pasa un bus entero, no sus bits sueltos
            let pista = match c.entradas.iter().find(|p| p.ancho.is_some()) {
                Some(p) if args.len() > c.entradas.len() => format!(
                    ". `{}` es una sola entrada de {}: pásale un bus entero, como `X` o `X[0..{}]`",
                    p.nombre.texto,
                    en_bits(p.ancho.unwrap_or(1)),
                    p.ancho.unwrap_or(1) - 1
                ),
                _ => String::new(),
            };
            return Err(error(
                format!(
                    "`{}` lleva {} {plural}, pero tiene {}{pista}",
                    circuito.texto,
                    c.entradas.len(),
                    args.len()
                ),
                circuito.rango.clone(),
            ));
        }
        let mut entradas = Vec::new();
        for (p, arg) in c.entradas.iter().zip(args) {
            let bits = self.construir(padre, arg, vars)?;
            let ancho = p.ancho.unwrap_or(1);
            if bits.len() != ancho {
                return Err(error(
                    format!(
                        "la entrada `{}` de `{}` es de {}, pero recibe {}",
                        p.nombre.texto,
                        circuito.texto,
                        en_bits(ancho),
                        en_bits(bits.len())
                    ),
                    rango_de(arg),
                ));
            }
            entradas.extend(bits);
        }
        let hijo = self.copiar(padre, nombre.clone(), c, entradas);
        self.ambitos[padre].instancias.insert(nombre, hijo);
        Ok(())
    }

    /// Copia `c` dentro de `padre`. Lo que se copia de una instancia del
    /// principal (con sus instancias internas) se dibuja dentro de su caja.
    fn copiar(
        &mut self,
        padre: usize,
        nombre: String,
        c: &'a Circuito,
        entradas: Vec<Ref>,
    ) -> usize {
        let arriba = padre == 0;
        let g = self.instancias.len();
        if arriba {
            self.actual = Some(g);
            self.instancias.push(Caja {
                nombre,
                circuito: c.nombre.texto.clone(),
                entradas: bits_de(&c.entradas)
                    .into_iter()
                    .zip(entradas.clone())
                    .collect(),
                salidas: Vec::new(),
                split: None,
            });
        }
        let hijo = self.ambito(c, Some(entradas));
        if arriba {
            self.actual = None;
            self.instancias[g].salidas = bits_de(&c.salidas)
                .into_iter()
                .map(|b| (b.clone(), Ref::Senal(hijo, b, 0..0)))
                .collect();
        }
        hijo
    }

    /// Crea los nodos de una expresión (las compuertas anidadas también) y
    /// devuelve sus bits, del más bajo al más alto.
    fn construir(&mut self, id: usize, e: &'a Expr, vars: &Variables) -> Result<Vec<Ref>, Error> {
        match e {
            Expr::Senal(a) => self.leer(id, a, vars),
            Expr::Constante(v) => Ok(vec![Ref::Nodo(self.nodo(Tipo::Constante(*v), Vec::new()))]),
            Expr::Compuerta(c, args) => {
                let mut partes = Vec::new();
                for a in args {
                    partes.push(self.construir(id, a, vars)?);
                }
                // Un bit suelto se repite para cada bit del bus: and(D, E) con D de 8 bits
                let ancho = partes.iter().map(Vec::len).max().unwrap_or(1);
                for (p, a) in partes.iter().zip(args) {
                    if p.len() != ancho && p.len() != 1 {
                        return Err(error(
                            format!(
                                "las entradas de `{}` deben tener el mismo ancho (o 1 bit): hay de {} y de {}",
                                c.nombre(),
                                en_bits(ancho),
                                en_bits(p.len())
                            ),
                            rango_de(a),
                        ));
                    }
                }
                Ok((0..ancho)
                    .map(|k| {
                        let fuentes = partes
                            .iter()
                            .map(|p| p[if p.len() == 1 { 0 } else { k }].clone())
                            .collect();
                        Ref::Nodo(self.nodo(Tipo::Compuerta(*c), fuentes))
                    })
                    .collect())
            }
            Expr::Split(separa, partes, rango) => {
                let mut bits = Vec::new();
                for p in partes {
                    bits.extend(self.construir(id, p, vars)?);
                }
                if bits.len() < 2 {
                    return Err(error(
                        "`split.derecha` separa un bus de 2 o más bits; la señal es de 1 bit"
                            .into(),
                        rango.clone(),
                    ));
                }
                // En el circuito principal se dibuja: un buffer por bit dentro de su caja
                if id != 0 {
                    return Ok(bits);
                }
                let g = self.instancias.len();
                self.actual = Some(g);
                let salidas: Vec<Ref> = bits
                    .iter()
                    .map(|b| {
                        Ref::Nodo(self.nodo(Tipo::Compuerta(Compuerta::Buffer), vec![b.clone()]))
                    })
                    .collect();
                self.actual = None;
                // Los pines sueltos llevan su número; el lado del bus no lleva nada
                let n = bits.len();
                let separa = *separa;
                let pines = |suelto: bool| {
                    (0..n).map(move |k| if suelto { k.to_string() } else { String::new() })
                };
                self.instancias.push(Caja {
                    // ponytail: se ubica por orden; si cambia el código puede saltar de lugar
                    nombre: format!("split{g}"),
                    circuito: "split".into(),
                    entradas: pines(!separa).zip(bits).collect(),
                    salidas: pines(separa).zip(salidas.clone()).collect(),
                    split: Some(if separa { Split::Separa } else { Split::Junta }),
                });
                Ok(salidas)
            }
            Expr::Instancia(circuito, args) => {
                if let Some(m) = self.ambitos[id].moldes.get(&circuito.texto).copied() {
                    return self.memoria(id, circuito, m, args, vars);
                }
                Err(error(
                    format!(
                        "`{0}(...)` crea un circuito: asígnalo a un nombre (`m = {0}(...);`) y usa sus salidas con `m.Salida`",
                        circuito.texto
                    ),
                    circuito.rango.clone(),
                ))
            }
            Expr::Molde(_, rango) => Err(error(
                "un molde de memoria se guarda en un nombre (`R = rom.size(...);`) y se usa con `R(dirección)`".into(),
                rango.clone(),
            )),
        }
    }

    /// Sigue los nombres (`X = Y;`, entradas de instancias, `m.Suma`) hasta llegar a un nodo.
    fn resolver(&self, mut r: Ref) -> Result<usize, Error> {
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
                Ref::Senal(a, n, rango) => {
                    primero.get_or_insert((n.clone(), rango.clone()));
                    match self.ambitos[a].senales.get(&n) {
                        Some(siguiente) => siguiente.clone(),
                        // `fa[2].Cin`: entrada de un arreglo que nadie conectó
                        None if n.contains('.') => {
                            return Err(error(
                                format!(
                                    "la entrada `{n}` no está conectada: asígnale una señal con `{n} = ...;`"
                                ),
                                rango,
                            ));
                        }
                        None => {
                            // Los buses se sugieren por su nombre: `A`, no `A[0]`
                            let nombres = self.ambitos[a]
                                .senales
                                .keys()
                                .map(|x| x.split('[').next().unwrap_or(x));
                            return Err(error(no_existe(&n, nombres), rango));
                        }
                    }
                }
                Ref::Miembro(a, m, bit, rango) => match self.ambitos[a].instancias.get(&m) {
                    Some(&hijo) => Ref::Senal(hijo, bit, rango),
                    None if self.fallidas.contains(&(a, m.clone())) => {
                        return Err(error(String::new(), rango));
                    }
                    None => return Err(error(format!("`{m}` no es un circuito"), rango)),
                },
            };
        }
        let (n, rango) = primero.expect("el lazo pasó por al menos un nombre");
        Err(error(
            format!("`{n}` se define a sí misma sin pasar por ninguna compuerta"),
            rango,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::compilar;
    use crate::netlist::{Clase, Tipo, Visor};

    fn errores(fuente: &str) -> Vec<String> {
        compilar(fuente)
            .unwrap_err()
            .into_iter()
            .map(|e| e.mensaje)
            .collect()
    }

    const MEDIO_SUMADOR: &str = "circuit MedioSumador { in A, B; out Suma, Acarreo; Suma = xor(A, B); Acarreo = and(A, B); }";

    const SUMADOR_4: &str = "
        circuit FullAdder { in A, B, Cin; out Sum, Cout;
            Sum = xor(A, B, Cin); Cout = or(and(A, B), and(Cin, xor(A, B))); }
        circuit Adder4 {
            in A[4], B[4], Cin; out S[4], Cout;
            FullAdder fa[4];
            for (i = 0; i < 4; i++) { fa[i].A = A[i]; fa[i].B = B[i]; S[i] = fa[i].Sum; }
            fa[0].Cin = Cin;
            for (i = 1; i < 4; i++) { fa[i].Cin = fa[i-1].Cout; }
            Cout = fa[3].Cout;
        }";

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
            errores("main { in A, A; out X, Y; X = and(A, B); A = 1; X = 0; X = 1; }"),
            [
                "`A` está declarada dos veces",
                "`B` no existe: declárala con `in` o asígnale un valor",
                "`A` es una entrada: no se le puede asignar un valor",
                "`X` ya tiene un valor: cada señal se asigna una sola vez",
                "la salida `Y` nunca recibe un valor",
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
    fn sugerencias_por_nombre_parecido() {
        assert_eq!(
            errores("main { in Entrada; out X; X = not(Entrda); }"),
            ["`Entrda` no existe; ¿quisiste decir `Entrada`?"]
        );
        assert_eq!(
            errores(&format!(
                "{MEDIO_SUMADOR} main {{ in A, B; out X; m = medioSumador(A, B); X = m.Sum; }}"
            )),
            ["`medioSumador` no es un circuito del proyecto; ¿quisiste decir `MedioSumador`?"]
        );
        assert_eq!(
            errores(&format!(
                "{MEDIO_SUMADOR} main {{ in A, B; out X; m = MedioSumador(A, B); X = m.Sum; }}"
            )),
            ["`MedioSumador` no tiene la salida `Sum`; ¿quisiste decir `Suma`?"]
        );
        // Nada parecido: el mensaje de siempre
        assert_eq!(
            errores("main { in A; out X; X = not(Zeta); }"),
            ["`Zeta` no existe: declárala con `in` o asígnale un valor"]
        );
    }

    #[test]
    fn errores_de_instancias() {
        let con = |cuerpo: &str| errores(&format!("{MEDIO_SUMADOR} main {{ {cuerpo} }}"));
        assert_eq!(
            con("out X; m = MedioSumador(1); X = m.Suma;"),
            ["`MedioSumador` lleva 2 entradas, pero tiene 1"]
        );
        // Los usos de una instancia que falló no repiten el error
        assert_eq!(
            con("out X, Y; m = Nada(1, 0); X = m.Suma; Y = m.Acarreo;"),
            ["`Nada` no es un circuito del proyecto"]
        );
        assert_eq!(
            errores(
                "circuit D { in Q[2]; out S; S = and(Q[0], Q[1]); }
                 main { in A, B; out X; d = D(A, B); X = d.S; }"
            ),
            [
                "`D` lleva 1 entrada, pero tiene 2. `Q` es una sola entrada de 2 bits: pásale un bus entero, como `X` o `X[0..1]`"
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
            ["`Bucle` se usa dentro de sí mismo"]
        );
    }

    #[test]
    fn buses_bit_a_bit_y_unir() {
        let red = compilar(
            "circuit N { in A[4], B[4], E; out X[4], Y[4], Z[2];
             X = and(A, B); Y = and(A, E); Z = split.izquierda(B[0], A[3]); }",
        )
        .unwrap();
        let nombres: Vec<&str> = red
            .nodos
            .iter()
            .filter(|n| matches!(n.tipo, Tipo::Entrada(..)))
            .map(|n| n.nombre.as_str())
            .collect();
        assert_eq!(
            nombres,
            [
                "A[0]", "A[1]", "A[2]", "A[3]", "B[0]", "B[1]", "B[2]", "B[3]", "E"
            ]
        );
        // 9 entradas, 4 + 4 compuertas, 2 buffers del split, 10 salidas
        assert_eq!(red.nodos.len(), 9 + 8 + 2 + 10);
        assert_eq!(red.nodos[9].entradas, [0, 4]); // X[0] = and(A[0], B[0])
        assert_eq!(red.nodos[9].nombre, "X[0]");
        assert_eq!(red.nodos[16].entradas, [3, 8]); // Y[3] = and(A[3], E)
        // split.izquierda(B[0], A[3]): el primero es el bit 0; cada bit pasa por un buffer de la caja
        let caja = &red.instancias[0];
        assert_eq!(caja.split, Some(super::Split::Junta));
        assert_eq!(caja.entradas, [("0".into(), 4), ("1".into(), 3)]);
        assert_eq!(caja.salidas, [(String::new(), 17), (String::new(), 18)]);
        assert_eq!(red.nodos[17].nombre, "Z[0]");
        assert_eq!(red.nodos[17].instancia, Some(0));
        let z1 = red
            .nodos
            .iter()
            .find(|n| n.nombre == "Z[1]" && n.instancia.is_none())
            .unwrap();
        assert_eq!(z1.entradas, [18]);
        // Separar: entra el bus por un pin, salen los bits numerados
        let red = compilar(
            "circuit S { in A[2]; out a0, a1; a = split.derecha(A); a0 = a[0]; a1 = a[1]; }",
        )
        .unwrap();
        assert_eq!(red.instancias[0].split, Some(super::Split::Separa));
        assert_eq!(red.instancias[0].salidas[1].0, "1");
    }

    #[test]
    fn arreglos_y_for() {
        let red = compilar(SUMADOR_4).unwrap();
        assert_eq!(red.nombre, "Adder4");
        assert_eq!(red.instancias.len(), 4);
        assert_eq!(red.instancias[3].nombre, "fa[3]");
        let salidas = red
            .nodos
            .iter()
            .filter(|n| matches!(n.tipo, Tipo::Salida(_)))
            .count();
        assert_eq!(salidas, 5);
        // Señales internas: el ancho sale de lo que se asigna
        assert!(
            compilar("circuit T { in A[4]; out X[4]; t = not(A); u[1] = A[0]; u[0] = 1; X = split.izquierda(u, t[2..3]); }")
                .is_ok()
        );
    }

    #[test]
    fn memorias_visores_y_tristate() {
        let red = compilar(
            "R = rom.size(2, 4, \"1 2 3 4\");
             main { in A[2], E, D; button B; clock CLK; out X[4], Y, Q[8];
                 X = R(A);
                 M = ram.size(1, 8);
                 Q = M(E, split.izquierda(D, D, D, D, D, D, D, D), B, CLK);
                 Y = tristate(D, E); Y = tristate(B, not(E)); Y = tristate(CLK, 0);
                 led L(Y); display7 S(X, Q[0..3]); }",
        )
        .unwrap();
        let clases: Vec<Clase> = red
            .nodos
            .iter()
            .filter_map(|n| match n.tipo {
                Tipo::Entrada(_, c) => Some(c),
                _ => None,
            })
            .collect();
        use Clase::*;
        assert_eq!(
            clases,
            [
                Interruptor,
                Interruptor,
                Interruptor,
                Interruptor,
                Pulsador,
                Reloj
            ]
        );
        assert_eq!(red.memorias.len(), 2);
        assert_eq!(red.memorias[0].datos, [1, 2, 3, 4]);
        assert!(red.memorias[1].ram);
        // ROM, split, RAM y display7 son cajas; el LED va suelto
        let cajas: Vec<&str> = red.instancias.iter().map(|c| c.circuito.as_str()).collect();
        assert_eq!(cajas, ["rom", "split", "ram", "display7"]);
        assert_eq!(red.instancias[0].salidas.len(), 4);
        // RAM: dirección (1) + dato (8) + escribir + reloj
        assert_eq!(red.instancias[2].entradas.len(), 11);
        assert_eq!(red.instancias[2].entradas[10].0, "C");
        assert_eq!(red.instancias[3].entradas[7].0, "dp");
        // Los tres tristate se juntan en un bus
        let bus = red.nodos.iter().find(|n| n.tipo == Tipo::Bus).unwrap();
        assert_eq!(bus.entradas.len(), 3);
        assert_eq!(bus.nombre, "Y");
        assert!(
            red.nodos
                .iter()
                .any(|n| n.tipo == Tipo::Visor(Visor::Led, "L".into()) && n.instancia.is_none())
        );
    }

    #[test]
    fn errores_de_memorias_y_visores() {
        let con = |cuerpo: &str| errores(&format!("main {{ in A[2], B; {cuerpo} }}"));
        assert_eq!(
            con("out X[4]; R = rom.size(3, 4); X = R(A);"),
            ["la dirección de `R` es de 3 bits, pero recibe 2 bits"]
        );
        assert_eq!(
            con("out X[4]; M = ram.size(2, 4); X = M(A);"),
            ["`M` se usa con `M(dirección, dato, escribir, reloj)`"]
        );
        assert_eq!(
            con("out X; R = rom.size(2, 1); X = R;"),
            ["`R` es un molde de memoria: úsalo con `R(dirección)`"]
        );
        assert_eq!(
            con("out X[8]; R = rom.size(2, 8); r = R(A); X = r.D;"),
            ["`r` es una señal, no un circuito: úsala sin punto, como `r` o `r[0]`"]
        );
        assert_eq!(
            con("out X; X = and(rom.size(2, 1), B);"),
            [
                "un molde de memoria se guarda en un nombre (`R = rom.size(...);`) y se usa con `R(dirección)`"
            ]
        );
        assert_eq!(
            con("display7 D(A, B);"),
            ["`display7` lleva 8 bits: a, b, c, d, e, f, g y dp, pero recibe 3 bits"]
        );
        assert_eq!(
            con("matrix M(2, 2, A, B);"),
            ["`matrix` lleva 4 bits: 2 filas y 2 columnas, pero recibe 3 bits"]
        );
        assert_eq!(
            con("led B(B);"),
            ["`B` ya tiene un valor: cada nombre se usa una sola vez"]
        );
        assert_eq!(
            con("out X; X = tristate(B, B); X = B;"),
            ["`X` ya tiene un valor: cada señal se asigna una sola vez"]
        );
        assert_eq!(
            con("out X; X = B; X = tristate(B, B);"),
            ["`X` ya tiene un valor: cada señal se asigna una sola vez"]
        );
        assert_eq!(
            con("out X; X = not(B); X = tristate(B, B);"),
            ["`X` ya tiene un valor: solo varios `tristate` pueden manejar la misma señal"]
        );
    }

    #[test]
    fn errores_de_buses() {
        assert_eq!(
            errores("main { in A; out X; X = split.derecha(A); }"),
            ["`split.derecha` separa un bus de 2 o más bits; la señal es de 1 bit"]
        );

        assert_eq!(
            errores("main { in A[4], B[2]; out X[4]; X = and(A, B); }"),
            [
                "las entradas de `and` deben tener el mismo ancho (o 1 bit): hay de 4 bits y de 2 bits"
            ]
        );
        assert_eq!(
            errores("main { in A[4]; out X; X = A; }"),
            ["`X` es de 1 bit, pero recibe 4 bits"]
        );
        assert_eq!(
            errores("main { in A[4]; out X; X = A[4]; }"),
            ["`A` tiene 4 bits: el índice 4 no existe (van de 0 a 3)"]
        );
        assert_eq!(
            errores("main { in A; out X; X = A[0]; }"),
            ["`A` es de un bit: no lleva índice"]
        );
        assert_eq!(
            errores("main { in A[2]; out X[2]; X[0] = A[j]; X[1] = A[1]; }")[0],
            "`j` no es una variable de `for`: en los índices solo van números y la variable del `for`"
        );
        assert_eq!(
            errores("main { in A; out S[2]; S[0] = A; }"),
            ["la salida `S[1]` nunca recibe un valor"]
        );
        let sin_acarreo = SUMADOR_4.replace("fa[0].Cin = Cin;", "");
        assert_eq!(
            errores(&sin_acarreo),
            ["la entrada `fa[0].Cin` no está conectada: asígnale una señal con `fa[0].Cin = ...;`"]
        );
        let fuera = SUMADOR_4.replace("i < 4; i++) { fa[i].Cin", "i < 5; i++) { fa[i].Cin");
        assert_eq!(
            errores(&fuera),
            ["`fa` tiene 4 circuitos: el índice 4 no existe (van de 0 a 3)"]
        );
    }
}
