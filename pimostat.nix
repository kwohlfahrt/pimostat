{ stdenv, rustPlatform, capnproto }:

rustPlatform.buildRustPackage rec {
  pname = "pimostat";
  version = "0.1.0";

  src = ./.;
  nativeBuildInputs = [ capnproto ];

  cargoSha256 = "04lcncgqbxcb5z26qbp5s5d8i092rr0c787pbh5igxc940fd7v4b";
}
