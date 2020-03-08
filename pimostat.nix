{ stdenv, makeRustPlatform, rustChannels, capnproto }:

let
  rustPlatform = with rustChannels.stable; makeRustPlatform {
    inherit cargo;
    rustc = rust;
  };
in rustPlatform.buildRustPackage rec {
  pname = "pimostat";
  version = "0.1.0";

  src = ./.;
  nativeBuildInputs = [ capnproto ];

  cargoSha256 = "14mask7jv2vhdhbc1zf4444mrdd08ibqsmcjwwkiybz6czss45zy";

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
