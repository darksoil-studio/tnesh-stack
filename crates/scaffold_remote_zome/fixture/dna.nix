{ inputs, ... }:

{
  perSystem = { inputs', self', lib, system, ... }: {
    packages.forum = inputs.tnesh-stack.outputs.builders.${system}.dna {
      dnaManifest = ./dna.yaml;
      zomes = { };
    };
  };
}

