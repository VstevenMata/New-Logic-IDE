{
  description = "LogicSM: diseña, visualiza y simula circuitos digitales";

  # nix run .                 abre LogicSM
  # nix run .#cli -- and 3    tabla de verdad de una compuerta predefinida
  # nix develop               entorno con cargo, clippy, rustfmt y rust-analyzer
  # nix develop .#web         entorno para la versión web: trunk serve
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  # Rust con el target wasm32 (el de nixpkgs solo compila para el sistema)
  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      sistemas = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      porSistema = f: nixpkgs.lib.genAttrs sistemas (s: f nixpkgs.legacyPackages.${s});

      # winit y wgpu abren estas librerías al arrancar (dlopen); en macOS no hacen falta
      librerias = pkgs: nixpkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux (with pkgs; [
        wayland libxkbcommon libGL vulkan-loader libx11 libxcursor libxrandr libxi
      ]);
    in
    {
      packages = porSistema (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "logicsm";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          # Solo lo que entra al ejecutable (los instaladores van adentro): cambiar el README no recompila
          src = nixpkgs.lib.fileset.toSource {
            root = ./.;
            fileset = nixpkgs.lib.fileset.unions [
              ./Cargo.toml ./Cargo.lock ./crates ./apps ./assets ./instalar.sh ./instalar.ps1
            ];
          };
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.makeWrapper ];
          # El ejecutable busca las librerías por su RPATH, sin depender de dónde estén en el sistema.
          # Fuera de NixOS (Fedora, Ubuntu…) no existe /run/opengl-driver y nix no ve los drivers de
          # video del sistema: se usan los de Mesa de nix (Intel, AMD y, si no hay, por software).
          postFixup = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            patchelf --add-rpath ${pkgs.lib.makeLibraryPath (librerias pkgs)} $out/bin/logicsm
            wrapProgram $out/bin/logicsm --run '
              if [ ! -e /run/opengl-driver ]; then
                export __EGL_VENDOR_LIBRARY_DIRS="''${__EGL_VENDOR_LIBRARY_DIRS:-${pkgs.mesa}/share/glvnd/egl_vendor.d}"
                export VK_DRIVER_FILES="''${VK_DRIVER_FILES:-$(echo ${pkgs.mesa}/share/vulkan/icd.d/*.json | tr " " :)}"
              fi'
          '';
          meta.mainProgram = "logicsm";
        };
      });

      apps = porSistema (pkgs: {
        cli = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/logicsm-cli";
        };
      });

      devShells = porSistema (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc clippy rustfmt rust-analyzer ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (librerias pkgs);
        };
        web =
          let p = pkgs.extend rust-overlay.overlays.default;
          in p.mkShell {
            packages = [
              (p.rust-bin.stable.latest.default.override { targets = [ "wasm32-unknown-unknown" ]; })
              p.trunk
            ];
          };
      });
    };
}
