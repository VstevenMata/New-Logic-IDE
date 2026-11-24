//! Proyecto: un solo archivo `.SA` con varios archivos adentro, como el
//! `.CircuitProject` de LogicCircuit. Cualquier circuito de cualquier archivo se usa
//! desde los demás por su nombre, sin `import`.
//!
//! En disco cada archivo empieza con una línea `//@ archivo Nombre` y las piezas
//! movidas van al final como `//@ pos Archivo⇥clave⇥x⇥y`. Como son comentarios, el
//! proyecto entero sigue siendo código válido. Un `.SA` sin esas líneas se abre como
//! un proyecto de un solo archivo.

use logicsm_core::lexer::{Tipo, tokenizar};
use logicsm_core::{Error, Netlist};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Prefijo de las líneas del proyecto que no son código de ningún archivo
// ponytail: una línea de código que empiece así se tomaría como marca; escapar si pasa
const MARCA: &str = "//@ ";

/// Posición de cada pieza movida en el lienzo, por su clave
pub type Posiciones = HashMap<String, [f32; 2]>;

/// Un error para la pestaña Problemas
#[derive(Clone, Debug)]
pub struct Problema {
    /// "Mux4, línea 3: mensaje"
    pub texto: String,
    /// Archivo y rango en bytes dentro de su código: para subrayarlo y saltar a él
    pub lugar: Option<(String, Range<usize>)>,
}

impl Problema {
    pub fn sin_lugar(texto: String) -> Self {
        Self { texto, lugar: None }
    }
}

pub struct Archivo {
    /// Sin extensión: `Mux2`
    pub nombre: String,
    pub codigo: String,
    /// El contenido guardado, para marcar los archivos con cambios
    guardado: String,
}

impl Archivo {
    fn nuevo(nombre: String, codigo: String) -> Self {
        Self {
            nombre,
            codigo,
            guardado: String::new(),
        }
    }

    pub fn sin_guardar(&self) -> bool {
        self.codigo != self.guardado
    }
}

pub struct Proyecto {
    /// El `.SA` del proyecto; `None` hasta que se guarda por primera vez
    pub ruta: Option<PathBuf>,
    /// Nunca vacío
    pub archivos: Vec<Archivo>,
    pub activo: usize,
    /// Piezas movidas en el lienzo, por archivo
    pub posiciones: HashMap<String, Posiciones>,
    /// El texto que tiene en disco (o el de un proyecto nuevo)
    guardado: String,
}

pub fn es_sa(ruta: &Path) -> bool {
    ruta.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("sa"))
}

impl Proyecto {
    /// Un archivo `main` vacío
    pub fn nuevo() -> Self {
        let mut p = Self {
            ruta: None,
            archivos: vec![Archivo::nuevo("main".into(), String::new())],
            activo: 0,
            posiciones: HashMap::new(),
            guardado: String::new(),
        };
        p.guardado = p.texto();
        p
    }

    pub fn abrir(ruta: PathBuf) -> std::io::Result<Self> {
        let texto = std::fs::read_to_string(&ruta)?;
        Ok(Self::leido(ruta, &texto))
    }

    /// El proyecto guardado en `ruta`, ya leído (en el navegador `ruta` es solo el nombre)
    pub fn leido(ruta: PathBuf, texto: &str) -> Self {
        let nombre = ruta.file_stem().unwrap_or_default().to_string_lossy();
        let mut p = Self::desde_texto(texto, &nombre);
        p.ruta = Some(ruta);
        p
    }

