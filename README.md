# Logic_F-4:13

Lenguaje de programación y entorno de desarrollo de escritorio para **diseñar, visualizar y simular circuitos digitales**.

Inspirado en LogicCircuit, pero con una diferencia clave: los circuitos y componentes se **escriben en código**, y la aplicación dibuja y simula el circuito automáticamente.

> Estado: diseño. Todavía no hay implementación.

---

## Objetivo

Que un estudiante pueda:

1. Escribir un circuito en Logic_F-4:13.
2. Compilarlo y ver el circuito dibujado automáticamente.
3. Activar entradas (interruptores, pulsadores, reloj) y ver las señales cambiar en tiempo real.
4. Crear sus propios componentes (un flip-flop, un sumador…) y reutilizarlos en otros circuitos.
5. Construir sistemas cada vez más grandes por composición: compuertas → half adder → full adder → ALU → CPU educativa.

---

## El lenguaje

Sintaxis estilo C++ (llaves y `;`), pero con muy pocas reglas.

### Compuertas: se declaran una vez y se usan infinitas veces

Como arrastrar un componente desde la paleta en LogicCircuit: se configura un **molde** con `.set(entradas)` y se llama las veces que se quiera. Cada llamada es una compuerta nueva e independiente.

```cpp
A3 = and.set(3);        // molde: AND de 3 entradas
N  = nor.set(2);        // molde: NOR de 2 entradas

circuit Ejemplo {
    in A, B, C, D;
    out X, Y;

    X = A3(A, B, C);    // compuerta 1
    Y = A3(B, C, D);    // compuerta 2 (otra distinta)
}
```

### Circuitos propios: se usan igual que las compuertas

```cpp
circuit FlipFlop {
    in S, R;
    out Q, Qn;

    N = nor.set(2);
    Q  = N(R, Qn);      // realimentación: Q depende de Qn y viceversa
    Qn = N(S, Q);
}
```

```cpp
main {
    in Set, Reset;
    out Led;

    ff = FlipFlop(Set, Reset);   // instancia independiente
    Led = ff.Q;                  // salidas con .nombre
}
```

### Reglas del proyecto

- **Sin `import`:** cualquier circuito de un archivo `.lx` en la carpeta raíz del proyecto se puede usar por su nombre.
- **Por referencia:** si se modifica `FlipFlop.lx`, todos los circuitos que lo usan reflejan el cambio al recompilar.
- **Se compila el circuito seleccionado**, no solo `main`. Al abrir `FlipFlop.lx` y presionar ▶ se simula ese circuito, y sus entradas aparecen como interruptores. Solo se compila ese circuito y sus dependencias, así que un error en otro archivo no bloquea la prueba.
- `main` es opcional: es solo el circuito que se abre por defecto.
- **Constantes:** `0` y `1` se pueden pasar donde va una señal, por ejemplo `FlipFlop(1, 0)` o `A3(A, B, 1)`.

```
mi-proyecto/
├── main.lx
├── FlipFlop.lx
└── Contador.lx
```

---

## Componentes

Hay dos formas de escribirlos:

1. **Compuertas y componentes internos:** molde `X = tipo.set(...);` y luego `X(...)`.
2. **Dispositivos de entrada/salida** (los que se ven o se tocan): `tipo nombre(...);`.

### Entradas

| Componente  | Código            | Comportamiento                                    |
|-------------|-------------------|---------------------------------------------------|
| Interruptor | `in A;`           | Un click lo enciende y queda encendido            |
| Pulsador    | `button B;`       | Vale 1 solo mientras se mantiene presionado       |
| Constante   | `0`, `1`          | Directo donde va una señal                        |
| Reloj       | `clock CLK(2);`   | Alterna 0/1 a 2 Hz (ajustable desde el panel)     |

Si un circuito con `in`/`button` se usa dentro de otro, esas entradas se comportan como pines normales.

### Salidas y visualización

