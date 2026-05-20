{
  inputs,
  ...
}:
{
  flake-file.inputs = {
    make-shell = {
      url = "github:nicknovitski/make-shell";
      inputs.flake-compat.follows = "";
    };
  };

  imports = [
    inputs.make-shell.flakeModules.default
  ];

  perSystem = {
    make-shells.default.name = "your-project-name-here";
  };
}
