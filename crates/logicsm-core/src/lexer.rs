//! Lexer: convierte el código `.SA` en tokens.
//!
//! No se detiene en el primer error: los junta y sigue, así el editor puede
//! resaltar el resto del archivo.

use crate::Error;
use std::ops::Range;

/// Las 22 palabras reservadas del lenguaje.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reservada {
    // Estructura
    Circuit,
    Main,
    For,
    In,
    Out,
    // Compuertas
    And,
    Or,
    Xor,
    Nand,
    Nor,
    Xnor,
    Not,
    Buffer,
    Tristate,
    // Memorias
    Rom,
    Ram,
    // Dispositivos
    Button,
    Clock,
    Led,
    Display7,
    Matrix,
    Probe,
}

impl Reservada {
    fn desde(texto: &str) -> Option<Self> {
        use Reservada::*;
        Some(match texto {
            "circuit" => Circuit,
            "main" => Main,
            "for" => For,
            "in" => In,
            "out" => Out,
            "and" => And,
            "or" => Or,
            "xor" => Xor,
            "nand" => Nand,
            "nor" => Nor,
            "xnor" => Xnor,
            "not" => Not,
            "buffer" => Buffer,
            "tristate" => Tristate,
            "rom" => Rom,
            "ram" => Ram,
            "button" => Button,
            "clock" => Clock,
            "led" => Led,
            "display7" => Display7,
            "matrix" => Matrix,
            "probe" => Probe,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tipo {
    Reservada(Reservada),
    Ident(String),
    Entero(u64),
    Texto(String),
    LlaveAbre,   // {
    LlaveCierra, // }
    ParAbre,     // (
    ParCierra,   // )
    CorAbre,     // [
    CorCierra,   // ]
    PuntoComa,   // ;
    Coma,        // ,
    Igual,       // =
    Punto,       // .
    PuntoPunto,  // ..
    Menor,       // <
    Mas,         // +
    MasMas,      // ++
    Menos,       // -
    Fin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub tipo: Tipo,
    /// Posición en bytes dentro del código fuente.
    pub rango: Range<usize>,
}

/// Devuelve los tokens (siempre termina en `Tipo::Fin`) y los errores encontrados.
pub fn tokenizar(fuente: &str) -> (Vec<Token>, Vec<Error>) {
    let b = fuente.as_bytes();
    let mut tokens = Vec::new();
    let mut errores = Vec::new();
    let mut error = |mensaje: String, rango: Range<usize>| errores.push(Error { mensaje, rango });
    let mut i = 0;

    while i < b.len() {
        let inicio = i;
        let tipo = match b[i] {
            b' ' | b'\t' | b'\r' | b'\n' => {
                i += 1;
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                match fuente[i + 2..].find("*/") {
                    Some(p) => i += 2 + p + 2,
                    None => {
                        i = b.len();
                        error("comentario `/*` sin cerrar".into(), inicio..i);
                    }
                }
                continue;
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let texto = &fuente[inicio..i];
                Reservada::desde(texto).map_or_else(|| Tipo::Ident(texto.into()), Tipo::Reservada)
            }
            b'0'..=b'9' => {
                // Se toma todo lo alfanumérico para rechazar cosas como `8a` o `0xZZ`.
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let texto = &fuente[inicio..i];
                match entero(texto) {
                    Some(n) => Tipo::Entero(n),
                    None => {
                        error(format!("número no válido `{texto}`"), inicio..i);
                        continue;
                    }
                }
            }
            b'"' => match fuente[i + 1..].find(['"', '\n']) {
                Some(p) if b[i + 1 + p] == b'"' => {
                    i += 1 + p + 1;
                    Tipo::Texto(fuente[inicio + 1..i - 1].into())
                }
                p => {
                    i = p.map_or(b.len(), |p| i + 1 + p);
                    error("texto sin cerrar: falta `\"`".into(), inicio..i);
                    continue;
                }
            },
            c => {
                i += 1;
                match c {
                    b'{' => Tipo::LlaveAbre,
                    b'}' => Tipo::LlaveCierra,
                    b'(' => Tipo::ParAbre,
                    b')' => Tipo::ParCierra,
                    b'[' => Tipo::CorAbre,
                    b']' => Tipo::CorCierra,
                    b';' => Tipo::PuntoComa,
                    b',' => Tipo::Coma,
                    b'=' => Tipo::Igual,
                    b'<' => Tipo::Menor,
                    b'-' => Tipo::Menos,
                    b'.' if b.get(i) == Some(&b'.') => {
                        i += 1;
                        Tipo::PuntoPunto
                    }
                    b'.' => Tipo::Punto,
                    b'+' if b.get(i) == Some(&b'+') => {
                        i += 1;
                        Tipo::MasMas
                    }
                    b'+' => Tipo::Mas,
                    _ => {
                        let ch = fuente[inicio..].chars().next().unwrap_or_default();
                        i = inicio + ch.len_utf8();
                        let pista = if ch.is_alphabetic() {
                            ": los nombres solo usan letras sin tilde ni ñ, dígitos y `_`"
                        } else {
                            ""
                        };
                        error(format!("carácter no válido `{ch}`{pista}"), inicio..i);
                        continue;
                    }
                }
            }
        };
        tokens.push(Token {
            tipo,
            rango: inicio..i,
        });
    }

    tokens.push(Token {
        tipo: Tipo::Fin,
        rango: b.len()..b.len(),
    });
    (tokens, errores)
}

/// `8`, `0xFF`, `0b1010`.
fn entero(texto: &str) -> Option<u64> {
    if let Some(hex) = texto.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = texto.strip_prefix("0b") {
        u64::from_str_radix(bin, 2).ok()
    } else {
        texto.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tipos(fuente: &str) -> Vec<Tipo> {
        let (tokens, errores) = tokenizar(fuente);
        assert!(errores.is_empty(), "{errores:?}");
        tokens.into_iter().map(|t| t.tipo).collect()
    }

    fn mensajes(fuente: &str) -> Vec<String> {
        tokenizar(fuente).1.into_iter().map(|e| e.mensaje).collect()
    }

    #[test]
    fn reservadas_y_nombres() {
        use Tipo::*;
        assert_eq!(
            tipos("circuit And and display7 _x"),
            [
                Reservada(super::Reservada::Circuit),
                Ident("And".into()), // distingue mayúsculas
                Reservada(super::Reservada::And),
                Reservada(super::Reservada::Display7),
                Ident("_x".into()),
                Fin,
            ]
        );
    }

    #[test]
    fn numeros_simbolos_y_comentarios() {
        use Tipo::*;
        assert_eq!(
            tipos("8 0xFF 0b1010 // nada\n A[0..3] /* nada */ i++ \"datos.hex\""),
            [
                Entero(8),
                Entero(255),
                Entero(10),
                Ident("A".into()),
                CorAbre,
                Entero(0),
                PuntoPunto,
                Entero(3),
                CorCierra,
                Ident("i".into()),
                MasMas,
                Texto("datos.hex".into()),
                Fin,
            ]
        );
    }

    #[test]
    fn rangos_en_bytes() {
        let (tokens, _) = tokenizar("in  Señal;");
        let rangos: Vec<_> = tokens.iter().map(|t| t.rango.clone()).collect();
        // `ñ` ocupa 2 bytes y queda como error entre `Se` y `al`.
        assert_eq!(rangos, [0..2, 4..6, 8..10, 10..11, 11..11]);
    }

    #[test]
    fn errores_sin_detenerse() {
        assert_eq!(
            mensajes("0xZZ 8a @ \"abc\n /* abierto"),
            [
                "número no válido `0xZZ`",
                "número no válido `8a`",
                "carácter no válido `@`",
                "texto sin cerrar: falta `\"`",
                "comentario `/*` sin cerrar",
            ]
        );
        // Después de un error sigue leyendo.
        let (tokens, _) = tokenizar("@ out");
        assert_eq!(tokens[0].tipo, Tipo::Reservada(Reservada::Out));
    }
}
