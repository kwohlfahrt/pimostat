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

  cargoSha256 = "1493lys8kz18yr0hlxvsnaiib90mz0aqmws196anhqndykc0lggr";

  meta = with stdenv.lib; {
    platforms = platforms.all;
  };
}
