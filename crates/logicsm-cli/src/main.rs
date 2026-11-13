//! Herramienta de desarrollo: prueba las compuertas predefinidas, por ejemplo `logicsm-cli and 3`.

use logicsm_sim::Compuerta;
use std::io::IsTerminal;
use std::process::ExitCode;

const MAX_ENTRADAS: usize = 10;

fn main() -> ExitCode {
    match ejecutar(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}\n\nusa `logicsm-cli --help` para ver las compuertas");
            ExitCode::FAILURE
        }
    }
}

fn ejecutar(args: Vec<String>) -> Result<(), String> {
    let Some(nombre) = args.first() else {
        ayuda();
        return Ok(());
    };
    if matches!(nombre.as_str(), "-h" | "--help" | "help") {
        ayuda();
        return Ok(());
    }
    let compuerta = Compuerta::desde_nombre(&nombre.to_lowercase())
        .ok_or_else(|| format!("«{nombre}» no es una compuerta predefinida"))?;
    let n = match args.get(1) {
        None if compuerta.entradas_validas(1) => 1,
        None => 2,
        Some(s) => s
            .parse()
            .map_err(|_| format!("«{s}» no es una cantidad de entradas"))?,
    };
    if !compuerta.entradas_validas(n) {
        return Err(match n {
            1 => format!("{nombre} necesita al menos 2 entradas"),
            _ => format!("{nombre} tiene siempre 1 entrada"),
        });
    }
    if n > MAX_ENTRADAS {
        return Err(format!(
            "máximo {MAX_ENTRADAS} entradas ({n} serían {} filas)",
            1u64 << n.min(63)
        ));
    }
    tabla(compuerta, n);
    Ok(())
}

fn tabla(compuerta: Compuerta, n: usize) {
    let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let bit = |b: bool| match (color, b) {
        (true, true) => "\x1b[1;32m1\x1b[0m",
        (true, false) => "\x1b[2m0\x1b[0m",
        (false, true) => "1",
        (false, false) => "0",
    };
    let letras: Vec<char> = ('A'..).take(n).collect();
    let entradas = if n == 1 { "entrada" } else { "entradas" };

    println!(
        "\n  {} de {n} {entradas}\n",
        compuerta.nombre().to_uppercase()
    );
    let cabecera: String = letras.iter().map(|l| format!(" {l} ")).collect();
    println!("  {cabecera}│ Y");
    println!("  {}┼───", "─".repeat(n * 3));
    for fila in 0..1u32 << n {
        let valores: Vec<bool> = (0..n).map(|i| fila >> (n - 1 - i) & 1 == 1).collect();
        let celdas: String = valores.iter().map(|&v| format!(" {} ", bit(v))).collect();
        println!("  {celdas}│ {}", bit(compuerta.evaluar(&valores)));
    }
    println!();
}

fn ayuda() {
    println!(
        "logicsm-cli {} · prueba las compuertas predefinidas de LogicSM

uso: logicsm-cli <compuerta> [entradas]

compuertas:
  and, or, xor, nand, nor, xnor   de 2 a {MAX_ENTRADAS} entradas (por defecto 2)
  not, buffer                     1 entrada

ejemplos:
  logicsm-cli and 3
  logicsm-cli xor 4
  logicsm-cli not",
        env!("CARGO_PKG_VERSION")
    );
}
