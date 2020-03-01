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

  cargoSha256 = "0plfp4666ffdzwfywrm8r2hwrrp66pvfp6c8h6x639xy7kjnrm6y";

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
