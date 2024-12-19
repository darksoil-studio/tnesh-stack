{ inputs, ... }:

{
  perSystem = { inputs', pkgs, self', lib, ... }: {

    packages.sync-npm-git-dependencies-with-nix = let
      # Uncomment this line when holonix gets updated to 24.11
      # craneLib = inputs.crane.mkLib inputs'.pnpmnixpkgs.legacyPackages;
      craneLib = inputs.crane.mkLib inputs'.pnpmnixpkgs.legacyPackages;

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