    fn desde_texto(texto: &str, nombre: &str) -> Self {
        let mut archivos: Vec<Archivo> = Vec::new();
        let mut posiciones: HashMap<String, Posiciones> = HashMap::new();
        for linea in texto.split_inclusive('\n') {
            let Some(marca) = linea.strip_prefix(MARCA) else {
                if let Some(a) = archivos.last_mut() {
                    a.codigo.push_str(linea);
                }
                continue;
            };
            let marca = marca.trim_end();
            if let Some(nombre) = marca.strip_prefix("archivo ") {
                archivos.push(Archivo::nuevo(nombre.trim().into(), String::new()));
            } else if let Some(pos) = marca.strip_prefix("pos ")
                && let [archivo, clave, x, y] = pos.split('\t').collect::<Vec<_>>()[..]
                && let (Ok(x), Ok(y)) = (x.parse(), y.parse())
            {
                posiciones
                    .entry(archivo.into())
                    .or_default()
                    .insert(clave.into(), [x, y]);
            }
        }
        if archivos.is_empty() {
            archivos.push(Archivo::nuevo(nombre.into(), texto.into()));
        } else {
            // El salto de línea que `texto` agrega después de cada archivo
            for a in &mut archivos {
                if a.codigo.ends_with('\n') {
                    a.codigo.pop();
                }
            }
        }
        for a in &mut archivos {
            a.guardado = a.codigo.clone();
        }
        let mut p = Self {
            ruta: None,
            archivos,
            activo: 0,
            posiciones,
            guardado: String::new(),
        };
        p.guardado = p.texto();
        p
    }

    /// Lo que se escribe en disco
    pub fn texto(&self) -> String {
        let mut texto =
            format!("// Proyecto de LogicSM: no borres ni cambies las líneas {MARCA}...\n");
        for a in &self.archivos {
            texto.push_str(&format!("{MARCA}archivo {}\n{}\n", a.nombre, a.codigo));
        }
        for a in &self.archivos {
            let Some(p) = self.posiciones.get(&a.nombre) else {
                continue;
            };
            let mut lineas: Vec<_> = p.iter().collect();
            lineas.sort_by(|x, y| x.0.cmp(y.0));
            for (clave, [x, y]) in lineas {
                texto.push_str(&format!("{MARCA}pos {}\t{clave}\t{x}\t{y}\n", a.nombre));
            }
        }
        texto
    }

    pub fn actual(&self) -> &Archivo {
        &self.archivos[self.activo]
    }

    pub fn actual_mut(&mut self) -> &mut Archivo {
        &mut self.archivos[self.activo]
    }

    /// Código, nombres, archivos o posiciones distintos de lo guardado
    pub fn sin_guardar(&self) -> bool {
        self.texto() != self.guardado
    }

    fn buscar(&self, nombre: &str) -> Option<usize> {
        self.archivos
            .iter()
            .position(|a| a.nombre.eq_ignore_ascii_case(nombre))
    }

    /// Escribe el proyecto en su archivo o, con `nueva` (la primera vez o con
    /// Guardar como), en ese otro, que pasa a ser el del proyecto.
    pub fn guardar(&mut self, nueva: Option<PathBuf>) -> Result<(), String> {
        let ruta = match nueva {
            Some(r) if es_sa(&r) => r,
            Some(r) => r.with_extension("SA"),
            None => self
                .ruta
                .clone()
                .ok_or("Elige dónde guardar el proyecto.")?,
        };
        let texto = self.texto();
        crate::plataforma::escribir(&ruta, texto.as_bytes())
            .map_err(|e| format!("No se pudo guardar {}: {e}", ruta.display()))?;
        for a in &mut self.archivos {
            a.guardado = a.codigo.clone();
        }
        self.guardado = texto;
        self.ruta = Some(ruta);
        Ok(())
    }

    /// Crea un archivo vacío y lo abre.
    pub fn crear(&mut self, nombre: &str) -> Result<(), String> {
        let nombre = self.nombre_libre(nombre, None)?;
        self.archivos.push(Archivo::nuevo(nombre, String::new()));
        self.activo = self.archivos.len() - 1;
        Ok(())
    }

    /// Cambia el nombre del archivo `i`. El circuito de adentro conserva el suyo:
    /// los demás archivos lo usan por ese nombre.
    pub fn renombrar(&mut self, i: usize, nombre: &str) -> Result<(), String> {
        let nombre = self.nombre_libre(nombre, Some(i))?;
        let viejo = std::mem::replace(&mut self.archivos[i].nombre, nombre.clone());
        if let Some(p) = self.posiciones.remove(&viejo) {
            self.posiciones.insert(nombre, p);
        }
        Ok(())
    }

