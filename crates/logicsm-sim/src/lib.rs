//! Simulador por eventos: cada compuerta tarda 1 unidad de tiempo y solo se
//! recalcula lo que cambió. Valores de 4 estados: 0, 1, X (desconocido) y Z.

use logicsm_core::netlist::{Netlist, Tipo};
pub use logicsm_core::{Compuerta, Valor};

/// Lo que recuerda la simulación entre llamadas: el valor de cada nodo, lo que
/// cambió y espera recalcularse, y el contenido de las RAM.
#[derive(Debug, Default)]
pub struct Estado {
    /// Valor de cada nodo de la red
    pub valores: Vec<Valor>,
    /// Unidades de tiempo simuladas
    pub tiempo: u64,
    /// Nodos que leen a cada nodo
    abanico: Vec<Vec<usize>>,
    /// Nodos por recalcular en el próximo paso, sin repetir
    pendientes: Vec<usize>,
    en_pendientes: Vec<bool>,
    /// Contenido de cada RAM (vacío en las ROM)
    ram: Vec<Vec<u64>>,
    /// Último valor del reloj de cada memoria, para escribir en el flanco de subida
    reloj: Vec<Valor>,
}

/// Los nodos que se recalculan con retardo; entradas y constantes cambian al
/// instante y los visores no dan señal.
fn con_retardo(t: &Tipo) -> bool {
    !matches!(t, Tipo::Entrada(..) | Tipo::Constante(_) | Tipo::Visor(..))
}

/// El número que forman los bits (el primero es el bit 0); `None` si alguno no es 0 o 1.
fn numero(bits: impl Iterator<Item = Valor>) -> Option<u64> {
    bits.enumerate()
        .try_fold(0, |n, (k, v)| Some(n | ((v.bit()? as u64) << k)))
}

/// Pasos máximos para asentarse: sin lazos alcanza con uno por nivel y los
/// flip-flops se asientan enseguida
fn limite(red: &Netlist) -> usize {
    4 * red.nodos.len() + 16
}

impl Estado {
    /// Todo en `inicial` (las constantes con su valor), memorias en 0 y todo por recalcular.
    fn preparar(&mut self, red: &Netlist, inicial: Valor) {
        let n = red.nodos.len();
        self.valores = red
            .nodos
            .iter()
            .map(|nodo| match nodo.tipo {
                Tipo::Constante(v) => Valor::de(v),
                _ => inicial,
            })
            .collect();
        self.abanico = vec![Vec::new(); n];
        for (i, nodo) in red.nodos.iter().enumerate() {
            for &e in &nodo.entradas {
                self.abanico[e].push(i);
            }
        }
        self.pendientes = (0..n)
            .filter(|&i| con_retardo(&red.nodos[i].tipo))
            .collect();
        self.en_pendientes = (0..n).map(|i| con_retardo(&red.nodos[i].tipo)).collect();
        self.ram = red
            .memorias
            .iter()
            .map(|m| {
                if m.ram {
                    vec![0; 1 << m.direccion]
                } else {
                    Vec::new()
                }
            })
            .collect();
        self.reloj = vec![inicial; red.memorias.len()];
        self.tiempo = 0;
    }

    /// Botón Reset: todo en 0 (entradas incluidas), también las RAM. Después se
    /// asienta cambiando cada nodo al instante y en orden: con todo en 0 un
    /// flip-flop arranca simétrico (Q = Qn = 0) y con retardo oscilaría para siempre.
    pub fn reiniciar(&mut self, red: &Netlist) {
        self.preparar(red, Valor::Cero);
        for _ in 0..limite(red) {
            let mut cambio = false;
            for (i, nodo) in red.nodos.iter().enumerate() {
                if con_retardo(&nodo.tipo) {
                    let v = self.calcular(red, i);
                    cambio |= v != self.valores[i];
                    self.valores[i] = v;
                }
            }
            if !cambio {
                self.pendientes.clear();
                self.en_pendientes.fill(false);
                return;
            }
        }
    }

