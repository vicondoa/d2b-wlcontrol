{
  description = "nixling-wlcontrol — clean Waybar indicator and control center for nixling VMs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
      pkgsFor = system: import nixpkgs { inherit system; };

      # GTK runtime/build closure used by the control center (Wave 2). Declared
      # now so the package and dev shell are stable across waves.
      gtkInputs =
        pkgs: with pkgs; [
          glib
          gtk4
          libadwaita
        ];
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "nixling-wlcontrol";
            version = "0.1.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              pkg-config
              wrapGAppsHook4
            ];
            buildInputs = gtkInputs pkgs;

            # The binary is provided by the wlcontrol-cli crate.
            cargoBuildFlags = [
              "-p"
              "wlcontrol-cli"
            ];

            meta = with pkgs.lib; {
              description = "Waybar indicator and control center for nixling microVMs";
              license = licenses.asl20;
              mainProgram = "nixling-wlcontrol";
              platforms = systems;
            };
          };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/nixling-wlcontrol";
        };
      });

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              pkg-config
              rustc
              cargo
              clippy
              rustfmt
              wrapGAppsHook4
            ];
            buildInputs = gtkInputs pkgs;
          };
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt-rfc-style);
    };
}
