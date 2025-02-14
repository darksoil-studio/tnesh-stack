{
  description = "Template for Holochain app development";

  inputs = {
    nixpkgs.follows = "holonix/nixpkgs";

    holonix.url = "github:holochain/holonix/main-0.4";
    tnesh-stack.url = "path:../../..";
    profiles-zome.url = "github:darksoil-studio/profiles-zome/main-0.4";
    file-storage.url =
      "github:darksoil-studio/file-storage/d4819e883590e7f23589a2f137512910209ff424";
  };

  outputs = inputs@{ ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, lib, self', ... }: { };
    };
}
