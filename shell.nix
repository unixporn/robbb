{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  packages = with pkgs; [
    rustc
    cargo
    rust-analyzer
    gcc
    pkg-config
    openssl
    sqlx-cli
    rustfmt
    clippy
    sqlitebrowser
    sqlite
  ];

  shellHook = ''
    export $(cat .env)
  '';

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
