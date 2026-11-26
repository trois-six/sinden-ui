# The sinden-driver binary of the release pinned in driver.lock. It is
# static (musl on Linux), so nothing is patched.
{
  lib,
  stdenvNoCC,
  fetchurl,
  lock,
}:
let
  targets = import ./targets.nix;
  inherit (stdenvNoCC.hostPlatform) system;
  target = targets.${system} or (throw "sinden-driver: no binary is published for ${system}");
  asset = lock.assets.${target} or (throw "sinden-driver: driver.lock has no asset for ${target}");
in
stdenvNoCC.mkDerivation {
  pname = "sinden-driver";
  inherit (lock) version;

  src = fetchurl {
    url = "https://github.com/trois-six/sinden-ui/releases/download/${lock.tag}/${asset.file}";
    inherit (asset) sha256;
  };

  dontUnpack = true;
  dontConfigure = true;
  dontBuild = true;
  dontPatchELF = true;
  dontStrip = true;

  installPhase = ''
    runHook preInstall
    install -Dm755 "$src" "$out/bin/sinden-driver"
    runHook postInstall
  '';

  meta = {
    description = "Driver service for the Sinden light gun";
    homepage = "https://github.com/trois-six/sinden-ui";
    license = lib.licenses.unfree;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    platforms = builtins.filter (s: lock.assets ? ${targets.${s}}) (builtins.attrNames targets);
    mainProgram = "sinden-driver";
  };
}
