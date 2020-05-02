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

  cargoSha256 = "16vkypw5v4f1jm5kvhcgd3b4d1wjjcdwrgf4h9cml98s9lgfviy6";

  preCheck = ''
    (cd ./tests/ssl/ && ${bash}/bin/bash ./gen_certs.sh)
  '';

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
