{ inputs, self, ... }:

{
  perSystem = { inputs', pkgs, self', lib, ... }: {

    packages.compare-dnas-integrity = let
      craneLib = inputs.crane.mkLib pkgs;

      cratePath = ./.;

      cargoToml =
        builtins.fromTOML (builtins.readFile "${cratePath}/Cargo.toml");
      crate = cargoToml.package.name;

      commonArgs = {
        src = craneLib.cleanCargoSource (craneLib.path ../../.);
        doCheck = false;
        buildInputs = self'.dependencies.holochain.buildInputs;
      };
      cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
        pname = "tnesh-workspace";
        version = "0.4.x";
      });
    in craneLib.buildPackage (commonArgs // {
      pname = crate;
      version = cargoToml.package.version;
      inherit cargoArtifacts;
    });
  };
}

