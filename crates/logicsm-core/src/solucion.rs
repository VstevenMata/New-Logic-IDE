//! Solución desde una tabla de verdad: una suma de productos chica que la cumple,
//! ya escrita como circuito.

/// Un producto: por cada entrada, `Some(true)` si va tal cual, `Some(false)` si va
/// negada y `None` si no aparece.
pub type Producto = Vec<Option<bool>>;

/// Productos cuya suma da 1 en las filas `unos` y 0 en las que no están ni en `unos`
/// ni en `libres` (las que dan igual). Como en la tabla de verdad, la fila `f` tiene la
/// primera de las `n` entradas en el bit más alto.
pub fn minimizar(n: usize, unos: &[usize], libres: &[usize]) -> Vec<Producto> {
    let filas = 1usize << n;
    let mut permitida = vec![false; filas];
    for &f in unos.iter().chain(libres) {
        permitida[f] = true;
    }
    // Un grupo de filas es (bits fijos, valor de esos bits)
    let implicante = |fijos: usize, valor: usize| {
        (0..filas)
            .filter(|f| f & fijos == valor)
            .all(|f| permitida[f])
    };
    // Primos: implicantes que dejan de serlo si se suelta cualquiera de sus bits
    let mut primos = Vec::new();
    for fijos in 0..filas {
        let mut valor = fijos;
        loop {
            let primo = implicante(fijos, valor)
                && (0..n)
                    .filter(|b| fijos >> b & 1 == 1)
                    .all(|b| !implicante(fijos & !(1 << b), valor & !(1 << b)));
            if primo && unos.iter().any(|f| f & fijos == valor) {
                primos.push((fijos, valor));
            }
            if valor == 0 {
                break;
            }
            valor = (valor - 1) & fijos;
        }
    }

    // Primero los esenciales (únicos que cubren alguna fila), después la menor cantidad
    // de primos que cubre lo que falta
    let cubre = |(fijos, valor): (usize, usize), f: usize| f & fijos == valor;
    let mut elegidos: Vec<(usize, usize)> = Vec::new();
    for &f in unos {
        let mut los_que_cubren = primos.iter().filter(|&&p| cubre(p, f));
        if let (Some(&p), None) = (los_que_cubren.next(), los_que_cubren.next())
            && !elegidos.contains(&p)
        {
            elegidos.push(p);
        }
    }
    let falta: Vec<usize> = unos
        .iter()
        .copied()
        .filter(|&f| !elegidos.iter().any(|&p| cubre(p, f)))
        .collect();
    // Una solución voraz (el que cubre más de lo que falta, con menos entradas) sirve de
    // cota para buscar la mínima
    let mut voraz = Vec::new();
    let mut quedan = falta.clone();
    while let Some(&mejor) = primos.iter().max_by_key(|&&p| {
        let nuevas = quedan.iter().filter(|&&f| cubre(p, f)).count();
        (nuevas, std::cmp::Reverse(p.0.count_ones()))
    }) {
        if quedan.is_empty() {
            break;
        }
        quedan.retain(|&f| !cubre(mejor, f));
        voraz.push(mejor);
    }
    let mut mejor = Some(voraz);
    cubrir(&falta, &primos, &mut Vec::new(), &mut mejor);
    elegidos.extend(mejor.unwrap_or_default());

    elegidos
        .into_iter()
        .map(|(fijos, valor)| {
            (0..n)
                .map(|i| {
                    let b = n - 1 - i;
                    (fijos >> b & 1 == 1).then_some(valor >> b & 1 == 1)
                })
                .collect()
        })
        .collect()
}

/// Busca en `mejor` los primos que cubren todas las filas de `falta` con la menor
/// cantidad y, si empatan, con menos entradas: prueba cada primo que cubre la primera
/// fila sin cubrir (lo mismo que el método de Petrick, sin desarrollar el producto).
// ponytail: exponencial en el peor caso; la tabla de verdad llega a 8 entradas y la cota
// voraz poda casi todo
fn cubrir(
    falta: &[usize],
    primos: &[(usize, usize)],
    elegidos: &mut Vec<(usize, usize)>,
    mejor: &mut Option<Vec<(usize, usize)>>,
) {
    let costo = |v: &[(usize, usize)]| {
        let entradas: u32 = v.iter().map(|p| p.0.count_ones()).sum();
        (v.len(), entradas)
    };
    let cubre = |(fijos, valor): (usize, usize), f: usize| f & fijos == valor;
    let sin_cubrir = falta
        .iter()
        .find(|&&f| !elegidos.iter().any(|&p| cubre(p, f)));
    let Some(&f) = sin_cubrir else {
        if mejor.as_ref().is_none_or(|m| costo(elegidos) < costo(m)) {
            *mejor = Some(elegidos.clone());
        }
        return;
    };
    // Falta al menos uno más: con tantos como la mejor ya no se gana
    if mejor.as_ref().is_some_and(|m| elegidos.len() >= m.len()) {
        return;
    }
    for &p in primos.iter().filter(|&&p| cubre(p, f)) {
        elegidos.push(p);
        cubrir(falta, primos, elegidos, mejor);
        elegidos.pop();
    }
}