    /// El nombre limpio, si sirve y nadie más lo usa (`propio` es el que se renombra).
    /// Debe ser un identificador, como los nombres de circuito de LogicCircuit.
    fn nombre_libre(&self, nombre: &str, propio: Option<usize>) -> Result<String, String> {
        let nombre = nombre.trim();
        let (tokens, errores) = tokenizar(nombre);
        let valido = errores.is_empty()
            && matches!(tokens.as_slice(), [t, _] if matches!(t.tipo, Tipo::Ident(_)));
        if !valido {
            return Err(format!(
                "`{nombre}` no sirve como nombre: usa letras sin tilde, dígitos y _, empieza con \
                 una letra, sin espacios, y que no sea una palabra reservada (por ejemplo Mux_4)."
            ));
        }
        match self.buscar(nombre) {
            Some(j) if Some(j) != propio => Err(format!("Ya existe {nombre} en el proyecto.")),
            _ => Ok(nombre.to_string()),
        }
    }

    /// Agrega los archivos de un ejemplo y abre el último. No reemplaza los que ya
    /// existen; devuelve un aviso por cada uno que es distinto.
    pub fn agregar(&mut self, archivos: &[(&str, &str)]) -> Vec<String> {
        let mut avisos = Vec::new();
        for &(nombre, codigo) in archivos {
            match self.buscar(nombre) {
                Some(i) => {
                    if self.archivos[i].codigo != codigo {
                        avisos.push(format!(
                            "{nombre} ya existe en el proyecto: no se reemplazó."
                        ));
                    }
                    self.activo = i;
                }
                None => {
                    self.archivos
                        .push(Archivo::nuevo(nombre.into(), codigo.into()));
                    self.activo = self.archivos.len() - 1;
                }
            }
        }
        avisos
    }

    /// Lo quita del proyecto (en disco, al guardar).
    pub fn borrar(&mut self, i: usize) -> Result<(), String> {
        if self.archivos.len() == 1 {
            return Err("El proyecto necesita al menos un archivo.".into());
        }
        let a = self.archivos.remove(i);
        self.posiciones.remove(&a.nombre);
        if self.activo > i || self.activo == self.archivos.len() {
            self.activo -= 1;
        }
        Ok(())
    }

    /// Compila el archivo abierto
    pub fn compilar(&self) -> Result<Netlist, Vec<Problema>> {
        self.compilar_archivo(self.activo)
    }

    /// Junta todos los archivos y compila el circuito principal del archivo `indice`
    /// (`main`, o el último que ningún otro usa). Cada error dice su archivo y línea.
    pub fn compilar_archivo(&self, indice: usize) -> Result<Netlist, Vec<Problema>> {
        use logicsm_core::parser::parsear;
        let abierto = &self.archivos[indice];
        let circuitos = parsear(&abierto.codigo).map_err(|e| ubicar_todos(abierto, &e))?;
        let Some(principal) = logicsm_core::principal(&circuitos) else {
            return Err(vec![Problema::sin_lugar(format!(
                "{}: no hay ningún circuito; empieza con `circuit Nombre {{ ... }}`",
                abierto.nombre
            ))]);
        };

        // Un archivo con errores de sintaxis se deja fuera: no bloquea a los demás
        let mut fuente = String::new();
        let mut tramos = Vec::new();
        let mut rotos = Vec::new();
        for (i, a) in self.archivos.iter().enumerate() {
            if i != indice && parsear(&a.codigo).is_err() {
                rotos.push(Problema::sin_lugar(format!(
                    "{} tiene errores de sintaxis: sus circuitos no se pueden usar hasta corregirlo.",
                    a.nombre
                )));
                continue;
            }
            tramos.push((fuente.len(), a));
            fuente.push_str(&a.codigo);
            fuente.push('\n');
        }

        logicsm_core::compilar_circuito(&fuente, &principal.nombre.texto).map_err(|errores| {
            let mut problemas: Vec<Problema> = errores
                .iter()
                .map(|e| {
                    let &(inicio, a) = tramos
                        .iter()
                        .rev()
                        .find(|(inicio, _)| *inicio <= e.rango.start)
                        .expect("el primer tramo empieza en 0");
                    let rango = e.rango.start - inicio..e.rango.end.saturating_sub(inicio);
                    ubicar(a, rango, &e.mensaje)
                })
                .collect();
            problemas.extend(rotos);
            problemas
        })
    }
}

