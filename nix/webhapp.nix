# Build a hApp
{ name, happ, ui, zip, writeText, runCommandNoCC, holochain, runCommandLocal
, meta }:

let
  uiZip = runCommandNoCC "ui-zip" { } ''
    mkdir $out
    mkdir $out/share
    cd ${ui}
    ${zip}/bin/zip -r $out/share/ui.zip .
  '';
  manifestYaml = writeText "webhapp.yaml" ''
    ---
    manifest_version: "1"
    name: ${name}
    ui:
      bundled: ./ui.zip
    happ_manifest:
      bundled: ./happ.happ
  '';

  debug = runCommandLocal "${name}-debug" { srcs = [ happ.meta.debug ui ]; } ''
      mkdir workdir

      cp ${uiZip}/share/ui.zip workdir
      cp ${happ.meta.debug} workdir/happ.happ
      cp ${manifestYaml} workdir/web-happ.yaml

    	${holochain}/bin/hc web-app pack workdir
    	mv workdir/${name}.webhapp $out
  '';

in runCommandNoCC "${name}-webhapp" {
  meta = meta // { inherit debug; };
  srcs = [ ui happ ];
  outputs = [ "out" ];
} ''
    mkdir workdir

    cp ${uiZip}/share/ui.zip workdir
    cp ${happ} workdir/happ.happ
    cp ${manifestYaml} workdir/web-happ.yaml

  	${holochain}/bin/hc web-app pack workdir
  	mv workdir/${name}.webhapp $out
''
