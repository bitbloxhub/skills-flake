{
  flake-file.inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crate2nix = {
      url = "github:nix-community/crate2nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-compat.follows = "";
      inputs.flake-parts.follows = "flake-parts";
      inputs.cachix.follows = "";
    };
  };

  perSystem =
    {
      pkgs,
      inputs',
      ...
    }:
    let
      updaterCargoNix = import ../updater/Cargo.nix;

      updaterCargoWorkspace = pkgs.callPackage updaterCargoNix {
        buildRustCrateForPkgs =
          pkgs:
          with pkgs;
          buildRustCrate.override {
            rustc = inputs'.fenix.packages.default.toolchain;
            cargo = inputs'.fenix.packages.default.toolchain;
          };
      };
    in
    {
      make-shells.default = {
        packages = [
          pkgs.nix-prefetch-git
          inputs'.fenix.packages.default.toolchain
          pkgs.rust-analyzer
          pkgs.cargo-expand
          inputs'.crate2nix.packages.default
        ];
      };

      packages.updater = updaterCargoWorkspace.rootCrate.build;

      treefmt = {
        programs.rustfmt = {
          enable = true;
          package = inputs'.fenix.packages.default.rustfmt;
        };
        settings.global.excludes = [
          "**/Cargo.nix"
          "**/skills-flake.lock.json"
        ];
      };
    };
}