/// "Mux4, línea 3: mensaje", con el lugar del error en el archivo
fn ubicar(a: &Archivo, rango: Range<usize>, mensaje: &str) -> Problema {
    let linea = a.codigo[..rango.start.min(a.codigo.len())]
        .matches('\n')
        .count()
        + 1;
    Problema {
        texto: format!("{}, línea {linea}: {mensaje}", a.nombre),
        lugar: Some((a.nombre.clone(), rango)),
    }
}

fn ubicar_todos(a: &Archivo, errores: &[Error]) -> Vec<Problema> {
    errores
        .iter()
        .map(|e| ubicar(a, e.rango.clone(), &e.mensaje))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_los_ejemplos_compilan() {
        for (titulo, archivos) in crate::ayuda::EJEMPLOS {
            let mut p = Proyecto::nuevo();
            assert!(p.agregar(archivos).is_empty());
            assert!(p.compilar().is_ok(), "{titulo}: {:?}", p.compilar().err());
        }
    }

    /// El ejemplo `titulo` compilado (su último archivo queda abierto)
    fn ejemplo(titulo: &str) -> Netlist {
        let (_, archivos) = crate::ayuda::EJEMPLOS
            .iter()
            .find(|(t, _)| *t == titulo)
            .unwrap();
        let mut p = Proyecto::nuevo();
        p.agregar(archivos);
        p.compilar().unwrap()
    }

    #[test]
    fn el_sumador_de_4_bits_suma() {
        let tabla = logicsm_sim::Tabla::de(&ejemplo("Sumador de 4 bits (buses y for)"), false);
        // Entradas A[0..3], B[0..3]; salidas S[0..3], Cout: el bit 0 va primero
        assert_eq!(tabla.entradas.len(), 8);
        let numero = |bits: &[bool]| bits.iter().rev().fold(0, |n, &b| n * 2 + usize::from(b));
        for (f, salidas) in tabla.filas.iter().enumerate() {
            let e = tabla.fila(f);
            let s: Vec<bool> = salidas.iter().map(|v| v.bit().unwrap()).collect();
            assert_eq!(numero(&e[..4]) + numero(&e[4..]), numero(&s), "fila {f}");
        }
    }

    #[test]
    fn el_latch_d_recuerda() {
        let red = ejemplo("Latch D");
        let q = red
            .nodos
            .iter()
            .position(|n| matches!(&n.tipo, logicsm_core::netlist::Tipo::Salida(s) if s == "Q"))
            .unwrap();
        let mut e = logicsm_sim::Estado::default();
        // (D, E) → Q: copia a D mientras E = 1 y recuerda con E = 0
        for (d, en, valor) in [(1, 1, 1), (1, 0, 1), (0, 0, 1), (0, 1, 0), (1, 0, 0)] {
            assert!(logicsm_sim::simular(&red, &mut e, &[d == 1, en == 1]));
            assert_eq!(e.valores[q].bit(), Some(valor == 1), "D = {d}, E = {en}");
        }
    }

    #[test]
    fn circuitos_de_otros_archivos_y_errores_ubicados() {
        let mut p = Proyecto::nuevo();
        let sumador = crate::ayuda::EJEMPLOS
            .iter()
            .find(|(titulo, _)| *titulo == "Sumador completo")
            .unwrap()
            .1;
        p.agregar(sumador);
        assert_eq!(p.actual().nombre, "SumadorCompleto");
        assert_eq!(p.compilar().unwrap().nombre, "SumadorCompleto");

        // Un archivo con errores de sintaxis no bloquea a los demás
        p.archivos[0].codigo = "circuit {".into();
        assert!(p.compilar().is_ok());

        // El error dice en qué archivo y línea está
        p.actual_mut().codigo = "circuit X {\n    out Y;\n    Y = Z;\n}".into();
        let errores = p.compilar().unwrap_err();
        // y dónde: el editor subraya justo la palabra
        let (archivo, rango) = errores[0].lugar.clone().unwrap();
        assert_eq!(archivo, "SumadorCompleto");
        assert_eq!(&p.actual().codigo[rango], "Z");
        assert_eq!(
            errores.into_iter().map(|e| e.texto).collect::<Vec<_>>(),
            [
                "SumadorCompleto, línea 3: `Z` no existe: declárala con `in` o asígnale un valor",
                "main tiene errores de sintaxis: sus circuitos no se pueden usar hasta corregirlo.",
            ]
        );
    }

    #[test]
    fn crear_y_borrar_archivos() {
        let mut p = Proyecto::nuevo();
        assert!(p.crear("Mux_4").is_ok());
        assert_eq!(p.actual().nombre, "Mux_4");
        assert!(p.actual().codigo.is_empty());
        assert!(p.crear("mux_4").is_err()); // ya existe
        assert!(p.crear("and").is_err()); // palabra reservada
        assert!(p.crear("dos palabras").is_err());
        assert!(p.crear("Señal").is_err());
        assert!(p.crear("").is_err());

        assert!(p.crear("Otro").is_ok());
        assert!(p.renombrar(2, "Mux_4").is_err()); // ya existe
        assert!(p.renombrar(2, "otro").is_ok()); // solo mayúsculas
        assert!(p.renombrar(2, "Final").is_ok());
        assert_eq!(p.archivos[2].nombre, "Final");
        assert!(p.borrar(2).is_ok());

        assert!(p.borrar(1).is_ok());
        assert_eq!((p.archivos.len(), p.activo), (1, 0));
        assert!(p.borrar(0).is_err()); // el último no se borra
    }

    #[test]
    fn un_solo_archivo_con_todo_adentro() {
        let mut p = Proyecto::nuevo();
        assert!(!p.sin_guardar());
        p.agregar(&[("A", "circuit A {\n}\n"), ("B", "sin salto")]);
        p.posiciones
            .entry("A".into())
            .or_default()
            .insert("caja m1".into(), [10.5, -3.0]);
        assert!(p.sin_guardar());

        let carpeta = std::env::temp_dir().join(format!("logicsm-prueba-{}", std::process::id()));
        std::fs::create_dir_all(&carpeta).unwrap();
        p.guardar(Some(carpeta.join("Proyecto"))).unwrap();
        assert!(!p.sin_guardar());
        let ruta = carpeta.join("Proyecto.SA");
        assert_eq!(p.ruta.as_deref(), Some(ruta.as_path()));

        // Se lee igual que se escribió
        let q = Proyecto::abrir(ruta).unwrap();
        let nombres: Vec<_> = q.archivos.iter().map(|a| a.nombre.as_str()).collect();
        assert_eq!(nombres, ["main", "A", "B"]);
        assert_eq!(q.archivos[1].codigo, "circuit A {\n}\n");
        assert_eq!(q.archivos[2].codigo, "sin salto");
        assert_eq!(q.posiciones["A"]["caja m1"], [10.5, -3.0]);
        assert!(!q.sin_guardar());

        // Un .SA suelto es un proyecto de un archivo con su nombre
        let suelto = carpeta.join("Mux.SA");
        std::fs::write(&suelto, "circuit Mux {\n}\n").unwrap();
        let r = Proyecto::abrir(suelto).unwrap();
        assert_eq!(r.archivos.len(), 1);
        assert_eq!(r.actual().nombre, "Mux");
        assert_eq!(r.actual().codigo, "circuit Mux {\n}\n");
        std::fs::remove_dir_all(&carpeta).unwrap();
    }
}
