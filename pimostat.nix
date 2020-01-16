{ stdenv, rustPlatform, capnproto }:

rustPlatform.buildRustPackage rec {
  pname = "pimostat";
  version = "0.1.0";

  src = ./.;
  nativeBuildInputs = [ capnproto ];

  cargoSha256 = "1h4lvrrh93b426xn6siblc8gk90n3b9rlpk0x0byisai74rby7wl";
}
