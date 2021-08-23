{ stdenv, lib, makeRustPlatform, rustChannels, capnproto, openssl, pkgconfig, bash }:

let
  rustPlatform = with rustChannels.stable; makeRustPlatform {
    inherit cargo;
    rustc = rust;
  };
in rustPlatform.buildRustPackage rec {
  pname = "pimostat";
  version = "0.1.0";

  src = ./.;
  buildInputs = [ openssl ];
  nativeBuildInputs = [ capnproto pkgconfig ];
  checkInputs = [ openssl.bin ];

  cargoSha256 = "0xrpji4i8p3kbaqz9dsj6z1x2mp6vc07qdlkyspljkpgbbswiy6v";

  preCheck = ''
    (cd ./tests/ssl/ && ${bash}/bin/bash ./gen_certs.sh)
  '';

  meta = with lib; {
    platforms = platforms.all;
  };
}
