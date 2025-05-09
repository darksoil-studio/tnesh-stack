{ inputs, ... }:

{
  perSystem = { inputs', config, pkgs, system, lib, self', options, ... }: {
    packages = {
      my_happ = inputs.tnesh-stack.outputs.builders.${system}.happ {
        happManifest = ./happ.yaml;
        dnas = { my_dna = inputs'.service.packages.my_dna; };
      };
    };
  };
}

