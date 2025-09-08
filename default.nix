{
  src,
  naersk,
  pkgConfig,
  release ? false,
}:
naersk.buildPackage {
  name = "pcp";
  inherit src;
  nativeBuildInputs = [pkgConfig];
  doCheck = false;

  cargoBuildFlags = (
    if release
    then ["--release"]
    else []
  );
}
