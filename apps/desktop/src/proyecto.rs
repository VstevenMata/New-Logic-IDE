//! Proyecto: una carpeta con archivos `.SA`. Cualquier circuito de cualquier
//! archivo se usa desde los demás por su nombre, sin `import`.

use logicsm_core::lexer::{Tipo, tokenizar};
use logicsm_core::{Error, Netlist};
use std::path::{Path, PathBuf};

/// Contenido del `main.SA` de un proyecto nuevo
pub const INICIO: &str = "// Crea más archivos con el botón + del panel Proyecto.
// Ayuda → Ejemplos agrega circuitos clásicos al proyecto.
main {
    in A, B, C;
    out X, Y;

    X = and(A, B);
    Y = or(X, not(C));
}
";

pub struct Archivo {
    /// Con extensión: `Mux2.SA`
    pub nombre: String,
    pub codigo: String,
    /// El contenido que tiene en disco, para saber si hay cambios sin guardar
    guardado: String,
    en_disco: bool,
}

impl Archivo {
    fn nuevo(nombre: String, codigo: String) -> Self {
        Self {
            nombre,
            codigo,
            guardado: String::new(),
            en_disco: false,
        }
    }

    pub fn sin_guardar(&self) -> bool {
        self.codigo != self.guardado
    }
}

pub struct Proyecto {
    /// `None` hasta que se guarda por primera vez
    pub carpeta: Option<PathBuf>,
    /// Nunca vacío
    pub archivos: Vec<Archivo>,
    pub activo: usize,
}

fn es_sa(ruta: &Path) -> bool {
    ruta.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("sa"))
}

impl Proyecto {
    pub fn nuevo() -> Self {
        Self {
            carpeta: None,
            archivos: vec![Archivo {
                nombre: "main.SA".into(),
                codigo: INICIO.into(),
                guardado: INICIO.into(),
                en_disco: false,
            }],
            activo: 0,
        }
    }

    /// Lee los `.SA` de la carpeta (no los de sus subcarpetas).
    pub fn abrir(carpeta: PathBuf) -> std::io::Result<Self> {
        let mut archivos = Vec::new();
        for entrada in std::fs::read_dir(&carpeta)? {
            let ruta = entrada?.path();
            if ruta.is_file() && es_sa(&ruta) {
                let codigo = std::fs::read_to_string(&ruta)?;
                archivos.push(Archivo {
                    nombre: ruta
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    guardado: codigo.clone(),
                    codigo,
                    en_disco: true,
                });
            }
        }
        archivos.sort_by_key(|a| a.nombre.to_lowercase());
        if archivos.is_empty() {
            archivos = Self::nuevo().archivos;
        }
        Ok(Self {
            carpeta: Some(carpeta),
            archivos,
            activo: 0,
        })
    }

    pub fn actual(&self) -> &Archivo {
        &self.archivos[self.activo]
    }

    pub fn actual_mut(&mut self) -> &mut Archivo {
        &mut self.archivos[self.activo]
    }

    pub fn sin_guardar(&self) -> bool {
        self.archivos.iter().any(Archivo::sin_guardar)
    }

    fn buscar(&self, nombre: &str) -> Option<usize> {
        self.archivos
            .iter()
            .position(|a| a.nombre.eq_ignore_ascii_case(nombre))
    }

    /// Guarda en `nueva` (la primera vez o con Guardar como) o en la carpeta actual.
    /// En una carpeta nueva nunca reemplaza archivos que ya estaban ahí.
    pub fn guardar(&mut self, nueva: Option<PathBuf>) -> Result<(), String> {
        let cambia = nueva.is_some() && nueva != self.carpeta;
        let Some(carpeta) = nueva.or_else(|| self.carpeta.clone()) else {
            return Err("Elige una carpeta para el proyecto.".into());
        };
        if cambia {
            let ocupados: Vec<&str> = self
                .archivos
                .iter()
                .filter(|a| carpeta.join(&a.nombre).exists())
                .map(|a| a.nombre.as_str())
                .collect();
            if !ocupados.is_empty() {
                return Err(format!(
                    "La carpeta ya tiene {}. Elige una carpeta vacía para no reemplazarlos.",
                    ocupados.join(", ")
                ));
            }
        }
        self.carpeta = Some(carpeta.clone());
        for a in &mut self.archivos {
            if cambia || a.sin_guardar() || !a.en_disco {
                std::fs::write(carpeta.join(&a.nombre), &a.codigo)
                    .map_err(|e| format!("No se pudo guardar {}: {e}", a.nombre))?;
                a.guardado = a.codigo.clone();
                a.en_disco = true;
            }
        }
        Ok(())
    }

