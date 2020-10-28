{ stdenv, makeRustPlatform, rustChannels, capnproto, openssl, pkgconfig, bash }:

let
  rustPlatform = with rustChannels.stable; makeRustPlatform {
    inherit cargo;
    rustc = rust;
  };
in rustPlatform.buildRustPackage rec {
  pname = "pimostat";
  version = "0.1.0";

  src = ./.;
  nativeBuildInputs = [ capnproto openssl pkgconfig ];

  cargoSha256 = "1q17psyrqia2si6mw93i25v7jhlawccr1930qp4ras1k8nfdl38l";

  preCheck = ''
    (cd ./tests/ssl/ && ${bash}/bin/bash ./gen_certs.sh)
  '';

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
