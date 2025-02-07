# Build a hApp
{ name, happ, ui, runCommandNoCC, holochain, runCommandLocal, meta }:

let
  manifestYaml = ''
    ---
    manifest_version: "1"
    name: ${name}
    ui:
      bundled: ${ui}
    happ_manifest:
      bundled: ${happ}
  '';
  manifestYamlDebug = ''
    ---
    manifest_version: "1"
    name: ${name}
    ui:
      bundled: ${ui}
    happ_manifest:
      bundled: ${happ.meta.debug}
  '';

  debug = runCommandLocal "${name}-debug" { srcs = [ happ.meta.debug ui ]; } ''
      mkdir workdir
        
      cp ${manifestYamlDebug} workdir/web-happ.yaml

    	${holochain}/bin/hc web-app pack workdir
    	mv workdir/${name}.webhapp $out
  '';

in runCommandNoCC "${name}-webhapp" {
  meta = meta // { inherit debug; };
  srcs = [ ui happ ];
  outputs = [ "out" ];
} ''
    mkdir workdir
      
    cp ${manifestYaml} workdir/web-happ.yaml

  	${holochain}/bin/hc web-app pack workdir
  	mv workdir/${name}.webhapp $out
''
