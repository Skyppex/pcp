{pkgs}: {
  fix = {
    type = "app";
    program = "${pkgs.writeShellApplication {
      name = "cargo-fix";
      runtimeInputs = [pkgs.cargo pkgs.clippy];
      text = ''
        cargo clippy --fix --allow-dirty --allow-staged --all-targets
      '';
    }}/bin/cargo-fix";
    meta = {
      description = "apply linter suggestions";
    };
  };
}
