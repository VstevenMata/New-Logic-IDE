<p align="center">
  <img src="assets/logo.png" alt="Logo de LogicSM" width="120">
</p>

<h1 align="center">LogicSM</h1>

<p align="center">
  Diseña, visualiza y simula circuitos digitales escribiendo código.
</p>

<p align="center">
  <a href="https://github.com/VstevenMata/logicsm/releases/latest"><img src="https://img.shields.io/github/v/release/VstevenMata/logicsm?label=versión" alt="Última versión"></a>
  <a href="#licencia"><img src="https://img.shields.io/badge/licencia-MIT%20%2F%20Apache--2.0-blue" alt="Licencia"></a>
</p>

---

LogicSM es un entorno de escritorio para aprender electrónica digital. En vez de dibujar cables, el circuito se escribe en un lenguaje sencillo, y la app lo dibuja y lo simula automáticamente.

```cpp
circuit HalfAdder {
    in A, B;
    out Suma, Acarreo;

    Suma    = xor(A, B);
    Acarreo = and(A, B);
}
```

## Qué permite

- **Escribir circuitos en código** y verlos dibujados al instante.
- **Simular en tiempo real** con interruptores, pulsadores, reloj y paso a paso.
- **Crear componentes propios** y reutilizarlos: de compuertas a sumadores, ALU y una CPU educativa.
- **Aprender con ayudas exactas:** errores con sugerencia, tabla de verdad y verificación de resultados.
- **Exportar** a Verilog y a LogicCircuit (próximamente).

## Instalación

**En el navegador**, sin instalar nada: <https://vstevenmata.github.io/logicsm/>. El proyecto se guarda en el navegador; Guardar lo descarga como `.SA`.

**Linux y macOS**

```bash
curl -fsSL https://raw.githubusercontent.com/VstevenMata/logicsm/main/instalar.sh | bash
```

**Windows** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/VstevenMata/logicsm/main/instalar.ps1 | iex
```

**NixOS**

```bash
nix run github:VstevenMata/logicsm
```

Después se abre con `logicsm` y se actualiza con `logicsm update`.

## Estado

En desarrollo. El lenguaje, el simulador y las herramientas educativas ya funcionan.

| | Etapa |
|---|---|
| ✅ | Interfaz con tema claro y oscuro, de escritorio y en el navegador |
| ✅ | Instaladores, actualización automática y publicación de versiones para Linux, macOS y Windows |
| ✅ | Revisión automática de cada cambio (CI) |
| ✅ | Lenguaje: circuitos propios, buses, `for`, dispositivos y memorias |
| ✅ | Simulador por eventos (0, 1, X, Z) y dibujo automático del circuito |
| ✅ | Reloj, pulsador, display de 7 segmentos, matriz de LEDs y sonda |
| ✅ | Tabla de verdad, verificación, circuito desde la tabla y equivalencia |
| ⏳ | Exportación a Verilog y LogicCircuit |

## Documentación

- [Diseño del lenguaje y la arquitectura](docs/DISENO.md)
- [Guía para desarrolladores](COMANDOS.md)

## Licencia

Doble licencia, a elección: [MIT](LICENSE-MIT) o [Apache-2.0](LICENSE-APACHE).
