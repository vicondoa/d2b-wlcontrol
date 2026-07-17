{
  description = "d2b-wlcontrol — Waybar indicator and control center for d2b workloads";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    d2b-client-toolkit = {
      url = "github:vicondoa/d2b-toolkit/800c2878533f600d8f085b3d2aafcddb970232b2";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      d2b-client-toolkit,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
      pkgsFor = system: import nixpkgs { inherit system; };
      version = "2.0.0";

      runtimeBins =
        pkgs: with pkgs; [
          quickshell
          xdg-utils
        ];
      runtimeFonts = pkgs: with pkgs; [ material-symbols ];
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          clientToolkitSource = d2b-client-toolkit.packages.${system}.default;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "d2b-wlcontrol";
            inherit version;
            src = pkgs.lib.cleanSource ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "d2b-client-2.0.0" = "sha256-H0IEHleS2dLCBxnosGF8ztkA/qTnsmyG6Y1QQIhZ4lU=";
              };
            };

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
                --replace-fail "../d2b-client-toolkit/crates/d2b-client-toolkit-waybar" \
                  "${clientToolkitSource}/share/d2b-client-toolkit/distribution/crates/d2b-client-toolkit-waybar" \
                --replace-fail "../d2b-client-toolkit/crates/d2b-client-toolkit-colors" \
                  "${clientToolkitSource}/share/d2b-client-toolkit/distribution/crates/d2b-client-toolkit-colors" \
                --replace-fail "../d2b-client-toolkit/crates/d2b-client-toolkit" \
                  "${clientToolkitSource}/share/d2b-client-toolkit/distribution/crates/d2b-client-toolkit"
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
          hmOptionStubs =
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
            };
          hmModule = import ./nix/home-manager.nix { inherit self; };
          hmEval = pkgs.lib.evalModules {
            specialArgs = { inherit pkgs; };
            modules = [
              hmOptionStubs
              hmModule
              {
                programs.d2b-wlcontrol = {
                  enable = true;
                  colorArtifactPath = "/etc/d2b/custom-ui-colors.json";
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
          hmWaybarDisabledEval = pkgs.lib.evalModules {
            specialArgs = { inherit pkgs; };
            modules = [
              hmOptionStubs
              hmModule
              {
                programs.d2b-wlcontrol = {
                  enable = true;
                  waybar.enable = false;
                };
              }
            ];
          };
          disabledWaybarFilesAbsent =
            !(hmWaybarDisabledEval.config.xdg.configFile ? "d2b-wlcontrol/waybar-module.json")
            && !(hmWaybarDisabledEval.config.xdg.configFile ? "d2b-wlcontrol/style.css");
          renderedModule =
            builtins.toJSON
              hmEval.config.programs.waybar.settings.mainBar."custom/d2b-wlcontrol";
          renderedStyle = pkgs.writeText "d2b-wlcontrol-waybar-style.css" hmEval.config.programs.waybar.style;
        in
        {
          package = self.packages.${system}.default;
          release-metadata = pkgs.runCommand "d2b-wlcontrol-release-metadata-${version}" { } ''
            grep -Fq 'version = "2.0.0"' ${./Cargo.toml}
            grep -Fq '## [Unreleased]' ${./CHANGELOG.md}
            grep -Fq '800c2878533f600d8f085b3d2aafcddb970232b2' ${./Cargo.toml}
            grep -Fq '800c2878533f600d8f085b3d2aafcddb970232b2' ${./flake.lock}
            grep -Fq '4018d9c9652bd826c2e6a9abccdcdcafb832d944' ${./Cargo.toml}
            grep -Fq 'c2c99bdd77ba66948fce81161dcc3efde608eefefb96f28fa934c9f58d96d838' ${./Cargo.toml}
            grep -Fq '2aaef697cc53abc8757a3593352cd5bd1d3f0d3f2031c6a2967f92afa5e74d97' ${./Cargo.toml}
            test ! -e ${./.}/crates/wlcontrol-d2b/src/transport.rs
            test ! -e ${./.}/crates/wlcontrol-d2b/src/wire.rs
            test ! -e ${./.}/crates/wlcontrol-d2b/tests/public_socket.rs
            test ! -e ${./.}/tests/fixtures/public-workload-v3-v1
            touch $out
          '';
          home-manager-module = pkgs.runCommand "d2b-wlcontrol-home-manager-module-${version}" { } ''
            config=${hmEval.config.xdg.configFile."d2b-wlcontrol/config.toml".source}
            grep -q 'public_socket = "/run/d2b/public.sock"' "$config"
            grep -q 'color_artifact_path = "/etc/d2b/custom-ui-colors.json"' "$config"
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
            test '${if disabledWaybarFilesAbsent then "true" else "false"}' = true
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
