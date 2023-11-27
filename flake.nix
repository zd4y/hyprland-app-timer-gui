{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        nativeBuildInputs = with pkgs; [
          meson
          gettext
          glib
          gtk4
          libadwaita
          desktop-file-utils
          ninja
          pkg-config
          rustc
          cargo
          rustPlatform.cargoSetupHook
        ];
      in
      {
        packages = rec {
          hyprland-app-timer-gui = pkgs.stdenv.mkDerivation {
            inherit nativeBuildInputs;
            name = "hyprland-app-timer-gui";
            src = pkgs.lib.cleanSource ./.;
            cargoDeps = pkgs.rustPlatform.importCargoLock {
              lockFile = ./Cargo.lock;
            };
            mesonFlags = [ "--buildtype=release" ];
          };
          default = hyprland-app-timer-gui;
        };
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs;
          buildInputs = with pkgs; [
            clippy
            rustfmt
          ];
        };
      }
    );
}