    /// Crea `Nombre.SA` con un circuito del mismo nombre y lo abre.
    pub fn crear(&mut self, nombre: &str) -> Result<(), String> {
        let (base, archivo) = self.nombre_libre(nombre, None)?;
        let codigo =
            format!("circuit {base} {{\n    in A, B;\n    out X;\n\n    X = and(A, B);\n}}\n");
        self.archivos.push(Archivo::nuevo(archivo, codigo));
        self.activo = self.archivos.len() - 1;
        Ok(())
    }

    /// Cambia el nombre del archivo `i` (también en la carpeta). El circuito de
    /// adentro conserva el suyo: los demás archivos lo usan por ese nombre.
    pub fn renombrar(&mut self, i: usize, nombre: &str) -> Result<(), String> {
        let (_, archivo) = self.nombre_libre(nombre, Some(i))?;
        let a = &self.archivos[i];
        if let (Some(carpeta), true) = (&self.carpeta, a.en_disco) {
            let nueva = carpeta.join(&archivo);
            // Cambiar solo mayúsculas (mux → Mux) es el mismo archivo en Windows y macOS
            if nueva.exists() && !archivo.eq_ignore_ascii_case(&a.nombre) {
                return Err(format!("La carpeta ya tiene {archivo}."));
            }
            std::fs::rename(carpeta.join(&a.nombre), nueva)
                .map_err(|e| format!("No se pudo renombrar {}: {e}", a.nombre))?;
        }
        self.archivos[i].nombre = archivo;
        Ok(())
    }

