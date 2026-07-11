{ self ? null }:
{ config, lib, pkgs, options, ... }:

let
  cfg = config.programs.d2b-wlcontrol;
  tomlFormat = pkgs.formats.toml { };
  packageForSystem =
    if self != null && self ? packages && self.packages ? ${pkgs.stdenv.hostPlatform.system}
    then self.packages.${pkgs.stdenv.hostPlatform.system}.default
    else null;
  executable =
    if cfg.package != null
    then lib.getExe cfg.package
    else "d2b-wlcontrol";
  baseSettings = {
    public_socket = cfg.publicSocketPath;
    refresh_interval_ms = cfg.refreshIntervalMs;
    command_timeout_ms = cfg.commandTimeoutMs;
    waybar = {
      icon = cfg.waybar.icon;
      label = cfg.waybar.label;
    };
    launcher_overrides = map
      (item: {
        target = item.target;
        item_id = item.itemId;
      }
      // lib.optionalAttrs (item.name != null) { name = item.name; }
      // lib.optionalAttrs (item.icon != null) { icon = item.icon; })
      cfg.launcherOverrides;
  };
  renderedSettings = lib.recursiveUpdate baseSettings cfg.settings;
  waybarModule = lib.recursiveUpdate {
    exec = "${executable} waybar";
    "return-type" = "json";
    "restart-interval" = 5;
    signal = 8;
    "on-click" = cfg.waybar.clickAction;
    "on-click-right" = "${executable} action cycle-display";
    "on-click-middle" = "${executable} action refresh";
    tooltip = true;
  } cfg.waybar.module;
  waybarHmAvailable =
    options ? programs
    && options.programs ? waybar
    && options.programs.waybar ? enable
    && options.programs.waybar ? settings;
  waybarStyleAvailable =
    options ? programs
    && options.programs ? waybar
    && options.programs.waybar ? style;
in
{
  options.programs.d2b-wlcontrol = {
    enable = lib.mkEnableOption "d2b workload control center";

    package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = packageForSystem;
      defaultText = lib.literalExpression "inputs.d2b-wlcontrol.packages.${pkgs.stdenv.hostPlatform.system}.default";
      description = "Package providing the d2b-wlcontrol binary.";
    };

    publicSocketPath = lib.mkOption {
      type = lib.types.str;
      default = "/run/d2b/public.sock";
      description = "Path to d2bd's public operator socket.";
    };

    refreshIntervalMs = lib.mkOption {
      type = lib.types.ints.positive;
      default = 2500;
      description = "Refresh cadence for the self-looping Waybar process.";
    };

    commandTimeoutMs = lib.mkOption {
      type = lib.types.ints.positive;
      default = 10000;
      description = "Public operation deadline in milliseconds.";
    };

    settings = lib.mkOption {
      type = tomlFormat.type;
      default = { };
      description = "Additional wlcontrol TOML settings merged over module defaults.";
    };

    launcherOverrides = lib.mkOption {
      default = [ ];
      description = "Presentation overrides for public configured launcher items.";
      type = lib.types.listOf (lib.types.submodule {
        options = {
          target = lib.mkOption {
            type = lib.types.str;
            description = "Canonical workload target.";
          };
          itemId = lib.mkOption {
            type = lib.types.str;
            description = "Configured launcher item ID.";
          };
          name = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "Optional display-name override.";
          };
          icon = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "Optional icon-name override.";
          };
        };
      });
    };

    waybar = {
      enable = lib.mkEnableOption "d2b-wlcontrol Waybar integration";

      moduleName = lib.mkOption {
        type = lib.types.str;
        default = "custom/d2b-wlcontrol";
        description = "Waybar custom-module name.";
      };

      barName = lib.mkOption {
        type = lib.types.str;
        default = "mainBar";
        description = "Waybar settings block receiving the module.";
      };

      modulesList = lib.mkOption {
        type = lib.types.enum [ "modules-left" "modules-center" "modules-right" ];
        default = "modules-right";
        description = "Waybar module list receiving the control indicator.";
      };

      icon = lib.mkOption {
        type = lib.types.str;
        default = "◆";
        description = "Bounded icon prefix rendered by wlcontrol.";
      };

      label = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "Optional bounded label rendered after the icon.";
      };

      clickAction = lib.mkOption {
        type = lib.types.str;
        default = "${executable} open";
        defaultText = "the selected package executable followed by open";
        description = "Waybar left-click command.";
      };

      module = lib.mkOption {
        type = lib.types.attrsOf lib.types.anything;
        default = { };
        description = "Waybar custom-module overrides.";
      };

      css = lib.mkOption {
        type = lib.types.lines;
        default = builtins.readFile ../data/style.css;
        description = "CSS/classes appended to Home Manager's Waybar style.";
      };

      injectHomeManager = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Inject the module, placement, and CSS when Home Manager manages Waybar.";
      };
    };
  };

  config = lib.mkIf cfg.enable (lib.mkMerge [
    {
      assertions = [
        {
          assertion = cfg.package != null;
          message = "programs.d2b-wlcontrol.package must be set when the module is imported without the flake output";
        }
      ];

      home.packages = [ cfg.package ];

      xdg.configFile."d2b-wlcontrol/config.toml".source =
        tomlFormat.generate "d2b-wlcontrol-config.toml" renderedSettings;

      xdg.configFile."d2b-wlcontrol/waybar-module.json".text =
        builtins.toJSON { ${cfg.waybar.moduleName} = waybarModule; } + "\n";

      xdg.configFile."d2b-wlcontrol/style.css".text = cfg.waybar.css;
    }
    (lib.mkIf (
      cfg.waybar.enable
      && cfg.waybar.injectHomeManager
      && waybarHmAvailable
      && config.programs.waybar.enable
    ) {
      programs.waybar.settings.${cfg.waybar.barName} = {
        ${cfg.waybar.moduleName} = waybarModule;
        ${cfg.waybar.modulesList} = lib.mkAfter [ cfg.waybar.moduleName ];
      };
    })
    (lib.mkIf (
      cfg.waybar.enable
      && cfg.waybar.injectHomeManager
      && waybarStyleAvailable
      && config.programs.waybar.enable
    ) {
      programs.waybar.style = lib.mkAfter ("\n" + cfg.waybar.css);
    })
  ]);
}
