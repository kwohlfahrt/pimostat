{
  description = "Raspberry Pi based thermostat";

  inputs = {
    nixpkgs.url = "nixpkgs";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
  };

  # adapted from https://hoverbear.org/blog/a-flake-for-your-crate/#flake-nix
  outputs = { self, nixpkgs, naersk } : let
    systems = [ "x86_64-linux" "aarch64-linux" ];
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);

    pimostat = { naersk, lib, capnproto, openssl, pkgconfig, bash, targetPlatform }:
    naersk.lib."${targetPlatform.system}".buildPackage rec {
      pname = "pimostat";
      root = ./.;
      buildInputs = [ openssl ];
      nativeBuildInputs = [ capnproto pkgconfig ];

      checkInputs = [ openssl.bin ];

      preCheck = ''
        (cd ./tests/ssl/ && ${bash}/bin/bash ./gen_certs.sh)
      '';

      meta = with lib; {
        platforms = platforms.all;
      };
    };
  in {
    defaultPackage = forAllSystems (system: (import nixpkgs { inherit system; overlays = [ self.overlay ]; }).pimostat);
    overlay = self: super: {
      pimostat = self.callPackage pimostat { inherit naersk; };
    };
    nixosModule = import ./module.nix;
  };
}