    /// `Nombre` o `Nombre.SA` → (`Nombre`, `Nombre.SA`), si sirve y nadie más lo usa
    /// (`propio` es el archivo que se renombra).
    fn nombre_libre(
        &self,
        nombre: &str,
        propio: Option<usize>,
    ) -> Result<(String, String), String> {
        let nombre = nombre.trim();
        let base = nombre
            .strip_suffix(".SA")
            .or_else(|| nombre.strip_suffix(".sa"))
            .unwrap_or(nombre);
        // El nombre del archivo es el del circuito que se crea: debe ser un nombre válido
        let (tokens, errores) = tokenizar(base);
        let valido = errores.is_empty()
            && matches!(tokens.as_slice(), [t, _] if matches!(t.tipo, Tipo::Ident(_)));
        if !valido {
            return Err(format!(
                "`{base}` no sirve como nombre: usa letras sin tilde, dígitos y _, empieza con \
                 una letra, sin espacios, y que no sea una palabra reservada (por ejemplo Mux_4)."
            ));
        }
        let archivo = format!("{base}.SA");
        match self.buscar(&archivo) {
            Some(j) if Some(j) != propio => Err(format!("Ya existe {archivo} en el proyecto.")),
            _ => Ok((base.to_string(), archivo)),
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

    /// Lo quita del proyecto y de la carpeta.
    pub fn borrar(&mut self, i: usize) -> Result<(), String> {
        if self.archivos.len() == 1 {
            return Err("El proyecto necesita al menos un archivo.".into());
        }
        let a = &self.archivos[i];
        if let (Some(carpeta), true) = (&self.carpeta, a.en_disco) {
            std::fs::remove_file(carpeta.join(&a.nombre))
                .map_err(|e| format!("No se pudo borrar {}: {e}", a.nombre))?;
        }
        self.archivos.remove(i);
        if self.activo > i || self.activo == self.archivos.len() {
            self.activo -= 1;
        }
        Ok(())
    }

    /// Junta todos los archivos y compila el circuito principal del archivo abierto
    /// (`main`, o el último que ningún otro usa). Cada error dice su archivo y línea.
    pub fn compilar(&self) -> Result<Netlist, Vec<String>> {
        use logicsm_core::parser::parsear;
        let abierto = self.actual();
        let circuitos = parsear(&abierto.codigo).map_err(|e| ubicar_todos(abierto, &e))?;
        let Some(principal) = logicsm_core::principal(&circuitos) else {
            return Err(vec![format!(
                "{}: no hay ningún circuito; empieza con `circuit Nombre {{ ... }}`",
                abierto.nombre
            )]);
        };

        // Un archivo con errores de sintaxis se deja fuera: no bloquea a los demás
        let mut fuente = String::new();
        let mut tramos = Vec::new();
        let mut rotos = Vec::new();
        for (i, a) in self.archivos.iter().enumerate() {
            if i != self.activo && parsear(&a.codigo).is_err() {
                rotos.push(format!(
                    "{} tiene errores de sintaxis: sus circuitos no se pueden usar hasta corregirlo.",
                    a.nombre
                ));
                continue;
            }
            tramos.push((fuente.len(), a));
            fuente.push_str(&a.codigo);
            fuente.push('\n');
        }

        logicsm_core::compilar_circuito(&fuente, &principal.nombre.texto).map_err(|errores| {
            let mut problemas: Vec<String> = errores
                .iter()
                .map(|e| {
                    let &(inicio, a) = tramos
                        .iter()
                        .rev()
                        .find(|(inicio, _)| *inicio <= e.rango.start)
                        .expect("el primer tramo empieza en 0");
                    ubicar(a, e.rango.start - inicio, &e.mensaje)
                })
                .collect();
            problemas.extend(rotos);
            problemas
        })
    }
}

/// "Mux4.SA, línea 3: mensaje"
fn ubicar(a: &Archivo, posicion: usize, mensaje: &str) -> String {
    let linea = a.codigo[..posicion.min(a.codigo.len())]
        .matches('\n')
        .count()
        + 1;
    format!("{}, línea {linea}: {mensaje}", a.nombre)
}

fn ubicar_todos(a: &Archivo, errores: &[Error]) -> Vec<String> {
    errores
        .iter()
        .map(|e| ubicar(a, e.rango.start, &e.mensaje))
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

    #[test]
    fn circuitos_de_otros_archivos_y_errores_ubicados() {
        let mut p = Proyecto::nuevo();
        let sumador = crate::ayuda::EJEMPLOS
            .iter()
            .find(|(titulo, _)| *titulo == "Sumador completo")
            .unwrap()
            .1;
        p.agregar(sumador);
        assert_eq!(p.actual().nombre, "SumadorCompleto.SA");
        assert_eq!(p.compilar().unwrap().nombre, "SumadorCompleto");

        // Un archivo con errores de sintaxis no bloquea a los demás
        p.archivos[0].codigo = "circuit {".into();
        assert!(p.compilar().is_ok());

        // El error dice en qué archivo y línea está
        p.actual_mut().codigo = "circuit X {\n    out Y;\n    Y = Z;\n}".into();
        assert_eq!(
            p.compilar().unwrap_err(),
            [
                "SumadorCompleto.SA, línea 3: `Z` no existe: declárala con `in` o asígnale un valor",
                "main.SA tiene errores de sintaxis: sus circuitos no se pueden usar hasta corregirlo.",
            ]
        );
    }

    #[test]
    fn crear_y_borrar_archivos() {
        let mut p = Proyecto::nuevo();
        assert!(p.crear("Mux_4").is_ok());
        assert_eq!(p.actual().nombre, "Mux_4.SA");
        assert!(p.actual().codigo.starts_with("circuit Mux_4 {"));
        assert!(p.crear("mux_4.sa").is_err()); // ya existe
        assert!(p.crear("and").is_err()); // palabra reservada
        assert!(p.crear("dos palabras").is_err());
        assert!(p.crear("Señal").is_err());
        assert!(p.crear("").is_err());

        assert!(p.crear("Otro").is_ok());
        assert!(p.renombrar(2, "Mux_4").is_err()); // ya existe
        assert!(p.renombrar(2, "otro").is_ok()); // solo mayúsculas
        assert!(p.renombrar(2, "Final.SA").is_ok());
        assert_eq!(p.archivos[2].nombre, "Final.SA");
        assert!(p.borrar(2).is_ok());

        assert!(p.borrar(1).is_ok());
        assert_eq!((p.archivos.len(), p.activo), (1, 0));
        assert!(p.borrar(0).is_err()); // el último no se borra
    }
}
