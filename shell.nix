{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  packages = with pkgs; [
    cargo
    clippy
    ffmpeg
    nodejs_22
    openssl
    pkg-config
    rustc
    rustfmt
  ];

  RUST_BACKTRACE = "1";
}
