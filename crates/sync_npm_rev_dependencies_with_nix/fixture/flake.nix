{
  description = "Template for Holochain app development";

  inputs = {
    nixpkgs.follows = "holonix/nixpkgs";

    holonix.url = "github:holochain/holonix/main-0.5";
    tnesh-stack.url = "path:../../..";
    profiles-zome.url = "github:darksoil-studio/profiles-zome/main-0.5";
    file-storage.url = "github:darksoil-studio/file-storage/main-0.5";
  };

  outputs = inputs@{ ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, lib, self', ... }: { };
    };
}
