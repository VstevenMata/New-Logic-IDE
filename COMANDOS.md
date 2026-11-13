# Comandos del proyecto

Resumen de los comandos para compilar, probar y publicar LogicSM. Todos se ejecutan en la raíz del repositorio.

## Compilar y probar en tu computadora

| Qué | Comando |
|---|---|
| Abrir LogicSM (compila si hace falta) | `nix run .` |
| Probar una compuerta con la CLI | `nix run .#cli -- and 3` |
| Solo compilar (queda en `./result/bin/`) | `nix build` |
| Entrar al entorno de desarrollo (cargo, clippy…) | `nix develop` |

Dentro de `nix develop`:

| Qué | Comando |
|---|---|
| Abrir LogicSM | `cargo run -p logicsm` |
| CLI | `cargo run -p logicsm-cli -- xor 4` |
| Formatear | `cargo fmt --all` |
| Revisar advertencias | `cargo clippy --workspace --all-targets -- -D warnings` |
| Tests | `cargo test --workspace` |

> La primera compilación tarda varios minutos; después es rápida.
> El ejecutable de `nix build` solo sirve en tu máquina. Para repartirlo, usa un release (abajo).

## Antes de abrir un PR

```bash
nix develop -c cargo fmt --all
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
nix develop -c cargo test --workspace
```

## Publicar una versión nueva (release)

El release es automático. GitHub Actions compila los ejecutables de Linux, macOS y Windows, crea el tag y publica el release cuando se fusiona a `main` un PR que cambia la versión.

1. En tu rama, cambia `version` en `Cargo.toml` (sección `[workspace.package]`), por ejemplo a `0.3.0`.
2. Actualiza `Cargo.lock`, porque si no, el release falla:

   ```bash
   nix develop -c cargo update -w
   ```

3. Haz commit, abre el PR y fusiónalo a `main`.

Listo: se crea el tag `v0.3.0` y el release. Puedes ver el avance en la pestaña **Actions** del repositorio (tarda unos 5 minutos). Si el PR no cambia la versión, no se publica nada.

Los que ya lo tienen instalado se actualizan con `logicsm update`.

## Probar la descarga (como un estudiante)

| Sistema | Comando |
|---|---|
| Linux / macOS | `curl -fsSL https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.sh \| bash` |
| Windows (PowerShell) | `irm https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.ps1 \| iex` |
| NixOS | `nix run github:VstevenMata/New-Logic-IDE` |

Para instalar sin preguntas, agrega `-s -- -y` en Linux/macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.sh | bash -s -- -y
```

## Comandos después de instalar

| Qué | Comando |
|---|---|
| Abrir LogicSM | `logicsm` |
| Actualizar a la última versión | `logicsm update` |
| Desinstalar (los proyectos no se borran) | `logicsm delete` |
| Ayuda | `logicsm help` |

## Borrar un release de prueba

```bash
gh release delete v0.1.0 --yes --cleanup-tag
```
