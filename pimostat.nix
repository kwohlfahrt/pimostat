{ stdenv, rustPlatform, capnproto }:

rustPlatform.buildRustPackage rec {
  pname = "pimostat";
  version = "0.1.0";

  src = ./.;
  nativeBuildInputs = [ capnproto ];

  cargoSha256 = "1waayzfwgp2fa0k9178sfrfd91zr8als6kb633vmqwbppxlqs56c";
}
