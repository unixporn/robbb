{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [ rustc cargo gcc pkg-config ];

  buildInputs = with pkgs; [ openssl sqlx-cli rustfmt clippy ];
  shellHook = ''
    export $(cat .env)
  '';
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
