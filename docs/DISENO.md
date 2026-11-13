# Diseño de LogicSM

Especificación del lenguaje, del simulador y de la arquitectura. Para instalar la app, ver el [README](../README.md).

Lenguaje de programación y entorno de desarrollo de escritorio para **diseñar, visualizar y simular circuitos digitales**.

Inspirado en LogicCircuit, pero con una diferencia clave: los circuitos y componentes se **escriben en código**, y la aplicación dibuja y simula el circuito automáticamente.


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

### Compuertas: se usan directo

Cada llamada es una compuerta nueva e independiente. El número de entradas sale de los argumentos.

```cpp
circuit Ejemplo {
    in A, B, C, D;
    out X, Y;

    X = and(A, B, C);   // AND de 3 entradas
    Y = and(C, D);      // AND de 2 (otra compuerta distinta)
}
```

### Circuitos propios: se usan igual que las compuertas

```cpp
circuit FlipFlop {
    in S, R;
    out Q, Qn;

    Q  = nor(R, Qn);    // realimentación: Q depende de Qn y viceversa
    Qn = nor(S, Q);
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

### Repetir sin copiar: arreglos de instancias y `for`

```cpp
circuit Adder8 {
    in A[8], B[8], Cin;
    out S[8], Cout;

    FullAdder fa[8];                 // 8 instancias independientes

    for (i = 0; i < 8; i++) {
        fa[i].A = A[i];
        fa[i].B = B[i];
        S[i]    = fa[i].Sum;
    }
    fa[0].Cin = Cin;
    for (i = 1; i < 8; i++) {
        fa[i].Cin = fa[i-1].Cout;    // acarreo al siguiente bit
    }
    Cout = fa[7].Cout;
}
```

El `for` solo **construye** el circuito al compilar (no se ejecuta durante la simulación). `i` solo se puede usar dentro de índices.

### Reglas del proyecto

- **Extensión `.SA`** para los archivos de código (el compilador también acepta `.sa`).
- **Sin `import`:** cualquier circuito de un archivo `.SA` en la carpeta raíz del proyecto se puede usar por su nombre.
- **Por referencia:** si se modifica `FlipFlop.SA`, todos los circuitos que lo usan reflejan el cambio al recompilar.
- **Se compila el circuito seleccionado**, no solo `main`. Al abrir `FlipFlop.SA` y presionar ▶ se simula ese circuito, y sus entradas aparecen como interruptores. Solo se compila ese circuito y sus dependencias, así que un error en otro archivo no bloquea la prueba.
- Un archivo puede tener varios circuitos; el nombre del archivo no importa, cuenta el nombre del `circuit`.
- Los moldes de memoria (`rom`, `ram`) declarados fuera de un `circuit` se ven en **todo el proyecto**, igual que los circuitos.
- Distingue mayúsculas: `Q` y `q` son distintos.
- `main` es opcional: es solo el circuito que se abre por defecto.
- **Constantes:** `0` y `1` se pueden pasar donde va una señal, por ejemplo `FlipFlop(1, 0)` o `and(A, B, 1)`.

```
mi-proyecto/
├── main.SA
├── FlipFlop.SA
└── Contador.SA
```

---

## Componentes

Hay dos formas de escribirlos:

1. **Compuertas y memorias:** las compuertas se llaman directo, `and(A, B)`. Las memorias se configuran con un molde `R = rom.size(...);` y luego `R(...)`.
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
| AND, OR, XOR, NAND, NOR, XNOR  | `and(A, B, C)` (2 o más entradas)               |
| NOT, buffer                    | `not(X)`, `buffer(X)` (siempre 1 entrada)       |
| Buffer tri-state               | `tristate(dato, habilitar)`                     |
| ROM                            | `R = rom.size(4, 8, "datos.hex");` → `R(dir)` (bits de dirección, bits de dato) |
| RAM                            | `M = ram.size(4, 8);` → `M(dir, dato, escribir, clk)` |

**Buses y splitters:** `in A[8];` declara un bus. `A[0]`, `A[0..3]` y `{a, b, c}` separan y unen bits. En el dibujo se ven como splitters.

### Ejemplo

```cpp
circuit Demo {
    clock CLK(1);
    button Reset;

    c = Contador(CLK, Reset);    // circuito propio (Contador.SA)
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
item        = circuit | assign ;              (* assign afuera = molde de memoria global *)

circuit     = ( "circuit" IDENT | "main" ) "{" { stmt } "}" ;

(* ---------- Sentencias ---------- *)
stmt        = port_decl
            | device_decl
            | inst_decl
            | for_stmt
            | assign ;

port_decl   = ( "in" | "out" ) port { "," port } ";" ;
port        = IDENT [ "[" INT "]" ] ;         (* A  o  A[8] = bus de 8 bits *)

device_decl = DEVICE IDENT [ "(" [ args ] ")" ] ";" ;   (* button B;  led L(Q); *)
DEVICE      = "button" | "clock" | "led" | "display7" | "matrix" | "probe" ;

inst_decl   = IDENT IDENT [ "[" iexpr "]" ] ";" ;   (* FullAdder fa[8]; *)

for_stmt    = "for" "(" IDENT "=" iexpr ";" IDENT "<" iexpr ";" IDENT "++" ")"
              "{" { stmt } "}" ;

assign      = target "=" expr ";" ;
target      = IDENT { index | "." IDENT } ;     (* Q,  S[3],  fa[i].Cin *)

(* ---------- Expresiones ---------- *)
expr        = postfix | concat ;
concat      = "{" expr { "," expr } "}" ;

postfix     = primary { "." IDENT
                      | "(" [ args ] ")"
                      | index } ;

primary     = IDENT | COMPONENT | INT | STRING | "(" expr ")" ;
COMPONENT   = "and" | "or" | "xor" | "nand" | "nor" | "xnor"
            | "not" | "buffer" | "tristate" | "rom" | "ram" ;
index       = "[" iexpr [ ".." iexpr ] "]" ;  (* rango inclusivo *)
iexpr       = iterm { ( "+" | "-" ) iterm } ;   (* índices: 3,  i,  i-1 *)
iterm       = INT | IDENT ;
args        = expr { "," expr } ;

(* ---------- Léxico ---------- *)
IDENT       = letter { letter | digit | "_" } ;
INT         = digit { digit } | "0x" hex { hex } | "0b" bin { bin } ;
STRING      = '"' { char - '"' } '"' ;
letter      = "A".."Z" | "a".."z" | "_" ;
comment     = "//" { char } newline | "/*" { char } "*/" ;
```

**Palabras reservadas (22):**

- Estructura: `circuit`, `main`, `for`, `in`, `out`
- Compuertas: `and`, `or`, `xor`, `nand`, `nor`, `xnor`, `not`, `buffer`, `tristate`
- Memorias: `rom`, `ram`
- Dispositivos: `button`, `clock`, `led`, `display7`, `matrix`, `probe`

Los componentes son reservados para que no se puedan tapar (`and = ...` es error de sintaxis). Agregar un componente nuevo implica sumarlo al lexer y a la gramática. `size` no es reservada: es un nombre que va después de `.`.

### Reglas semánticas

1. **El orden no importa** dentro de un circuito: una señal se puede usar antes de definirse (necesario para la realimentación).
2. **Asignar a un nombre nuevo lo crea:** molde (`M = ram.size(4, 8);`), instancia (`ff = FlipFlop(S, R);`) o señal (`x = nor(A, B);`).
3. **Un solo conductor por señal:** asignar dos veces la misma señal es error (excepto salidas tri-state en un bus).
4. **Cantidades y anchos deben coincidir:** `not(A, B)` es error (espera 1 entrada), igual que `and(A)` (pide 2 o más).
5. **Resolución de nombres:** circuito actual → moldes y circuitos del proyecto (cualquier `.SA` de la carpeta raíz). Los componentes son reservados, así que nunca chocan.
6. Todo `out` debe estar asignado. Entradas sin usar generan advertencia.
7. Una entrada de subcircuito sin conectar vale **X** y genera advertencia.
8. El `for` se desenrolla al compilar; su variable solo existe en índices.

---

## Simulación y entorno

- **Retardo:** cada compuerta tarda 1 unidad de tiempo. Así se ven las carreras y los *glitches*, y los flip-flops funcionan.
- **Estado inicial:** todo arranca en **X** (rojo), como en la realidad. El botón **Reset** pone todo en 0.
- **Paso a paso:** cada click avanza una unidad de tiempo y resalta las compuertas que cambiaron.
- **Dibujo:** se genera automáticamente; las compuertas se pueden arrastrar para acomodarlas y las posiciones se guardan en un `.layout.json` junto al `.SA`. Los circuitos se crean en código, no dibujando.
- **Idioma:** interfaz y mensajes de error en español; palabras clave en inglés.
- **Ejemplos incluidos:** compuertas básicas, half adder, full adder, flip-flop SR, contador y display 7 segmentos.

---

## Herramientas educativas

Funcionan sin IA, con algoritmos exactos dentro del compilador y el simulador. No necesitan internet y nunca dan una respuesta incorrecta.

| Herramienta | Qué hace | Cómo |
|---|---|---|
| **Errores con sugerencia** | *"`FlipFlp` no existe, ¿quisiste decir `FlipFlop`?"*, *"AND3 espera 3 entradas, recibió 2"* | Distancia de edición entre nombres; verificación de cantidades y anchos |
| **Tabla de verdad** | Genera la tabla completa del circuito seleccionado | El simulador prueba todas las combinaciones de entradas |
| **Verificación** | *"Con A=1, B=0 tu salida da 0 y debería dar 1"* | El estudiante llena la columna "esperado" en el panel de la tabla de verdad y el simulador marca las filas que fallan |
| **Solución** | A partir de una tabla de verdad, genera el circuito mínimo ya escrito en Logic_F-4:13 | Quine-McCluskey / mapas de Karnaugh |
| **Equivalencia** | Dice si dos circuitos hacen lo mismo y, si no, muestra la combinación donde difieren | Comparación de tablas de verdad |

---

## Exportación e importación

### Verilog

Verilog es el lenguaje de descripción de hardware (HDL) estándar de la industria para diseñar chips y programar FPGAs. Exportar a Verilog permite:

- cargar un circuito en una **FPGA** y verlo funcionar como hardware real;
- abrirlo en herramientas profesionales;
- servir de puente hacia cursos avanzados de diseño digital.

Como el IR es una lista de componentes conectados, la traducción es casi directa:

```verilog
module HalfAdder(input A, input B, output Sum, output Carry);
    assign Sum   = A ^ B;
    assign Carry = A & B;
endmodule
```

### LogicCircuit (`.CircuitProject`)

LogicCircuit guarda los proyectos en XML (`.CircuitProject`) con elementos `LogicalCircuit` (cada circuito), `Pin` (entradas y salidas), `CircuitSymbol` (componente colocado en la cuadrícula, con `X`/`Y`) y `Wire` (tramo de cable `X1,Y1 → X2,Y2`). Los cables se conectan **por coordenadas**: un cable está conectado a un pin si su punta cae exactamente sobre él.

- **Exportar** (Logic_F-4:13 → LogicCircuit): la conexión lógica sale del IR y las posiciones del layout por capas, ajustado a la cuadrícula de LogicCircuit. Cada circuito propio se exporta como un `LogicalCircuit`, así que se conserva la jerarquía. Pendiente: confirmar en su esquema (`Schema/CircuitProject.xml`) cómo identifica las compuertas predefinidas y sus atributos.
- **Importar** (LogicCircuit → Logic_F-4:13): leer el XML, reconstruir las conexiones a partir de la geometría y generar código `.SA`. Así un estudiante puede ver sus circuitos de LogicCircuit escritos como código.

LogicCircuit es GPL-2.0: se implementa un lector/escritor propio del formato, sin copiar su código.

---

## Arquitectura

### Tecnologías

**Rust** en todo el proyecto: compilador, simulador e interfaz. Se obtiene un solo ejecutable por plataforma (Windows, Linux, macOS), sin webview ni React. El simulador y el lienzo comparten memoria directamente, sin IPC. Como extra, eframe compila a WASM, lo que permite una versión web.

| Tecnología | Para qué |
|---|---|
| `eframe` / `egui` | Ventana, editor, lienzo del circuito y paneles |
| `egui_dock` | Paneles acoplables tipo IDE |
| `rfd` | Diálogos nativos de abrir y guardar |
| `serde` + `serde_json` | Configuración y posiciones del dibujo |
| `notify` | Vigilar la carpeta del proyecto y recompilar al cambiar un `.SA` |
| `quick-xml` | Exportar e importar proyectos de LogicCircuit |
| `ariadne` | Errores con el código subrayado en `logicsm-cli` |
| `egui_plot` | Pestaña de ondas |
| `cargo test` + `insta` | Tests del compilador y del simulador; `insta` guarda la salida esperada del parser y de los errores |
| GitHub Actions | CI en cada PR (`fmt`, `clippy`, `test`) y release automático al fusionar a `main` un cambio de versión |
| Nix flake | Entorno de desarrollo reproducible (opcional) |

**Se descartó:** LSP (el lenguaje ya tiene su IDE), base de datos (los proyectos son archivos `.SA`), `tokio` (no hay red, salvo `logicsm update`, que lo hace el instalador) y `syntect` (el resaltado usa el lexer propio).

**Escrito por nosotros, sin librerías:** lexer, parser, análisis semántico, IR, simulador, layout por capas, tabla de verdad y Quine-McCluskey.

### Pipeline del compilador

```
código .SA → Lexer → Parser → AST → Análisis semántico → Elaboración → Circuit IR
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
│   ├── logicsm-core/       # lexer, parser, AST, semántica, elaboración → IR
│   ├── logicsm-sim/        # motor de simulación
│   └── logicsm-cli/        # herramienta de desarrollo: logicsm-cli and 3 → tabla de verdad
└── apps/
    └── desktop/            # LogicSM (binario logicsm): editor, lienzo, paneles
```

`logicsm-cli` solo prueba las compuertas predefinidas (`and`, `nor`, `xor`… con n entradas) y no tiene lógica propia: llama a `logicsm-core` y `logicsm-sim`. No se incluye en el release.

Dependencias del desktop: `eframe`, `egui_dock` (paneles tipo IDE), `rfd` (diálogos de archivo), `serde`.

El editor de código usa el **lexer real** del compilador para el resaltado de sintaxis, y los diagnósticos salen del análisis semántico.

---

## Plan de desarrollo

**MVP terminado cuando:** se escribe el FullAdder en el editor, se ve dibujado, se activan sus entradas, la tabla de verdad es correcta, y los tests del parser y del simulador pasan en GitHub Actions.

1. **Núcleo:** lexer, parser, semántica, IR y simulador, probados con `cargo test` y con las compuertas predefinidas desde `logicsm-cli`.
2. **App mínima:** editor, circuito dibujado automáticamente, interruptores, constantes, compuertas, `out` y LED.
3. **Secuenciales:** pulsador, reloj y realimentación (flip-flops con compuertas).
4. **Visualización:** display 7 segmentos y sonda.
5. **Buses:** splitters, tri-state, ROM, RAM y matriz de LEDs.
6. **Herramientas educativas:** errores con sugerencia (desde la fase 1), tabla de verdad, verificación, solución desde tabla de verdad y equivalencia.
7. **Publicación:** release automático por plataforma y CI con tests en cada PR (hecho).
8. **Exportación e importación:** Verilog, exportar a LogicCircuit y luego importar desde LogicCircuit.

---

## Licencia

Doble licencia, a elección del usuario, como es habitual en Rust:

- [MIT](../LICENSE-MIT)
- [Apache-2.0](../LICENSE-APACHE)

Salvo que se indique lo contrario, cualquier contribución enviada para su inclusión en el proyecto se licencia bajo los mismos términos, sin condiciones adicionales.