| Componente          | Código                                   | Comportamiento                      |
|---------------------|------------------------------------------|-------------------------------------|
| Salida              | `out Y;`                                 | Pin de salida del circuito          |
| LED                 | `led L(Q);`                              | Encendido cuando Q = 1              |
| Display 7 segmentos | `display7 D(a, b, c, d, e, f, g, dp);`   | Una señal por segmento              |
| Matriz de LEDs      | `matrix M(8, 8, filas, cols);`           | Pantalla de puntos                  |
| Sonda / osciloscopio| `probe P(CLK, Q);`                       | Diagrama de ondas en el tiempo      |

### Compuertas y componentes internos

| Componente                     | Código                                          |
|--------------------------------|-------------------------------------------------|
| AND, OR, XOR, NAND, NOR, XNOR  | `A3 = and.set(3);`                              |
| NOT, buffer                    | `not(X)`, `buffer(X)` (siempre 1 entrada)       |
| Buffer tri-state               | `T = tristate.set(1);` → `T(dato, habilitar)`   |
| ROM                            | `R = rom.set(4, 8, "datos.hex");` (dir, dato)   |
| RAM                            | `M = ram.set(4, 8);` → `M(dir, dato, escribir, clk)` |

**Buses y splitters:** `in A[8];` declara un bus. `A[0]`, `A[0..3]` y `{a, b, c}` separan y unen bits. En el dibujo se ven como splitters.

### Ejemplo

```cpp
circuit Demo {
    clock CLK(1);
    button Reset;

    c = Contador(CLK, Reset);    // circuito propio (Contador.lx)
    d = DecoHex(c.Q);            // circuito propio: 4 bits → 7 segmentos

    display7 D(d.a, d.b, d.c, d.d, d.e, d.f, d.g, 0);
    led Par(not(c.Q[0]));
    probe Ondas(CLK, c.Q[0], c.Q[1]);
}
```

---

## Gramática (EBNF)

```ebnf
(* ---------- Archivo ---------- *)
file        = { item } EOF ;
item        = circuit | assign ;              (* assign afuera = molde global *)

circuit     = ( "circuit" IDENT | "main" ) "{" { stmt } "}" ;

(* ---------- Sentencias ---------- *)
stmt        = port_decl
            | device_decl
            | assign ;

port_decl   = ( "in" | "out" | "button" ) port { "," port } ";" ;
port        = IDENT [ "[" INT "]" ] ;         (* A  o  A[8] = bus de 8 bits *)

device_decl = DEVICE IDENT "(" [ args ] ")" ";" ;
DEVICE      = "clock" | "led" | "display7" | "matrix" | "probe" ;

assign      = target "=" expr ";" ;
target      = IDENT [ index ] ;

(* ---------- Expresiones ---------- *)
expr        = postfix | concat ;
concat      = "{" expr { "," expr } "}" ;

postfix     = primary { "." IDENT
                      | "(" [ args ] ")"
                      | index } ;

primary     = IDENT | INT | STRING | "(" expr ")" ;
index       = "[" INT [ ".." INT ] "]" ;      (* rango inclusivo *)
args        = expr { "," expr } ;

(* ---------- Léxico ---------- *)
IDENT       = letter { letter | digit | "_" } ;
INT         = digit { digit } | "0x" hex { hex } | "0b" bin { bin } ;
STRING      = '"' { char - '"' } '"' ;
letter      = "A".."Z" | "a".."z" | "_" ;
comment     = "//" { char } newline | "/*" { char } "*/" ;
```

**Palabras reservadas (10):** `circuit`, `main`, `in`, `out`, `button`, `clock`, `led`, `display7`, `matrix`, `probe`.

`and`, `or`, `not`, `rom`, `ram`, etc. **no** son palabras reservadas: son nombres predefinidos. Agregar un componente nuevo no requiere tocar el parser.

### Reglas semánticas

