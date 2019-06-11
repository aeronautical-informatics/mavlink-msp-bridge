with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "rust-env";

  buildInputs = [
    git pkgconfig rustup 
    
    libudev
  ];
}
