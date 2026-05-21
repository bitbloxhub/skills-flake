{
  lib,
  inputs,
  ...
}:
{
  flake-file.inputs.files = {
    url = "github:mightyiam/files";
    # https://github.com/mightyiam/files/commit/6283e9b
    flake = false;
  };

  imports = [ (inputs.files + "/flake-module.nix") ];

  perSystem =
    {
      config,
      ...
    }:
    {
      # Expose the files writer as an app
      apps.write-files = {
        type = "app";
        program = lib.getExe config.files.writer.drv;
        meta.description = "Write generated files to the repository";
      };
    };
}
