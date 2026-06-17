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

      runtimeBins = pkgs: with pkgs; [ quickshell xdg-utils ];
      runtimeFonts = pkgs: with pkgs; [ material-symbols ];
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

            nativeBuildInputs = with pkgs; [ makeWrapper ];

            # The binary is provided by the wlcontrol-cli crate.
            cargoBuildFlags = [
              "-p"
              "wlcontrol-cli"
            ];

            postInstall = ''
              wrapProgram "$out/bin/nixling-wlcontrol" \
                --prefix PATH : ${pkgs.lib.makeBinPath (runtimeBins pkgs)} \
                --prefix XDG_DATA_DIRS : ${pkgs.lib.makeSearchPath "share" (runtimeFonts pkgs)}
            '';

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
              rustc
              cargo
              clippy
              rustfmt
              quickshell
              xdg-utils
              material-symbols
            ];
          };
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt-rfc-style);
    };
}
