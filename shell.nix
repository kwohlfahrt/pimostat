let
  overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> { overlays = [ overlay ]; };
in
  pkgs.stdenv.mkDerivation {
  name = "rust-env";
  buildInputs = with pkgs.latest.rustChannels.stable; [ rust cargo pkgs.capnproto ];

  # Set Environment Variables
  RUST_BACKTRACE = 1;
}
