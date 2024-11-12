{ inputs, ... }:

{
  perSystem = { inputs', self', pkgs, system, lib, ... }: {

    packages.scaffold-tnesh-zome = let
      craneLib = inputs.crane.mkLib pkgs;

      cratePath = ./.;

      cargoToml =
        builtins.fromTOML (builtins.readFile "${cratePath}/Cargo.toml");
      crate = cargoToml.package.name;

      commonArgs = {
        src = craneLib.cleanCargoSource (craneLib.path ../../.);
        doCheck = false;
        buildInputs = self'.dependencies.holochain.buildInputs;
        cargoExtraArgs = "--locked --package scaffold_tnesh_zome";
      };
    in craneLib.buildPackage (commonArgs // {
      pname = crate;
      version = cargoToml.package.version;
    });

  };
}
