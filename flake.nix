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
          overlays = [ rust-overlay.overlays.default ];
        };

        # Common Args
        commonArgs = {
          pname = "robbb";
          # Will Require `--impure` to be passed to it can read that ENV VAR
          version = if builtins.getEnv "VERSION" != "" then
            builtins.getEnv "VERSION"
          else
            "0.0.1";
          src = builtins.path { path = pkgs.lib.cleanSource ./.; name = "robbb"; };
          nativeBuildInputs = with pkgs; [ rust-toolchain pkg-config ];
          buildInputs = with pkgs; [ openssl sqlx-cli rust-analyzer sqlite ];
        };
        # Use the toolchain from the `rust-toolchain` file
        rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;

        # Build Deps so We don't have to build them everytime
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Run Cargo Fmt
        robbbFmt = craneLib.cargoFmt (commonArgs // { inherit cargoArtifacts; });

        # Run Clippy (only if cargo fmt passes)
        robbbClippy = craneLib.cargoClippy (commonArgs // {
          cargoArtifacts = robbbFmt;
          cargoClippyExtraArgs = "-- -D warnings";
        });

        # Build Robb (only if all above tests pass)
        robbb = craneLib.buildPackage
          (commonArgs // { cargoArtifacts = robbbClippy; });
      in {
        # `nix flake check` (build, fmt and clippy)
        checks = { inherit robbb; };

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
          packages = [ pkgs.darwin.apple_sdk.frameworks.Security ] ++ commonArgs.nativeBuildInputs ++ commonArgs.buildInputs;
        };
      });
}
