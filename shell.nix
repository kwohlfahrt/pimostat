{ pkgs ? import <nixpkgs> {}}:

let
  src = pkgs.fetchFromGitHub {
    owner = "mozilla";
    repo = "nixpkgs-mozilla";
    # commit from: 2018-03-27
    rev = "b9c99d043b1cb55ee8c08265223b7c35d687acb9";
    sha256 = "0akyhdv5p0qiiyp6940k9bvismjqm9f8xhs0gpznjl6509dwgfxl";
  };

  overlay = (import "${src.out}/rust-overlay.nix" pkgs pkgs).latest;

in pkgs.stdenv.mkDerivation {
  name = "rust-env";
  buildInputs = with overlay.rustChannels.stable; [ rust cargo pkgs.capnproto ];

  # Set Environment Variables
  RUST_BACKTRACE = 1;
}
