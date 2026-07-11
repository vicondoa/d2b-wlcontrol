{
  description = "d2b-wlcontrol — Waybar indicator and control center for d2b workloads";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    d2b-toolkit = {
      url = "github:vicondoa/d2b-toolkit/v0.2.0";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, d2b-toolkit }:
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
          toolkitSource = d2b-toolkit.packages.${system}.default;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "d2b-wlcontrol";
            version = "0.2.0";
            src = pkgs.lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [ makeWrapper ];

            # The binary is provided by the wlcontrol-cli crate.
            cargoBuildFlags = [
              "-p"
              "wlcontrol-cli"
            ];
            cargoTestFlags = [ "--workspace" ];

            postInstall = ''
              wrapProgram "$out/bin/d2b-wlcontrol" \
                --prefix PATH : ${pkgs.lib.makeBinPath (runtimeBins pkgs)} \
                --prefix XDG_DATA_DIRS : ${pkgs.lib.makeSearchPath "share" (runtimeFonts pkgs)}
            '';

            postPatch = ''
              substituteInPlace Cargo.toml \
                --replace-fail "../d2b-toolkit/crates/d2b-client" \
                  "${toolkitSource}/share/d2b-toolkit/crates/d2b-client" \
                --replace-fail "../d2b-toolkit/crates/d2b-toolkit-core" \
                  "${toolkitSource}/share/d2b-toolkit/crates/d2b-toolkit-core" \
                --replace-fail "../d2b-toolkit/crates/d2b-wayland-core" \
                  "${toolkitSource}/share/d2b-toolkit/crates/d2b-wayland-core" \
                --replace-fail "../d2b-toolkit/crates/d2b-wayland-waybar" \
                  "${toolkitSource}/share/d2b-toolkit/crates/d2b-wayland-waybar"
            '';

            meta = with pkgs.lib; {
              description = "Waybar indicator and control center for d2b workloads";
              license = licenses.asl20;
              mainProgram = "d2b-wlcontrol";
              platforms = systems;
            };
          };
        }
      );

      checks = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          hmEval = pkgs.lib.evalModules {
            specialArgs = { inherit pkgs; };
            modules = [
              (
                { lib, ... }:
                {
                  options.assertions = lib.mkOption {
                    type = lib.types.listOf lib.types.anything;
                    default = [ ];
                  };
                  options.home.packages = lib.mkOption {
                    type = lib.types.listOf lib.types.package;
                    default = [ ];
                  };
                  options.xdg.configFile = lib.mkOption {
                    type = lib.types.attrsOf lib.types.anything;
                    default = { };
                  };
                  options.programs.waybar.enable = lib.mkOption {
                    type = lib.types.bool;
                    default = false;
                  };
                  options.programs.waybar.style = lib.mkOption {
                    type = lib.types.lines;
                    default = "";
                  };
                  options.programs.waybar.settings = lib.mkOption {
                    type = lib.types.attrsOf (
                      lib.types.submodule {
                        freeformType = lib.types.attrsOf lib.types.anything;
                        options."modules-left" = lib.mkOption {
                          type = lib.types.listOf lib.types.str;
                          default = [ ];
                        };
                        options."modules-center" = lib.mkOption {
                          type = lib.types.listOf lib.types.str;
                          default = [ ];
                        };
                        options."modules-right" = lib.mkOption {
                          type = lib.types.listOf lib.types.str;
                          default = [ ];
                        };
                      }
                    );
                    default = { };
                  };
                }
              )
              (import ./nix/home-manager.nix { inherit self; })
              {
                programs.d2b-wlcontrol = {
                  enable = true;
                  launcherOverrides = [
                    {
                      target = "tools.host.d2b";
                      itemId = "firefox";
                      name = "Web";
                      icon = "language";
                    }
                  ];
                  waybar = {
                    enable = true;
                    modulesList = "modules-left";
                    icon = "◇";
                    label = "d2b";
                    clickAction = "d2b-wlcontrol open";
                    module."on-click-right" = "d2b-wlcontrol action refresh";
                  };
                };
                programs.waybar.enable = true;
                programs.waybar.settings.mainBar.modules-left = [ "clock" ];
              }
            ];
          };
          renderedModule =
            builtins.toJSON hmEval.config.programs.waybar.settings.mainBar."custom/d2b-wlcontrol";
          renderedStyle =
            pkgs.writeText "d2b-wlcontrol-waybar-style.css" hmEval.config.programs.waybar.style;
        in
        {
          package = self.packages.${system}.default;
          home-manager-module = pkgs.runCommand "d2b-wlcontrol-home-manager-module" { } ''
            config=${hmEval.config.xdg.configFile."d2b-wlcontrol/config.toml".source}
            grep -q 'public_socket = "/run/d2b/public.sock"' "$config"
            grep -q 'icon = "◇"' "$config"
            grep -q 'label = "d2b"' "$config"
            grep -q 'target = "tools.host.d2b"' "$config"
            grep -q 'item_id = "firefox"' "$config"
            printf '%s' '${renderedModule}' | ${pkgs.jq}/bin/jq -e '
              .exec | contains("d2b-wlcontrol waybar")
            ' >/dev/null
            printf '%s' '${renderedModule}' | ${pkgs.jq}/bin/jq -e '
              has("interval") | not
            ' >/dev/null
            printf '%s' '${builtins.toJSON hmEval.config.programs.waybar.settings}' \
              | grep -q '"modules-left":\["clock","custom/d2b-wlcontrol"\]'
            grep -q '#custom-d2b-wlcontrol.unsafe-local' ${renderedStyle}
            touch $out
          '';
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/d2b-wlcontrol";
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

      homeManagerModules.default = import ./nix/home-manager.nix { inherit self; };
    };
}
