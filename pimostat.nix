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

  cargoSha256 = "0wd6jpbw8gki3pgd3h55bn9h51cpn4pg3k28z7iz1fja9s5pbr5b";

  preCheck = ''
    (cd ./tests/ssl/ && ${bash}/bin/bash ./gen_certs.sh)
  '';

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
