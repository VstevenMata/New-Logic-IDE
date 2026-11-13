//! Comandos de terminal: `logicsm update | delete | help`. Usan el instalador, que va dentro del
//! ejecutable: así la instalación es un solo archivo, como en GGEZ.

use std::process::{Command, ExitCode};

const URL: &str = "https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main";

pub fn ejecutar(orden: &str) -> ExitCode {
    consola();
    match orden {
        "update" => instalador(false),
        "delete" | "uninstall" => instalador(true),
        "help" | "-h" | "--help" => {
            ayuda();
            ExitCode::SUCCESS
        }
        "--version" | "-V" => {
            println!("LogicSM {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        otro => {
            eprintln!("logicsm: comando desconocido: {otro}\n");
            ayuda();
            ExitCode::FAILURE
        }
    }
}

fn ayuda() {
    println!(
        "Uso: logicsm [comando]

  open      abre LogicSM (es lo que hace logicsm sin comando)
  update    actualiza a la última versión
  delete    desinstala LogicSM; tus proyectos no se borran
  help      muestra esta ayuda"
    );
}

fn instalador(desinstalar: bool) -> ExitCode {
    if std::env::current_exe().is_ok_and(|exe| exe.starts_with("/nix/store")) {
        eprintln!("LogicSM se instaló con nix: actualízalo o quítalo con nix.");
        return ExitCode::FAILURE;
    }
    match correr_instalador(desinstalar) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: no se pudo abrir el instalador: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Actualizar baja el instalador más nuevo; desinstalar usa el que va adentro, así funciona sin internet
#[cfg(unix)]
fn correr_instalador(desinstalar: bool) -> std::io::Result<bool> {
    use std::io::Write;
    use std::process::Stdio;

    if !desinstalar {
        let orden = format!("curl -fsSL {URL}/instalar.sh | bash -s -- -y");
        return Ok(Command::new("bash")
            .arg("-c")
            .arg(orden)
            .status()?
            .success());
    }
    let mut bash = Command::new("bash")
        .args(["-s", "--", "--desinstalar"])
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut entrada) = bash.stdin.take() {
        entrada.write_all(include_str!("../../../instalar.sh").as_bytes())?;
    }
    Ok(bash.wait()?.success())
}

/// En Windows un .exe en uso no se puede reemplazar ni borrar: el instalador se abre en una ventana
/// propia, espera a que este proceso termine y recién ahí toca el .exe
#[cfg(windows)]
fn correr_instalador(desinstalar: bool) -> std::io::Result<bool> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x10;

    let pid = std::process::id();
    let orden = if desinstalar {
        let ruta = std::env::temp_dir().join("logicsm-instalador.ps1");
        // Con BOM, para que Windows PowerShell 5 lea bien las tildes
        std::fs::write(
            &ruta,
            format!("\u{feff}{}", include_str!("../../../instalar.ps1")),
        )?;
        let ruta = ruta.display().to_string().replace('\'', "''");
        format!("& '{ruta}' -Desinstalar -Esperar {pid}")
    } else {
        format!(
            "try {{ & ([scriptblock]::Create((irm {URL}/instalar.ps1))) -Si -Esperar {pid} }} \
             catch {{ Write-Host $_ -ForegroundColor Red; Read-Host 'Presiona Enter para cerrar' }}"
        )
    };
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &orden,
        ])
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()?;
    println!("El instalador se abrió en una ventana nueva.");
    Ok(true)
}

/// La app de Windows no tiene consola propia (así no abre una ventana negra al hacerle doble clic):
/// para escribir en la terminal desde donde se llamó, se engancha a ella
pub fn consola() {
    #[cfg(windows)]
    {
        unsafe extern "system" {
            safe fn AttachConsole(proceso: u32) -> i32;
        }
        const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
