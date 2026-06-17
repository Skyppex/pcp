{
  pkgs,
  config,
  ...
}: {
  languages.rust = {
    enable = true;
  };

  enterTest = ''nix flake check'';
}
