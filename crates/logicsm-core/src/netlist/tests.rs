use crate::compilar;
use crate::netlist::{Clase, Tipo, Visor};

fn errores(fuente: &str) -> Vec<String> {
    compilar(fuente)
        .unwrap_err()
        .into_iter()
        .map(|e| e.mensaje)
        .collect()
}

const MEDIO_SUMADOR: &str =
    "circuit MedioSumador { in A, B; out Suma, Acarreo; Suma = xor(A, B); Acarreo = and(A, B); }";

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
    let red = compilar("circuit FF { in S, R; out Q; Q = nor(R, Qn); Qn = nor(S, Q); }").unwrap();
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
    let red =
        compilar("circuit S { in A[2]; out a0, a1; a = split.derecha(A); a0 = a[0]; a1 = a[1]; }")
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
        con("matrix M(2, 3, A, B);"),
        ["`matrix` lleva 5 bits: 2 filas y 3 columnas (o 6 bits, uno por LED), pero recibe 3 bits"]
    );
    assert!(compilar("main { in A[2], B; matrix M(1, 3, A, B); }").is_ok());
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
        ["las entradas de `and` deben tener el mismo ancho (o 1 bit): hay de 4 bits y de 2 bits"]
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