1. **El orden no importa** dentro de un circuito: una señal se puede usar antes de definirse (necesario para la realimentación).
2. **Asignar a un nombre nuevo lo crea:** molde (`N = nor.set(2);`), instancia (`ff = FlipFlop(S, R);`) o señal (`x = N(A, B);`).
3. **Un solo conductor por señal:** asignar dos veces la misma señal es error (excepto salidas tri-state en un bus).
4. **Cantidades y anchos deben coincidir:** `A3(A, B)` es error (espera 3 entradas).
5. **Resolución de nombres:** circuito actual → moldes globales del archivo → circuitos de la carpeta raíz → predefinidos.
6. Todo `out` debe estar asignado. Entradas sin usar generan advertencia.

---

## Arquitectura

### Tecnología

- **Rust** en todo el proyecto.
- **egui / eframe** para la interfaz nativa de escritorio (Windows, Linux, macOS). Sin webview ni React: el simulador y el lienzo comparten memoria directamente, sin IPC.
- El resultado es un solo ejecutable por plataforma. Como extra, eframe compila a WASM, lo que permite una versión web.

### Pipeline del compilador

```
código .lx → Lexer → Parser → AST → Análisis semántico → Elaboración → Circuit IR
                                                                          │
                                                   ┌──────────────────────┼─────────────┐
                                                Simulador            Visualizador   (futuro: Verilog,
                                                                                     tablas de verdad)
```

- **Parser** escrito a mano (descenso recursivo), para tener buenos mensajes de error.
- **Elaboración:** resuelve moldes, instancia subcircuitos, expande buses y verifica anchos. El IR solo contiene nodos concretos conectados.
- **Circuit IR:** netlist independiente de la interfaz gráfica.

### Simulador

- Valores de 4 estados: **0, 1, X** (desconocido) y **Z** (alta impedancia, para tri-state).
- **Por eventos**, con soporte de **realimentación**: los flip-flops se construyen con compuertas. Itera hasta que las señales se estabilizan y avisa si un circuito oscila.
- Jerarquía aplanada para simular (`ff.N1`, `ff.N2`…) y conservada para visualizar.

### Visualización

- **Layout por capas**, al estilo de una red neuronal: entradas a la izquierda, compuertas en columnas según su profundidad y salidas a la derecha. El orden dentro de cada columna minimiza cruces de cables.
- Cables en **ángulo recto**. Los cables de realimentación rodean por arriba o por abajo.
- Compuertas con su forma clásica. Los circuitos propios se dibujan como una caja con pines; con doble click se abre su interior con los valores de esa instancia.
- Colores: **verde = 1**, **gris = 0**, **rojo = X**.

### Workspace

```
New-Logic-IDE/
├── Cargo.toml              # workspace
├── crates/
│   ├── logicf4-core/       # lexer, parser, AST, semántica, elaboración → IR
│   ├── logicf4-sim/        # motor de simulación
│   └── logicf4-cli/        # logicf4 check | sim | table archivo.lx
└── apps/
    └── desktop/            # eframe: editor, lienzo, paneles
```

Dependencias del desktop: `eframe`, `egui_dock` (paneles tipo IDE), `rfd` (diálogos de archivo), `serde`.

El editor de código usa el **lexer real** del compilador para el resaltado de sintaxis, y los diagnósticos salen del análisis semántico.

---

## Plan de desarrollo

1. **Núcleo:** lexer, parser, semántica, IR y simulador, probados desde la CLI con tests.
2. **App mínima:** editor, circuito dibujado automáticamente, interruptores, constantes, compuertas, `out` y LED.
3. **Secuenciales:** pulsador, reloj y realimentación (flip-flops con compuertas).
4. **Visualización:** display 7 segmentos y sonda.
5. **Buses:** splitters, tri-state, ROM, RAM y matriz de LEDs.
6. **Publicación:** GitHub Actions (build, tests, binarios por plataforma).

Más adelante: tablas de verdad, mapas de Karnaugh y exportación a Verilog.

---

## Licencia

Por definir.
