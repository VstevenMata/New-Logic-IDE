/// Compuertas predefinidas del lenguaje (`and(A, B)`, `not(X)`…).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compuerta {
    And,
    Or,
    Xor,
    Nand,
    Nor,
    Xnor,
    Not,
    Buffer,
}

use Compuerta::*;

impl Compuerta {
    pub const TODAS: [Compuerta; 8] = [And, Or, Xor, Nand, Nor, Xnor, Not, Buffer];

    pub fn nombre(self) -> &'static str {
        match self {
            And => "and",
            Or => "or",
            Xor => "xor",
            Nand => "nand",
            Nor => "nor",
            Xnor => "xnor",
            Not => "not",
            Buffer => "buffer",
        }
    }

    pub fn desde_nombre(nombre: &str) -> Option<Self> {
        Self::TODAS.into_iter().find(|c| c.nombre() == nombre)
    }

    /// `not` y `buffer` siempre tienen 1 entrada; las demás, 2 o más.
    pub fn entradas_validas(self, n: usize) -> bool {
        match self {
            Not | Buffer => n == 1,
            _ => n >= 2,
        }
    }

    // ponytail: solo 0 y 1; X y Z llegan con el simulador por eventos
    pub fn evaluar(self, entradas: &[bool]) -> bool {
        let unos = entradas.iter().filter(|&&e| e).count();
        match self {
            And => unos == entradas.len(),
            Or => unos > 0,
            Xor => unos % 2 == 1,
            Nand => unos != entradas.len(),
            Nor => unos == 0,
            Xnor => unos % 2 == 0,
            Not => unos == 0,
            Buffer => unos == 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Salidas de la tabla de verdad, de 000… a 111… (A es el bit más significativo)
    fn tabla(c: Compuerta, n: usize) -> Vec<u8> {
        (0..1 << n)
            .map(|fila| {
                let e: Vec<bool> = (0..n).map(|i| fila >> (n - 1 - i) & 1 == 1).collect();
                c.evaluar(&e) as u8
            })
            .collect()
    }

    #[test]
    fn tablas_de_verdad() {
        assert_eq!(tabla(And, 3), [0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(tabla(Or, 2), [0, 1, 1, 1]);
        assert_eq!(tabla(Xor, 3), [0, 1, 1, 0, 1, 0, 0, 1]);
        assert_eq!(tabla(Nand, 2), [1, 1, 1, 0]);
        assert_eq!(tabla(Nor, 3), [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(tabla(Xnor, 2), [1, 0, 0, 1]);
        assert_eq!(tabla(Not, 1), [1, 0]);
        assert_eq!(tabla(Buffer, 1), [0, 1]);
    }

    #[test]
    fn nombres_y_entradas() {
        assert_eq!(Compuerta::desde_nombre("nor"), Some(Nor));
        assert_eq!(Compuerta::desde_nombre("NOR"), None);
        assert!(And.entradas_validas(5) && !And.entradas_validas(1));
        assert!(Not.entradas_validas(1) && !Not.entradas_validas(2));
    }
}