/// El circuito `nombre` con esas entradas y salidas (un nombre por bit: `A`, `B[0]`...)
/// y la suma de productos de cada salida.
pub fn escribir(
    nombre: &str,
    entradas: &[String],
    salidas: &[String],
    productos: &[Vec<Producto>],
) -> String {
    let mut codigo = format!("circuit {nombre} {{\n");
    codigo += &format!("    in {};\n", puertos(entradas));
    codigo += &format!("    out {};\n", puertos(salidas));
    for (salida, suma) in salidas.iter().zip(productos) {
        codigo += &format!("    {salida} = {};\n", expresion(entradas, suma));
    }
    codigo + "}\n"
}

/// `A`, `B[0]`, `B[1]` → `A, B[2]`
fn puertos(bits: &[String]) -> String {
    let mut grupos: Vec<(&str, usize)> = Vec::new();
    for b in bits {
        let base = b.split('[').next().unwrap_or(b);
        match grupos.last_mut() {
            Some((anterior, ancho)) if *anterior == base && b.contains('[') => *ancho += 1,
            _ => grupos.push((base, usize::from(b.contains('[')))),
        }
    }
    grupos
        .iter()
        .map(|&(base, ancho)| match ancho {
            0 => base.to_string(),
            n => format!("{base}[{n}]"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn expresion(entradas: &[String], suma: &[Producto]) -> String {
    let producto = |p: &Producto| {
        let literales: Vec<String> = entradas
            .iter()
            .zip(p)
            .filter_map(|(e, v)| match v {
                Some(true) => Some(e.clone()),
                Some(false) => Some(format!("not({e})")),
                None => None,
            })
            .collect();
        match literales.len() {
            0 => "1".to_string(),
            1 => literales[0].clone(),
            _ => format!("and({})", literales.join(", ")),
        }
    };
    let terminos: Vec<String> = suma.iter().map(producto).collect();
    match terminos.len() {
        0 => "0".to_string(),
        1 => terminos[0].clone(),
        _ => format!("or({})", terminos.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_y_casos_extremos() {
        // A B → X: xor
        let p = minimizar(2, &[1, 2], &[]);
        assert_eq!(p.len(), 2);
        let entradas = ["A".to_string(), "B".to_string()];
        assert_eq!(
            expresion(&entradas, &p),
            "or(and(not(A), B), and(A, not(B)))"
        );
        // Nunca 1, siempre 1, y una fila libre que permite simplificar: A or B
        assert_eq!(expresion(&entradas, &minimizar(2, &[], &[])), "0");
        assert_eq!(expresion(&entradas, &minimizar(2, &[0, 1, 2, 3], &[])), "1");
        assert_eq!(
            expresion(&entradas, &minimizar(2, &[1, 2], &[3])),
            "or(B, A)"
        );
    }

    #[test]
    fn minimo_aunque_no_haya_esenciales() {
        // 0, 1, 2, 5, 6, 7: seis primos de dos filas, ninguno esencial. Elegir por
        // cantidad puede dar 4; el mínimo es 3 (como A'B' + BC' + AC)
        let p = minimizar(3, &[0, 1, 2, 5, 6, 7], &[]);
        assert_eq!(p.len(), 3, "{p:?}");
        // Y cada producto tiene 2 entradas
        assert!(p.iter().all(|t| t.iter().flatten().count() == 2));
    }

    #[test]
    fn circuito_con_buses() {
        let entradas: Vec<String> = ["S", "D[0]", "D[1]"].map(String::from).to_vec();
        let salidas = vec!["Y".to_string()];
        // Multiplexor: S = 0 → D[0], S = 1 → D[1]
        let unos: Vec<usize> = (0..8)
            .filter(|f| if f & 4 == 0 { f & 2 != 0 } else { f & 1 != 0 })
            .collect();
        let codigo = escribir("Mux", &entradas, &salidas, &[minimizar(3, &unos, &[])]);
        assert!(codigo.starts_with("circuit Mux {\n    in S, D[2];\n    out Y;\n"));
        assert!(crate::compilar(&codigo).is_ok(), "{codigo}");
    }
}
