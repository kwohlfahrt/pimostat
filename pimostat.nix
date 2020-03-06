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

  cargoSha256 = "1gl9n6rq5wlsnvcida2ln1py47hnwrc6vb732fziyd546s5w0kpw";

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
