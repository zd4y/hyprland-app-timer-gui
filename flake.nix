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
          desktop-file-utils
          ninja
          pkg-config
          rustc
          cargo
          rustPlatform.cargoSetupHook
        ];
        buildInputs = with pkgs; [
          gtk4
          libadwaita
          openssl
        ];
      in
      {
        packages = rec {
          hyprland-app-timer-gui = pkgs.stdenv.mkDerivation {
            inherit nativeBuildInputs buildInputs;
            pname = "hyprland-app-timer-gui";
            src = pkgs.lib.cleanSource ./.;
            cargoDeps = pkgs.rustPlatform.importCargoLock {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "hyprland-app-timer-0.1.0" = "sha256-7uk0MnZIPBRhdbem7PW2s9oAxCi1GrUg/yH2JMwxDoE=";
              };
            };
            mesonFlags = [ "--buildtype=release" ];
          };
          default = hyprland-app-timer-gui;
        };
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs;
          buildInputs = buildInputs ++ (with pkgs; [
            clippy
            rustfmt
          ]);
        };
      }
    );
}
