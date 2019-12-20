with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "rust-env";

  buildInputs = [
    zsh
    git pkg-config rustup 
    libudev
  ];
  shellHook = ''
    export NIX_ENFORCE_PURITY=0
    export PKG_CONFIG_ALLOW_CROSS=1
    exec zsh
  '';
}
