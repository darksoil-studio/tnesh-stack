{ inputs, ... }:

{
  perSystem = { inputs', self', system, ... }: {
    packages.my_zome_integrity =
      inputs.tnesh-stack.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
        cargoArtifacts = inputs'.tnesh-stack.packages.zomeCargoArtifacts;
        # matchingZomeHash = inputs'.previousZomeVersion.packages.my_zome;
      };
  };
}
