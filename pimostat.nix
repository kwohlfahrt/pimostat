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
  buildInputs = [ openssl ];
  nativeBuildInputs = [ capnproto pkgconfig ];
  checkInputs = [ openssl.bin ];

  cargoSha256 = "087hn84j8kaljdnmpi619pm9f646ihy3kylv16iq530jp39r52js";

  preCheck = ''
    (cd ./tests/ssl/ && ${bash}/bin/bash ./gen_certs.sh)
  '';

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
