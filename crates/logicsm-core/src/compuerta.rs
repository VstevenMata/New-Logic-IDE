/// Valor de un cable: 0, 1, X (desconocido) o Z (alta impedancia, nadie lo maneja).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Valor {
    Cero,
    Uno,
    #[default]
    X,
    Z,
}

impl Valor {
    pub fn de(b: bool) -> Self {
        if b { Valor::Uno } else { Valor::Cero }
    }

    /// `Some` si es 0 o 1
    pub fn bit(self) -> Option<bool> {
        match self {
            Valor::Cero => Some(false),
            Valor::Uno => Some(true),
            Valor::X | Valor::Z => None,
        }
    }

    pub fn simbolo(self) -> &'static str {
        match self {
            Valor::Cero => "0",
            Valor::Uno => "1",
            Valor::X => "X",
            Valor::Z => "Z",
        }
    }

    fn no(self) -> Self {
        self.bit().map_or(Valor::X, |b| Valor::de(!b))
    }
}

/// Compuertas predefinidas del lenguaje (`and(A, B)`, `not(X)`, `tristate(D, E)`…).
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
    /// `tristate(dato, habilitar)`: deja pasar el dato si habilitar = 1; si no, da Z
    Tristate,
}

use Compuerta::*;

impl Compuerta {
    pub const TODAS: [Compuerta; 9] = [And, Or, Xor, Nand, Nor, Xnor, Not, Buffer, Tristate];

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
            Tristate => "tristate",
        }
    }

    pub fn desde_nombre(nombre: &str) -> Option<Self> {
        Self::TODAS.into_iter().find(|c| c.nombre() == nombre)
    }

    /// `not` y `buffer` siempre tienen 1 entrada, `tristate` 2; las demás, 2 o más.
    pub fn entradas_validas(self, n: usize) -> bool {
        match self {
            Not | Buffer => n == 1,
            Tristate => n == 2,
            _ => n >= 2,
        }
    }

    /// Una entrada en Z se lee como X. Un 0 decide un AND y un 1 decide un OR
    /// aunque las demás sean X.
    pub fn evaluar(self, entradas: &[Valor]) -> Valor {
        self.evaluar_con(entradas.iter().copied())
    }

    /// Como `evaluar`, pero recorre las entradas sin juntarlas en un vector: el
    /// simulador lo llama por cada compuerta que cambia
    pub fn evaluar_con(self, e: impl Iterator<Item = Valor> + Clone) -> Valor {
        let hay = |v: Valor| e.clone().any(|x| x == v);
        let son_bits = || e.clone().all(|x| x.bit().is_some());
        let primera = || e.clone().next().unwrap_or(Valor::X);
        match self {
            And if hay(Valor::Cero) => Valor::Cero,
            Or if hay(Valor::Uno) => Valor::Uno,
            And | Or if !son_bits() => Valor::X,
            And => Valor::Uno,
            Or => Valor::Cero,
            Xor => e
                .clone()
                .try_fold(false, |paridad, x| Some(paridad ^ x.bit()?))
                .map_or(Valor::X, Valor::de),
            Nand => And.evaluar_con(e.clone()).no(),
            Nor => Or.evaluar_con(e.clone()).no(),
            Xnor => Xor.evaluar_con(e.clone()).no(),
            Not => primera().no(),
            Buffer => primera().no().no(),
            Tristate => match e.clone().nth(1).unwrap_or(Valor::X) {
                Valor::Uno => primera().no().no(),
                Valor::Cero => Valor::Z,
                _ => Valor::X,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Valor::{Cero, Uno, X, Z};

    /// Salidas de la tabla de verdad, de 000… a 111… (A es el bit más significativo)
    fn tabla(c: Compuerta, n: usize) -> Vec<u8> {
        (0..1 << n)
            .map(|fila| {
                let e: Vec<Valor> = (0..n)
                    .map(|i| Valor::de(fila >> (n - 1 - i) & 1 == 1))
                    .collect();
                c.evaluar(&e).bit().unwrap() as u8
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
    fn x_y_z() {
        assert_eq!(And.evaluar(&[Cero, X]), Cero);
        assert_eq!(And.evaluar(&[Uno, X]), X);
        assert_eq!(Or.evaluar(&[Uno, Z]), Uno);
        assert_eq!(Nor.evaluar(&[Cero, X]), X);
        assert_eq!(Xor.evaluar(&[Uno, X]), X);
        assert_eq!(Not.evaluar(&[Z]), X);
        assert_eq!(Tristate.evaluar(&[Uno, Uno]), Uno);
        assert_eq!(Tristate.evaluar(&[Uno, Cero]), Z);
        assert_eq!(Tristate.evaluar(&[Cero, X]), X);
    }

    #[test]
    fn nombres_y_entradas() {
        assert_eq!(Compuerta::desde_nombre("nor"), Some(Nor));
        assert_eq!(Compuerta::desde_nombre("NOR"), None);
        assert!(And.entradas_validas(5) && !And.entradas_validas(1));
        assert!(Not.entradas_validas(1) && !Not.entradas_validas(2));
        assert!(Tristate.entradas_validas(2) && !Tristate.entradas_validas(3));
    }
}