    /// Pone las entradas (en el orden de los `in`, `button` y `clock`) y avanza una
    /// unidad de tiempo. Devuelve `false` si no había nada por recalcular.
    pub fn paso(&mut self, red: &Netlist, entradas: &[bool]) -> bool {
        // Al empezar, o si la red cambió, todo arranca en X
        if self.valores.len() != red.nodos.len() {
            self.preparar(red, Valor::X);
        }
        let mut entradas = entradas.iter();
        for (i, nodo) in red.nodos.iter().enumerate() {
            if let Tipo::Entrada(..) = nodo.tipo {
                let v = Valor::de(entradas.next().copied().unwrap_or(false));
                if self.valores[i] != v {
                    self.valores[i] = v;
                    self.agendar(red, i);
                }
            }
        }
        if self.pendientes.is_empty() {
            return false;
        }
        // Todos se calculan con los valores de antes y después cambian juntos
        let pendientes = std::mem::take(&mut self.pendientes);
        let mut nuevos = Vec::with_capacity(pendientes.len());
        for &i in &pendientes {
            self.en_pendientes[i] = false;
            nuevos.push(self.calcular(red, i));
        }
        for (&i, v) in pendientes.iter().zip(nuevos) {
            if self.valores[i] != v {
                self.valores[i] = v;
                self.agendar(red, i);
            }
        }
        self.tiempo += 1;
        true
    }

    /// Lo que lee al nodo `i` se recalcula en el próximo paso
    fn agendar(&mut self, red: &Netlist, i: usize) {
        for &j in &self.abanico[i] {
            if !self.en_pendientes[j] && con_retardo(&red.nodos[j].tipo) {
                self.en_pendientes[j] = true;
                self.pendientes.push(j);
            }
        }
    }

    /// El valor que debe tener el nodo `i` según sus entradas
    fn calcular(&mut self, red: &Netlist, i: usize) -> Valor {
        let nodo = &red.nodos[i];
        let v = |k: usize| self.valores[nodo.entradas[k]];
        match nodo.tipo {
            Tipo::Salida(_) => v(0),
            Tipo::Compuerta(c) => {
                let e: Vec<Valor> = nodo.entradas.iter().map(|&e| self.valores[e]).collect();
                c.evaluar(&e)
            }
            // Ninguno conduce: Z; uno solo, o todos de acuerdo: ese valor; si no, X
            Tipo::Bus => nodo
                .entradas
                .iter()
                .map(|&e| self.valores[e])
                .filter(|&x| x != Valor::Z)
                .try_fold(Valor::Z, |total, x| match total {
                    Valor::Z => Some(x),
                    _ if total == x => Some(x),
                    _ => None,
                })
                .unwrap_or(Valor::X),
            Tipo::Memoria { memoria, bit } => {
                let m = &red.memorias[memoria];
                let d = m.direccion;
                let direccion = numero((0..d).map(v)).map(|a| a as usize);
                let palabra = if m.ram {
                    let dato = numero((d..d + m.dato).map(v));
                    let (escribir, reloj) = (v(d + m.dato), v(d + m.dato + 1));
                    // ponytail: con la dirección o el dato en X no escribe nada
                    if self.reloj[memoria] == Valor::Cero
                        && reloj == Valor::Uno
                        && escribir == Valor::Uno
                        && let (Some(a), Some(x)) = (direccion, dato)
                    {
                        self.ram[memoria][a] = x;
                    }
                    self.reloj[memoria] = reloj;
                    direccion.map(|a| self.ram[memoria][a])
                } else {
                    direccion.map(|a| m.datos.get(a).copied().unwrap_or(0))
                };
                palabra.map_or(Valor::X, |p| Valor::de((p >> bit) & 1 == 1))
            }
            Tipo::Entrada(..) | Tipo::Constante(_) | Tipo::Visor(..) => self.valores[i],
        }
    }
}

