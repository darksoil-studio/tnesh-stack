{
  inputs = {
    holonix.url = "github:holochain/holonix/main-0.4";
    nixpkgs.follows = "holonix/nixpkgs";
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
        ./crates/sync_npm_rev_dependencies_with_nix/default.nix
        ./crates/dna_hash/default.nix
        ./crates/scaffold_tnesh_zome/default.nix
        ./nix/builders-option.nix
        ./nix/dependencies-option.nix
      ];

      systems = builtins.attrNames inputs.holonix.devShells;

      perSystem = { inputs', self', config, pkgs, system, lib, ... }: rec {
        dependencies.holochain.buildInputs = (with pkgs; [ perl openssl go ])
          ++ (pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.pkg-config ])
          ++ (pkgs.lib.optionals (system == "x86_64-darwin")
            [ pkgs.apple-sdk_10_15 ]);
        builders = {
          rustZome = { crateCargoToml, workspacePath, cargoArtifacts ? null
            , matchingZomeHash ? null, meta ? { }, zomeEnvironmentVars ? { }
            , excludedCrates ? [ ] }:
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
                meta zomeEnvironmentVars excludedCrates;
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
          webhapp = { name, ui, happ, meta ? { } }:
            pkgs.callPackage ./nix/webhapp.nix {
              inherit name happ ui meta;
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

        packages.holochain = inputs'.holonix.packages.holochain.override {
          cargoExtraArgs =
            " --features unstable-functions,unstable-sharding,unstable-countersigning";
        };

        packages.synchronized-pnpm = let
          pnpm-sync-npm-rev-dependencies-with-nix = pkgs.symlinkJoin {
            name = "pnpm-sync-npm-rev-dependencies-with-nix";
            paths = [ self'.packages.sync-npm-rev-dependencies-with-nix ];
            buildInputs = [ pkgs.makeWrapper ];
            postBuild = ''
              wrapProgram $out/bin/sync-npm-rev-dependencies-with-nix --add-flags "--package-manager pnpm"
            '';
          };
        in pkgs.symlinkJoin {
          name = "synchronized-pnpm";
          paths = [ pkgs.pnpm ];
          buildInputs = [ pkgs.makeWrapper ];
          postBuild = ''
            wrapProgram $out/bin/pnpm --run "${pnpm-sync-npm-rev-dependencies-with-nix}/bin/sync-npm-rev-dependencies-with-nix"
          '';
        };

        devShells.synchronized-npm-rev-dependencies-with-nix = pkgs.mkShell {
          packages = [
            self'.packages.sync-npm-rev-dependencies-with-nix
            packages.npm-rev-version
          ];

          shellHook = ''
            sync-npm-rev-dependencies-with-nix
          '';
        };

        devShells.synchronized-pnpm = pkgs.mkShell {
          packages = [
            packages.synchronized-pnpm
            self'.packages.sync-npm-rev-dependencies-with-nix
            packages.npm-rev-version
          ];

          shellHook = ''
            sync-npm-rev-dependencies-with-nix --package-manager pnpm
          '';
        };

        packages.hc-scaffold-happ = let
          hcScaffold = flake.lib.wrapCustomTemplate {
            inherit pkgs system;
            customTemplatePath = ./templates/app;
          };
        in pkgs.writeShellScriptBin "hc-scaffold" ''
          if [[ "$@" == *"web-app"* ]]; then
            ${hcScaffold}/bin/hc-scaffold "$@" --setup-nix -F  
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

        packages.npm-rev-version =
          (pkgs.writeShellScriptBin "npm-rev-version" ''
            commit=$(${pkgs.git}/bin/git rev-parse HEAD)
            version=$(cat $1 | ${pkgs.jq}/bin/jq '.version' -r)
            new_version=$version-rev.$commit
            ${pkgs.nodejs_20}/bin/npm version $new_version
          '');
      };
    };
}
