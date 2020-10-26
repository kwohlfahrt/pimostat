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

  cargoSha256 = "0hy99n8a03whmx6385yx2caz1g7vp4hp3xkkj0qlwylcd5paf186";

  preCheck = ''
    (cd ./tests/ssl/ && ${bash}/bin/bash ./gen_certs.sh)
  '';

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
