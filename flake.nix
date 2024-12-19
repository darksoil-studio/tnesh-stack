{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";

    holonix.url = "github:holochain/holonix/main-0.4";
    holonix.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.follows = "holonix/rust-overlay";
    crane.follows = "holonix/crane";
  };

  nixConfig = {
    extra-substituters = [
      "https://holochain-ci.cachix.org"
      "https://darksoil-studio.cachix.org"
    ];
    extra-trusted-public-keys = [
      "holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8="
      "darksoil-studio.cachix.org-1:UEi+aujy44s41XL/pscLw37KEVpTEIn8N/kn7jO8rkc="
    ];
  };

  outputs = inputs@{ ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } rec {
      flake = {
        flakeModules.builders = ./nix/builders-option.nix;
        flakeModules.dependencies = ./nix/dependencies-option.nix;

        lib = rec {
          # TODO: remove this when scaffolding is fixed again
          wrapCustomTemplate = { system, pkgs, customTemplatePath }:
            let scaffolding = inputs.holonix.packages.${system}.hc-scaffold;
            in pkgs.runCommand "hc-scaffold" {
              buildInputs = [ pkgs.makeWrapper ];
              src = customTemplatePath;
            } ''
                mkdir $out
                mkdir $out/bin
                # We create the bin folder ourselves and link every binary in it
                ln -s ${scaffolding}/bin/* $out/bin
                # Except the hello binary
                rm $out/bin/hc-scaffold
                cp $src -R $out/template
                # Because we create this ourself, by creating a wrapper
                makeWrapper ${scaffolding}/bin/hc-scaffold $out/bin/hc-scaffold \
                  --add-flags "--template $out/template"
              	'';
          filterPnpmSources = { lib }:
            orig_path: type:
            let
              path = (toString orig_path);
              base = baseNameOf path;

              matchesSuffix = lib.any (suffix: lib.hasSuffix suffix base) [
                ".ts"
                ".js"
                ".json"
                ".yaml"
                ".html"
              ];
            in type == "directory" || matchesSuffix;
          cleanPnpmDepsSource = { lib }:
            src:
            lib.cleanSourceWith {
              src = lib.cleanSource src;
              filter = filterPnpmSources { inherit lib; };

              name = "pnpm-workspace";
            };
          filterScaffoldingSources = { lib }:
            orig_path: type:
            let
              path = (toString orig_path);
              base = baseNameOf path;
              parentDir = baseNameOf (dirOf path);

              matchesSuffix = lib.any (suffix: lib.hasSuffix suffix base) [
                # Keep rust sources
                ".rs"
                # Keep all toml files as they are commonly used to configure other
                # cargo-based tools
                ".toml"
                # Keep templates
                ".hbs"
              ];

              # Cargo.toml already captured above
              isCargoFile = base == "Cargo.lock";

              # .cargo/config.toml already captured above
              isCargoConfig = parentDir == ".cargo" && base == "config";
            in type == "directory" || matchesSuffix || isCargoFile
            || isCargoConfig;
          cleanScaffoldingSource = { lib }:
            src:
            lib.cleanSourceWith {
              src = lib.cleanSource src;
              filter = filterScaffoldingSources { inherit lib; };

              name = "scaffolding-workspace";
            };
        };
      };

      imports = [
        ./crates/scaffold_remote_zome/default.nix
        ./crates/compare_dnas_integrity/default.nix
        ./crates/zome_wasm_hash/default.nix
        ./crates/sync_npm_git_dependencies_with_nix/default.nix
        ./crates/dna_hash/default.nix
        ./crates/scaffold_tnesh_zome/default.nix
        ./nix/builders-option.nix
        ./nix/dependencies-option.nix
      ];

      systems = builtins.attrNames inputs.holonix.devShells;

      perSystem = { inputs', self', config, pkgs, system, lib, ... }: rec {
        dependencies.holochain.buildInputs = (with pkgs; [ perl openssl go bzip2 ])
          ++ (lib.optionals pkgs.stdenv.isLinux [ pkgs.pkg-config ]);
        builders = {
          rustZome = { crateCargoToml, workspacePath, cargoArtifacts ? null
            , matchingZomeHash ? null, meta ? { }, zomeEnvironmentVars ? { } }:
            let
              deterministicCraneLib = let
                rustToolchain =
                  inputs.holonix.outputs.packages."x86_64-linux".rust;
              in (inputs.crane.mkLib
                inputs.holonix.inputs.nixpkgs.outputs.legacyPackages.${
                  "x86_64-linux"
                }).overrideToolchain rustToolchain;

              craneLib = (inputs.crane.mkLib pkgs).overrideToolchain
                inputs'.holonix.packages.rust;
              zome-wasm-hash = self'.packages.zome-wasm-hash;

            in pkgs.callPackage ./nix/zome.nix {
              inherit deterministicCraneLib craneLib crateCargoToml
                cargoArtifacts workspacePath matchingZomeHash zome-wasm-hash
                meta zomeEnvironmentVars;
            };
          sweettest = { dna, workspacePath, crateCargoToml, buildInputs ? [ ]
            , nativeBuildInputs ? [ ], cargoArtifacts ? null }:
            let
              craneLib = (inputs.crane.mkLib pkgs).overrideToolchain
                inputs'.holonix.packages.rust;
            in pkgs.callPackage ./nix/sweettest.nix {
              inherit dna craneLib crateCargoToml cargoArtifacts workspacePath;
              buildInputs = buildInputs
                ++ self'.dependencies.holochain.buildInputs;
            };
          dna = { dnaManifest, zomes, matchingIntegrityDna ? null, meta ? { } }:
            pkgs.callPackage ./nix/dna.nix {
              inherit zomes dnaManifest matchingIntegrityDna meta;
              compare-dnas-integrity = self'.packages.compare-dnas-integrity;
              holochain = inputs'.holonix.packages.holochain;
              dna-hash = self'.packages.dna-hash;
            };
          happ = { happManifest, dnas, meta ? { } }:
            pkgs.callPackage ./nix/happ.nix {
              inherit dnas happManifest meta;
              holochain = inputs'.holonix.packages.holochain;
            };
        };

        devShells.holochainDev = pkgs.mkShell {
          buildInputs = self'.dependencies.holochain.buildInputs;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [
            devShells.holochainDev
            inputs'.holonix.devShells.default
            devShells.synchronized-pnpm
          ];
        };

        packages = {
          zomeCargoArtifacts = let
            craneLib = (inputs.crane.mkLib pkgs).overrideToolchain
              inputs'.holonix.packages.rust;
            src =
              craneLib.cleanCargoSource (craneLib.path ./nix/reference-happ);
            commonArgs = {
              inherit src;
              doCheck = false;
              CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
            };
            cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
              pname = "zome";
              version = "for-holochain-0.4.x";
            });

          in cargoArtifacts;

          holochainCargoArtifacts = let
            craneLib = (inputs.crane.mkLib pkgs).overrideToolchain
              inputs'.holonix.packages.rust;
            cargoArtifacts = craneLib.buildDepsOnly {
              buildInputs = self'.dependencies.holochain.buildInputs;
              src =
                craneLib.cleanCargoSource (craneLib.path ./nix/reference-happ);
              doCheck = false;
              # RUSTFLAGS =
              #   "--remap-path-prefix ${cargoVendorDir}=/build/source/";
              # CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS =
              #   " -Clink-arg=-fuse-ld=mold";
              # CARGO_PROFILE = "release";
              CARGO_PROFILE = "release";
              pname = "sweettest";
              version = "for-holochain-0.4.x";
            };
          in cargoArtifacts;
        };

        packages.synchronized-pnpm = pkgs.symlinkJoin {
          name = "synchronized-pnpm";
          paths = [ inputs'.nixpkgs.legacyPackages.pnpm ];
          buildInputs = [ pkgs.makeWrapper ];
          postBuild = ''
            wrapProgram $out/bin/pnpm  --run ${self'.packages.sync-npm-git-dependencies-with-nix}/bin/sync-npm-git-dependencies-with-nix
          '';
        };

        devShells.synchronized-pnpm = pkgs.mkShell {
          packages = let
            npm-warning = pkgs.writeShellScriptBin "echo-npm-warning" ''
              							echo "
              -----------------

              WARNING: this repository is not managed with npm, but pnpm.

              Don't worry! They are really similar to each other. Here are some helpful reminders:
                            
              If you are trying to run \`npm install\`, you can run \`pnpm install\`
              If you are trying to run \`npm install some_dependency\`, you can run \`pnpm add some_dependency\`
              If you are trying to run a script like \`npm run build\`, you can run \`pnpm build\`
              If you are trying to run a script for a certain workspace like \`npm run build -w ui\`, you can run \`pnpm -F ui build\`

              The npm command that you just ran will continue now, but it is recommended that you do all commands in this repository with pnpm.

              -----------------

              "
            '';
            npm-with-warning = pkgs.symlinkJoin {
              name = "npm";
              paths = [ pkgs.nodejs_20 ];
              buildInputs = [ pkgs.makeWrapper ];
              postBuild =
                "    wrapProgram $out/bin/npm \\\n		  --run ${npm-warning}/bin/echo-npm-warning\n  ";
            };
          in [
            npm-with-warning
            pkgs.nodejs_20
            packages.synchronized-pnpm
            self'.packages.sync-npm-git-dependencies-with-nix
          ];

          shellHook = ''
            sync-npm-git-dependencies-with-nix
          '';
        };

        packages.hc-scaffold-happ = let
          hcScaffold = flake.lib.wrapCustomTemplate {
            inherit pkgs system;
            customTemplatePath = ./templates/app;
          };
        in pkgs.writeShellScriptBin "hc-scaffold" ''
          if [[ "$@" == *"web-app"* ]]; then
            ${hcScaffold}/bin/hc-scaffold "$@" --package-manager pnpm --setup-nix -F  
          elif [[ "$@" == *"zome"* ]]; then
            ${hcScaffold}/bin/hc-scaffold "$@"
            git add Cargo.lock
          else
            ${hcScaffold}/bin/hc-scaffold "$@"
          fi
        '';

        packages.hc-scaffold-zome = flake.lib.wrapCustomTemplate {
          inherit pkgs system;
          customTemplatePath = ./templates/zome;
        };
      };
    };
}
