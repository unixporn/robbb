{ callPackage,
  pkg-config,
  openssl,
  dockerTools,
  trupSrc,
  trupRev,
  sources ? import ./nix/sources.nix,
  naersk ? callPackage sources.naersk {},
}: rec {
  trup-rs = naersk.buildPackage rec {
    name = "trup-rs";
    version = "master";
    src = trupSrc;
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [ openssl ];
    VERSION = trupRev;
  };

  image = dockerTools.buildImage {
    name = "trup-rs";
    tag = trupRev;
    config = {
      Cmd = [ "${trup-rs}/bin/trup-rs" ];
    };
  };
}
