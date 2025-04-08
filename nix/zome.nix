{ lib, workspacePath, cargoArtifacts, runCommandLocal, runCommandNoCC, binaryen
, deterministicCraneLib, craneLib, crateCargoToml, matchingZomeHash ? null
, zome-wasm-hash, meta, zomeEnvironmentVars ? { }, excludedCrates ? [ ] }:

let
  cargoToml = builtins.fromTOML (builtins.readFile crateCargoToml);
  crate = cargoToml.package.name;

  src = craneLib.cleanCargoSource (craneLib.path workspacePath);

  listCratesPathsFromWorkspace = src:
    let
      allFiles = lib.filesystem.listFilesRecursive src;
      allCargoTomlsPaths =
        builtins.filter (path: lib.strings.hasSuffix "/Cargo.toml" path)
        allFiles;
      cargoTomlsPathsWithoutWorkspace = builtins.filter (path:
        builtins.hasAttr "package" (builtins.fromTOML (builtins.readFile path)))
        allCargoTomlsPaths;
      cratesPathsWithoutWorkspace = builtins.map (path: builtins.dirOf path)
        cargoTomlsPathsWithoutWorkspace;
    in cratesPathsWithoutWorkspace;

  listCratesNamesFromWorskspace = src:
    let
      allCratesPaths = listCratesPathsFromWorkspace src;
      cratesCargoToml = builtins.map
        (path: builtins.fromTOML (builtins.readFile (path + "/Cargo.toml")))
        allCratesPaths;
      cratesWithoutWorkspace =
        builtins.filter (toml: builtins.hasAttr "package" toml) cratesCargoToml;
      cratesNames =
        builtins.map (toml: toml.package.name) cratesWithoutWorkspace;
    in cratesNames;

  isCrateZome = path:
    let
      hasSrc =
        lib.filesystem.pathIsDirectory (builtins.toString (path + "/src"));
      hasMain = hasSrc
        && (builtins.pathExists (builtins.toString (path + "/src/main.rs")));
      hasBinDir = hasSrc && (lib.filesystem.pathIsDirectory
        (builtins.toString (path + "/src/bin")));
    in hasSrc && !hasMain && !hasBinDir;

  listBinaryCratesPathsFromWorkspace = src:
    let
      allCratesPaths = listCratesPathsFromWorkspace src;
      binaryCratesPaths =
        builtins.filter (cratePath: !(isCrateZome cratePath)) allCratesPaths;
    in binaryCratesPaths;

  listBinaryCratesFromWorkspace = src:
    let
      binaryCratesPaths = listBinaryCratesPathsFromWorkspace src;
      binaryCratesCargoToml = builtins.map
        (path: builtins.fromTOML (builtins.readFile (path + "/Cargo.toml")))
        binaryCratesPaths;
      binaryCrates =
        builtins.map (toml: toml.package.name) binaryCratesCargoToml;
    in binaryCrates;

  nonWasmCrates = listBinaryCratesFromWorkspace src;
  excludedCratesArgument = builtins.toString
    (builtins.map (c: " --exclude ${c}") (nonWasmCrates ++ excludedCrates));

  allCratesNames = listCratesNamesFromWorskspace src;

  workspaceName = if builtins.length allCratesNames > 0 then
    builtins.elemAt allCratesNames 0
  else
    "";

  cleanBinaryCrates = { lib }:
    src:
    let
      binaryCratesPaths = listBinaryCratesPathsFromWorkspace src;
      isInsideBinCrate = path:
        builtins.any (binaryCratePath:
          (lib.strings.hasPrefix "${
              (builtins.substring 51 (builtins.stringLength binaryCratePath)
                binaryCratePath)
            }/" (builtins.substring 51 (builtins.stringLength path) path)))
        binaryCratesPaths;

    in lib.cleanSourceWith {
      inherit src;
      filter = orig_path: type:
        (lib.strings.hasSuffix "Cargo.toml" orig_path)
        || (lib.strings.hasSuffix "lib.rs" orig_path)
        || (lib.strings.hasSuffix "main.rs" orig_path)
        || !(isInsideBinCrate orig_path);

      name = "clean-binary-crates";
    };

  commonArgs = {
    src = if (builtins.length nonWasmCrates) > 0 then
      (cleanBinaryCrates { inherit lib; } src)
    else
      src;
    doCheck = false;
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
    pname = "${workspaceName}-workspace";
    version = cargoToml.package.version;
    cargoBuildCommand =
      "cargo build --release --locked --workspace ${excludedCratesArgument}";
    cargoCheckCommand = "";
    cargoExtraArgs = "";
  };

  buildPackageCommonArgs = commonArgs // zomeEnvironmentVars // {
    pname = crate;
    version = cargoToml.package.version;
    cargoToml = crateCargoToml;
  };

  zomeCargoArtifacts =
    (craneLib.callPackage ./buildDepsOnlyWithArtifacts.nix { })
    (commonArgs // { inherit cargoArtifacts; });

  wasm = craneLib.buildPackage (buildPackageCommonArgs // {
    cargoArtifacts = zomeCargoArtifacts;
    pname = "${crate}-debug";
  });

  deterministicWasm = let
    cargoArtifacts = deterministicCraneLib.buildDepsOnly (commonArgs // { });

    wasm = deterministicCraneLib.buildPackage
      (buildPackageCommonArgs // { inherit cargoArtifacts; });
  in runCommandLocal "${crate}-deterministic" { }
  "	cp ${wasm}/lib/${crate}.wasm $out \n";

  debug = runCommandLocal "${crate}-debug" { } ''
    cp ${wasm}/lib/${crate}.wasm $out 
  '';

  release = runCommandNoCC crate { buildInputs = [ binaryen ]; } ''
    wasm-opt --strip-debug -Oz -o $out ${deterministicWasm}
  '';

  guardedRelease = if matchingZomeHash != null then
    runCommandNoCC "check-zome-${crate}-hash" {
      srcs = [ release matchingZomeHash ];
      buildInputs = [ zome-wasm-hash ];
    } ''
      ORIGINAL_HASH=$(zome-wasm-hash ${matchingZomeHash})
      NEW_HASH=$(zome-wasm-hash ${release})

      if [[ "$ORIGINAL_HASH" != "$NEW_HASH" ]]; then
        echo "The hash for the new ${crate} zome does not match the hash of the original zome"
        exit 1
      fi

      cp ${release} $out
    ''
  else
    release;

in runCommandNoCC crate {
  meta = meta // { inherit debug; };
  outputs = [ "out" "hash" ];
  buildInputs = [ zome-wasm-hash ];
} ''
  cp ${guardedRelease} $out
  zome-wasm-hash $out > $hash
''
