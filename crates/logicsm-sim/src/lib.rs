//! Simulador: calcula los valores de un circuito ya compilado.

pub use logicsm_core::Compuerta;
use logicsm_core::netlist::{Netlist, Tipo};

/// Calcula el valor de cada nodo de la red.
///
/// `valores` guarda el estado entre llamadas (la memoria de los flip-flops) y
/// `entradas` va en el orden de los `in`. Devuelve `false` si el circuito oscila.
// ponytail: solo 0 y 1, empezando en 0; X, Z y retardos llegan con el simulador por eventos
pub fn simular(red: &Netlist, valores: &mut Vec<bool>, entradas: &[bool]) -> bool {
    valores.resize(red.nodos.len(), false);
    let mut entradas = entradas.iter();
    for (i, n) in red.nodos.iter().enumerate() {
        if let Tipo::Entrada(_) = n.tipo {
            valores[i] = entradas.next().copied().unwrap_or(false);
        }
    }
    // Sin lazos alcanza con una pasada por nivel; si no se estabiliza, oscila.
    for _ in 0..=red.nodos.len() {
        let mut cambio = false;
        for (i, n) in red.nodos.iter().enumerate() {
            let nuevo = match n.tipo {
                Tipo::Entrada(_) => continue,
                Tipo::Constante(v) => v,
                Tipo::Salida(_) => valores[n.entradas[0]],
                Tipo::Compuerta(c) => {
                    let e: Vec<bool> = n.entradas.iter().map(|&e| valores[e]).collect();
                    c.evaluar(&e)
                }
            };
            if nuevo != valores[i] {
                valores[i] = nuevo;
                cambio = true;
            }
        }
        if !cambio {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn salidas(red: &Netlist, valores: &[bool]) -> Vec<bool> {
        red.nodos
            .iter()
            .zip(valores)
            .filter(|(n, _)| matches!(n.tipo, Tipo::Salida(_)))
            .map(|(_, &v)| v)
            .collect()
    }

    #[test]
    fn medio_sumador() {
        let red = logicsm_core::compilar(
            "circuit HA { in A, B; out S, C; S = xor(A, B); C = and(A, B); }",
        )
        .unwrap();
        let mut v = Vec::new();
        for (a, b, s, c) in [(0, 0, 0, 0), (0, 1, 1, 0), (1, 0, 1, 0), (1, 1, 0, 1)] {
            assert!(simular(&red, &mut v, &[a == 1, b == 1]));
            assert_eq!(salidas(&red, &v), [s == 1, c == 1]);
        }
    }

    #[test]
    fn flip_flop_recuerda() {
        let red = logicsm_core::compilar(
            "circuit FF { in S, R; out Q; Q = nor(R, Qn); Qn = nor(S, Q); }",
        )
        .unwrap();
        let mut v = Vec::new();
        // (S, R) → Q: set, mantener, reset, mantener
        for (s, r, q) in [(1, 0, 1), (0, 0, 1), (0, 1, 0), (0, 0, 0)] {
            assert!(simular(&red, &mut v, &[s == 1, r == 1]));
            assert_eq!(salidas(&red, &v), [q == 1]);
        }
    }

    #[test]
    fn detecta_oscilacion() {
        let red = logicsm_core::compilar("main { out X; X = not(X); }").unwrap();
        assert!(!simular(&red, &mut Vec::new(), &[]));
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
        let mut v = Vec::new();
        for fila in 0..8u8 {
            let e: Vec<bool> = (0..3).map(|i| (fila >> (2 - i)) & 1 == 1).collect();
            let unos = fila.count_ones();
            assert!(simular(&red, &mut v, &e));
            // S = bit bajo de la suma, Cout = bit alto
            assert_eq!(
                salidas(&red, &v),
                [unos % 2 == 1, unos >= 2],
                "fila {fila:03b}"
            );
        }
    }
}
