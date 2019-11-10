with import <nixpkgs> {}; callPackage ./pimostat.nix {} // {
  # Environment Variables
  RUST_BACKTRACE = 1;
}
