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

    address = mkOption {
      type = types.str;
      description = ''
        The address to listen for incoming connections on.
      '';
    };

    certificate = mkOption {
      type = types.nullOr types.file;
      default = null;
      description = ''
        The SSL certificate to use for incoming connections.
      '';
    };

    file = mkOption {
      type = types.path;
      description = ''
        The w1-therm compatible file to read temperatures from.
      '';
    };

    interval = mkOption {
      type = types.nullOr types.ints.positive;
      default = null;
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

    address = mkOption {
      type = types.str;
      description = ''
        The address to listen for incoming connections on.
      '';
    };

    certificate = mkOption {
      type = types.nullOr types.file;
      default = null;
      description = ''
        The SSL certificate to use for incoming connections.
      '';
    };

    sensor = mkOption {
      type = types.str;
      description = ''
        The address of the sensor to connect to.
      '';
    };

    disableTls = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Do not require TLS on connection to the sensor.
      '';
    }

    temperature = mkOption {
      type = typeTemp;
      default = "19.0";
      description = ''
        The temperature to target set to.
      '';
    };

    hysteresis = mkOption {
      type = types.nullOr typeTemp;
      default = null;
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
        The address of the controller to listen to.
      '';
    };

    disableTls = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Do not require TLS on connection to the controller.
      '';
    }

    gpio = mkOption {
      type = types.path;
      description = ''
        The GPIO to write to in response to changes.
      '';
    };
  };

  config = {
    systemd.services.pimostat-sensor = with cfg.sensor; mkIf enable {
      description = "w1-therm sensor service";
      serviceConfig = {
        Type = "simple";
        ExecStart = let
          options = lib.concatStringsSep " " [
            (if interval != null then "--interval ${interval}" else "")
            (if certificate != null then "--cert ${certificate}" else "")
          ];
        in "${pkgs.pimostat}/bin/sensor ${escapeShellArg file} ${toString interval}";
      };
    };

    systemd.sockets.pimostat-sensor = with cfg.sensor; mkIf enable {
      description = "Pimostat sensor socket";
      socketConfig = {
        ListenStream = "${toString port}";
      };
      wantedBy = [ "sockets.target" ];
    };

    systemd.services.pimostat-controller = with cfg.controller; mkIf enable {
      description = "Pimostat controller service";
      serviceConfig = {
        Type = "simple";
        ExecStart = let
          options = lib.concatStringsSep " " [
            (if disabletls then "--no-tls" else "")
            (if hysteresis != null then "--hysteresis ${hysteresis}" else "")
            (if certificate != null then "--cert ${certificate}" else "")
          ];
        in "${pkgs.pimostat}/bin/controller ${options} ${sensor} ${temperature}";
      };
    };

    systemd.sockets.pimostat-controller = with cfg.controller; mkIf enable {
      description = "Pimostat controller socket";
      socketConfig = {
        ListenStream = "${toString port}";
      };
      wantedBy = [ "sockets.target" ];
    };

    systemd.services.pimostat-actor = with cfg.actor; mkIf enable {
      description = "Pimostat actor service";
      after = [ "sockets.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "simple";
        ExecStart = let
          tls = if disableTls then "--no-tls" else "";
        in "${pkgs.pimostat}/bin/actor ${tls} ${controller} ${gpio}";
      };
    };

    networking.firewall.allowedTCPPorts =
      (optional cfg.sensor.enable cfg.sensor.port) ++
      (optional cfg.controller.enable cfg.controller.port);
  };
}
