{ callPackage,
  pkg-config,
  openssl,
  cacert,
  dockerTools,
  robbbSrc,
  robbbRev,
  sources ? import ./nix/sources.nix,
  naersk ? callPackage sources.naersk {},
}: rec {
  robbb = naersk.buildPackage rec {
    name = "robbb";
    version = "master";
    src = robbbSrc;
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [ openssl ];
    VERSION = robbbRev;
  };

  image = dockerTools.buildImage {
    name = "robbb";
    tag = robbbRev;
    config = {
      Cmd = [ "${robbb}/bin/robbb" ];
      Env = [
        "NIX_SSL_CERT_FILE=${cacert}/etc/ssl/certs/ca-bundle.crt"
      ];
    };
  };
}