/// Pone las entradas y avanza hasta que nada cambie. Devuelve `false` si el
/// circuito oscila (sigue cambiando después de muchos pasos).
pub fn simular(red: &Netlist, estado: &mut Estado, entradas: &[bool]) -> bool {
    for _ in 0..limite(red) {
        if !estado.paso(red, entradas) {
            return true;
        }
    }
    estado.pendientes.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Valor::{Cero, Uno, X, Z};

    fn salidas(red: &Netlist, estado: &Estado) -> Vec<Valor> {
        red.nodos
            .iter()
            .zip(&estado.valores)
            .filter(|(n, _)| matches!(n.tipo, Tipo::Salida(_)))
            .map(|(_, &v)| v)
            .collect()
    }

    fn bits(texto: &str) -> Vec<Valor> {
        texto
            .chars()
            .map(|c| match c {
                '0' => Cero,
                '1' => Uno,
                'Z' => Z,
                _ => X,
            })
            .collect()
    }

    #[test]
    fn medio_sumador() {
        let red = logicsm_core::compilar(
            "circuit HA { in A, B; out S, C; S = xor(A, B); C = and(A, B); }",
        )
        .unwrap();
        let mut e = Estado::default();
        for (a, b, s, c) in [(0, 0, 0, 0), (0, 1, 1, 0), (1, 0, 1, 0), (1, 1, 0, 1)] {
            assert!(simular(&red, &mut e, &[a == 1, b == 1]));
            assert_eq!(salidas(&red, &e), [Valor::de(s == 1), Valor::de(c == 1)]);
        }
    }

    #[test]
    fn flip_flop_arranca_en_x_y_recuerda() {
        let red = logicsm_core::compilar(
            "circuit FF { in S, R; out Q; Q = nor(R, Qn); Qn = nor(S, Q); }",
        )
        .unwrap();
        let mut e = Estado::default();
        // Sin set ni reset no se sabe qué guarda
        assert!(simular(&red, &mut e, &[false, false]));
        assert_eq!(salidas(&red, &e), [X]);
        // (S, R) → Q: set, mantener, reset, mantener
        for (s, r, q) in [(1, 0, 1), (0, 0, 1), (0, 1, 0), (0, 0, 0)] {
            assert!(simular(&red, &mut e, &[s == 1, r == 1]));
            assert_eq!(salidas(&red, &e), [Valor::de(q == 1)]);
        }
        // Reset pone todo en 0 y el lazo queda en un valor conocido
        e.reiniciar(&red);
        assert!(simular(&red, &mut e, &[false, false]));
        assert_ne!(salidas(&red, &e), [X]);
    }

    #[test]
    fn retardo_de_una_unidad_por_compuerta() {
        let red = logicsm_core::compilar("main { in A; out X; X = not(not(A)); }").unwrap();
        let mut e = Estado::default();
        assert!(simular(&red, &mut e, &[false]));
        let t = e.tiempo;
        // A cambia: dos compuertas y la salida, una unidad cada una
        assert!(simular(&red, &mut e, &[true]));
        assert_eq!(e.tiempo - t, 3);
        assert_eq!(salidas(&red, &e), [Uno]);
    }

    #[test]
    fn detecta_oscilacion() {
        let red = logicsm_core::compilar("main { out X; X = not(X); }").unwrap();
        let mut e = Estado::default();
        // Desde X se queda en X; después de Reset alterna sin parar
        assert!(simular(&red, &mut e, &[]));
        e.reiniciar(&red);
        assert!(!simular(&red, &mut e, &[]));
    }

    #[test]
    fn tristate_en_un_bus() {
        let red = logicsm_core::compilar(
            "main { in A, B, EA, EB; out Y;
             Y = tristate(A, EA); Y = tristate(B, EB); }",
        )
        .unwrap();
        let mut e = Estado::default();
        // A, B, EA, EB → Y
        for (entradas, y) in [
            ("0000", Z),
            ("1010", Uno),
            ("1001", Cero),
            ("1011", X),
            ("0011", Cero),
        ] {
            let v: Vec<bool> = entradas.chars().map(|c| c == '1').collect();
            assert!(simular(&red, &mut e, &v));
            assert_eq!(salidas(&red, &e), [y], "{entradas}");
        }
    }

    #[test]
    fn rom_y_ram() {
        let red = logicsm_core::compilar(
            "main { in A[2], D[2], W, C; out X[3], Y[2];
             R = rom.size(2, 3, \"5 7 1\");
             M = ram.size(2, 2);
             X = R(A);
             Y = M(A, D, W, C); }",
        )
        .unwrap();
        let mut e = Estado::default();
        // Entradas: A[0], A[1], D[0], D[1], W, C. Salidas: X[0..2], Y[0..1]
        let mut paso = |entradas: &str| {
            let v: Vec<bool> = entradas.chars().map(|c| c == '1').collect();
            assert!(simular(&red, &mut e, &v));
            salidas(&red, &e)
        };
        // Dirección 0: ROM = 5 (101); la RAM empieza en 0
        assert_eq!(paso("000000"), bits("10100"));
        // Dirección 1 (A[0] = 1): ROM = 7; dirección 3 no tiene dato: 0
        assert_eq!(paso("100000")[..3], bits("111"));
        assert_eq!(paso("110000")[..3], bits("000"));
        // Escribe 2 (D[1] = 1) en la dirección 1 con un flanco de subida del reloj
        paso("100110");
        assert_eq!(paso("100111")[3..], bits("01"));
        // Sin flanco no escribe: cambiar el dato con el reloj arriba no hace nada
        assert_eq!(paso("101111")[3..], bits("01"));
        // Otra dirección sigue en 0 y la 1 conserva el dato
        assert_eq!(paso("000000")[3..], bits("00"));
        assert_eq!(paso("100000")[3..], bits("01"));
    }

    #[test]
    fn sumador_completo_con_dos_medios_sumadores() {
        let red = logicsm_core::compilar(
            "circuit MedioSumador { in A, B; out Suma, Acarreo; Suma = xor(A, B); Acarreo = and(A, B); }
             circuit SumadorCompleto {
                 in A, B, Cin; out S, Cout;
                 m1 = MedioSumador(A, B);
                 m2 = MedioSumador(m1.Suma, Cin);
                 S = m2.Suma;
                 Cout = or(m1.Acarreo, m2.Acarreo);
             }",
        )
        .unwrap();
        let mut e = Estado::default();
        for fila in 0..8u8 {
            let v: Vec<bool> = (0..3).map(|i| (fila >> (2 - i)) & 1 == 1).collect();
            let unos = fila.count_ones();
            assert!(simular(&red, &mut e, &v));
            // S = bit bajo de la suma, Cout = bit alto
            assert_eq!(
                salidas(&red, &e),
                [Valor::de(unos % 2 == 1), Valor::de(unos >= 2)],
                "fila {fila:03b}"
            );
        }
    }

    #[test]
    fn sumador_de_4_bits_con_arreglo_y_for() {
        let red = logicsm_core::compilar(
            "circuit FullAdder { in A, B, Cin; out Sum, Cout;
                 Sum = xor(A, B, Cin); Cout = or(and(A, B), and(Cin, xor(A, B))); }
             circuit Adder4 {
                 in A[4], B[4], Cin; out S[4], Cout;
                 FullAdder fa[4];
                 for (i = 0; i < 4; i++) { fa[i].A = A[i]; fa[i].B = B[i]; S[i] = fa[i].Sum; }
                 fa[0].Cin = Cin;
                 for (i = 1; i < 4; i++) { fa[i].Cin = fa[i-1].Cout; }
                 Cout = fa[3].Cout;
             }",
        )
        .unwrap();
        let a_bits = |n: u32| (0..4).map(move |k| (n >> k) & 1 == 1);
        let mut e = Estado::default();
        for (a, b, c) in [(5, 6, 0), (15, 1, 0), (9, 9, 1), (0, 0, 0)] {
            // Entradas en orden: A[0..3], B[0..3], Cin
            let v: Vec<bool> = a_bits(a).chain(a_bits(b)).chain([c == 1]).collect();
            assert!(simular(&red, &mut e, &v));
            let total = a + b + c;
            // Salidas en orden: S[0..3], Cout
            let esperado: Vec<Valor> = a_bits(total).chain([total >= 16]).map(Valor::de).collect();
            assert_eq!(salidas(&red, &e), esperado, "{a} + {b} + {c}");
        }
    }
}
