{
  description = "d2b-wlcontrol — Waybar indicator and control center for d2b workloads";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    d2b-client-toolkit = {
      url = "github:vicondoa/d2b-toolkit/926de54e7320599c373524a10b65aaf13b6ff422";
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

      # Cargo git dependencies vendored by `rustPlatform.buildRustPackage`.
      # Every hash covers a single (repository, commit) pair; Cargo.lock may
      # list several crates resolved from the same pair (a workspace member
      # plus its own transitive git dependencies), so several package keys
      # below intentionally share one hash. Nix builds never substitute a
      # local `../d2b-client-toolkit` checkout: the toolkit and its frozen
      # `d2b` client sources are fetched straight from GitHub at the exact
      # pinned revisions, hermetically, from a plain `git clone` of this repo.
      cargoLock = {
        lockFile = ./Cargo.lock;
        outputHashes = {
          # github:vicondoa/d2b-toolkit @ 926de54e7320599c373524a10b65aaf13b6ff422
          "d2b-client-toolkit-2.0.0" = "sha256-vGb04cQDlO8KBoI5n0N//LLKhoLX8wK4nE0wu2UMJjQ=";
          # github:vicondoa/d2b @ 9dc902243cdd7aba7ef269988b96f0aae6e037da
          "d2b-client-2.0.0" = "sha256-mDNv+gkV0GKOFDWJEunuR76mPIwQsSg9AJcxsI5qhMQ=";
          "d2b-contracts-2.0.0" = "sha256-mDNv+gkV0GKOFDWJEunuR76mPIwQsSg9AJcxsI5qhMQ=";
          "d2b-session-2.0.0" = "sha256-mDNv+gkV0GKOFDWJEunuR76mPIwQsSg9AJcxsI5qhMQ=";
          "d2b-session-unix-2.0.0" = "sha256-mDNv+gkV0GKOFDWJEunuR76mPIwQsSg9AJcxsI5qhMQ=";
        };
      };

      runtimeBins =
        pkgs: with pkgs; [
          quickshell
          xdg-utils
        ];
      runtimeFonts = pkgs: with pkgs; [ material-symbols ];

      # Shared base for the package build and the hermetic fmt/clippy checks
      # below: same source, same vendored git/registry dependencies, same
      # toolchain. Each check overrides only the phases it needs.
      mkCargoDerivation =
        pkgs: extraAttrs:
        pkgs.rustPlatform.buildRustPackage (
          {
            pname = "d2b-wlcontrol";
            inherit version cargoLock;
            src = pkgs.lib.cleanSource ./.;
          }
          // extraAttrs
        );
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = mkCargoDerivation pkgs {
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
          fmt = mkCargoDerivation pkgs {
            pname = "d2b-wlcontrol-fmt-check";
            nativeBuildInputs = [ pkgs.rustfmt ];
            buildPhase = ''
              runHook preBuild
              cargo fmt --all -- --check
              runHook postBuild
            '';
            doCheck = false;
            dontFixup = true;
            installPhase = "mkdir -p $out";
          };
          clippy = mkCargoDerivation pkgs {
            pname = "d2b-wlcontrol-clippy-check";
            nativeBuildInputs = [ pkgs.clippy ];
            buildPhase = ''
              runHook preBuild
              cargo clippy --workspace --all-targets --offline -- -D warnings
              runHook postBuild
            '';
            doCheck = false;
            dontFixup = true;
            installPhase = "mkdir -p $out";
          };
          release-metadata = pkgs.runCommand "d2b-wlcontrol-release-metadata-${version}" { } ''
            grep -Fq 'version = "2.0.0"' ${./Cargo.toml}
            grep -Fq '## [Unreleased]' ${./CHANGELOG.md}
            grep -Fq '926de54e7320599c373524a10b65aaf13b6ff422' ${./Cargo.toml}
            grep -Fq '926de54e7320599c373524a10b65aaf13b6ff422' ${./flake.lock}
            grep -Fq '9dc902243cdd7aba7ef269988b96f0aae6e037da' ${./Cargo.toml}
            grep -Fq '5a20cef3a64281df819eeb76bdfe385999755479b467b559653011582fb9c043' ${./Cargo.toml}
            grep -Fq '35c33c2e23e1b9f03b5abc3bbca2d3320e38c42dfc7aceb7e3476d28210cde8c' ${./Cargo.toml}
            grep -Fq 'git = "https://github.com/vicondoa/d2b-toolkit"' ${./Cargo.toml}
            ! grep -Fq '../d2b-client-toolkit' ${./Cargo.toml}
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
