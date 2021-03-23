{ pkgs ? import <nixpkgs> {}  }:
pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo gcc openssl pkg-config sqlx-cli ];
          shellHook = ''
            export $(cat .env)
          '';
}
