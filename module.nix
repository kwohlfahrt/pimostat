{ config, pkgs, lib, ... }:

let
  cfg = config.services.pimostat;
  inherit (lib) types mkOption mkIf optional escapeShellArg;
  typeTemp = types.strMatching "[0-9]+(.[0-9]+)?";
in {
  options.services.pimostat.sensor = {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Whether to enable the sensor component of pimostat.
      '';
    };

    port = mkOption {
      type = types.port;
      description = ''
        The port to listen for incoming connections on.
      '';
    };

    file = mkOption {
      type = types.path;
      description = ''
        The w1-therm compatible file to read temperatures from.
      '';
    };

    interval = mkOption {
      type = types.ints.positive;
      default = 60;
      description = ''
        The interval (in seconds) at which to read the temperature.
      '';
    };
  };

  options.services.pimostat.controller = {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Whether to enable the controller component of pimostat.
      '';
    };

    port = mkOption {
      type = types.port;
      description = ''
        The port to listen for incoming connections on.
      '';
    };

    sensor = mkOption {
      type = types.str;
      description = ''
        The URL of the sensor to read from.
      '';
    };

    temperature = mkOption {
      type = typeTemp;
      default = "19.0";
      description = ''
        The temperature to target set to.
      '';
    };

    hysteresis = mkOption {
      type = typeTemp;
      default = "1.0";
      description = ''
        The hysteresis range about the threshold.
      '';
    };
  };

  options.services.pimostat.actor = {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Whether to enable the actor component of pimostat.
      '';
    };

    controller = mkOption {
      type = types.str;
      description = ''
        The URL of the controller to listen to.
      '';
    };

    gpio = mkOption {
      type = types.path;
      description = ''
        The GPIO to write to in response to changes.
      '';
    };
  };

  config = {
    systemd.services.pimostat-sensor = mkIf cfg.sensor.enable {
      description = "w1-therm sensor service";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "simple";
        ExecStart = with cfg.sensor;
          "${pkgs.pimostat}/bin/sensor ${toString port} ${escapeShellArg file} ${toString interval}";
      };
    };

    systemd.services.pimostat-controller = mkIf cfg.controller.enable {
      description = "Pimostat controller service";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "simple";
        ExecStart = with cfg.controller;
          "${pkgs.pimostat}/bin/controller ${toString port} ${sensor} ${temperature} ${hysteresis}";
      };
    };

    systemd.services.pimostat-actor = mkIf cfg.actor.enable {
      description = "Pimostat actor service";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "simple";
        ExecStart = with cfg.actor;
          "${pkgs.pimostat}/bin/actor ${controller} ${gpio}";
      };
    };

    networking.firewall.allowedTCPPorts =
      (optional cfg.sensor.enable cfg.sensor.port) ++
      (optional cfg.controller.enable cfg.controller.port);
  };
}
