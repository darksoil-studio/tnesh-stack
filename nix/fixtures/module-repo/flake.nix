{
  description = "Template for Holochain app development";

  inputs = {
    nixpkgs.follows = "holonix/nixpkgs";
    holonix.url = "github:holochain/holonix";

    tnesh-stack.url = "path:./../../..";
    profiles-zome.url = "github:darksoil-studio/profiles-zome/main-0.5";
    # previousZomeVersion.url = "github:darksoil-studio/tnesh-stack/67dffe4af2c8675cd47d0b404fd0473d6a93ddfd?dir=nix/fixtures/module-repo";
  };

  outputs = inputs@{ ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./zomes/coordinator/zome.nix
        ./zomes/integrity/zome.nix
        inputs.tnesh-stack.flakeModules.builders
      ];

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, lib, self', ... }: {
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            inputs'.tnesh-stack.devShells.synchronized-pnpm
            inputs'.tnesh-stack.devShells.holochainDev
            # inputs'.tnesh-stack.devShells.zomeDev
            # inputs'.tnesh-stack.devShells.sweettestDev
            inputs'.holonix.devShells.default
          ];
          packages = [ pkgs.nodejs_20 ];

        };
      };
    };
}
