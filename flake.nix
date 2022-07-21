{

  description = "r/unixporn discord bot";
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.05";
    utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, crane, rust-overlay, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        rust-toolchain =
          pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;
        commonArgs = {
          src = ./.;
          nativeBuildInputs = with pkgs; [ rust-toolchain pkg-config ];
          buildInputs = with pkgs; [
            openssl
            sqlx-cli
            rust-analyzer
            sqlitebrowser
            sqlite
          ];
        };
        # Build Deps so We don't have to build them everytime
        cargoArtifacts =
          craneLib.buildDepsOnly (commonArgs // { pname = "robbb-deps"; });
        # Run Clippy
        robbbClippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "-- -D warnings";
        });
        robbb = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "robbb";
        });
      in {
        # Run Checks
        checks = { inherit robbb robbbClippy; };
        # `nix build`
        packages.default = robbb;

        # `nix run`
        apps.default = utils.lib.mkApp { drv = robbb; };

        # `nix develop`
        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks;

          shellHook = ''
            export $(cat .env)
          '';
          # Extra inputs can be added here
          inherit (commonArgs) nativeBuildInputs;
          inherit (commonArgs) buildInputs;
        };
      });
}
