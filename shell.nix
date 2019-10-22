with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "rust-env";

  buildInputs = [
    git pkg-config rustup 
    libudev
  ];
}
