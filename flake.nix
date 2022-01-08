{
  description = "Raspberry Pi based thermostat";

  inputs = {
    nixpkgs.url = "nixpkgs";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, naersk } : let
    systems = [ "x86_64-linux" "aarch64-linux" ];
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);

    pimostat = { naersk-lib, lib, capnproto, openssl, pkgconfig, bash }:
    naersk-lib.buildPackage rec {
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
    defaultPackage = forAllSystems (system: nixpkgs.legacyPackages.${system}.callPackage pimostat {
      naersk-lib = naersk.lib."${system}";
    });
  };
}

