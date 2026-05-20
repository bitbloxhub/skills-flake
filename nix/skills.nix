{
  self,
  ...
}:
{
  flake.lib.mkSkills =
    {
      pkgs,
      lockFile,
    }:
    let
      lock = builtins.fromJSON (builtins.readFile lockFile);

      mkSkill =
        skillName: entry:
        let
          inherit (entry) source;
          skillPath = "${entry.root_dir}";
        in
        pkgs.fetchgit {
          name = "skill-${skillName}";
          inherit (source) url;
          inherit (source) rev;
          inherit (entry) hash;
          sparseCheckout = [ skillPath ];
          rootDir = skillPath;
        };

      mkTree =
        node:
        builtins.mapAttrs (
          name: value:
          if builtins.isAttrs value && value ? hash && value ? source then
            mkSkill name value
          else
            mkTree value
        ) node;
    in
    mkTree lock.source;

  perSystem =
    {
      pkgs,
      ...
    }:
    let
      skillsTree = self.lib.mkSkills {
        inherit pkgs;
        lockFile = ../skills-flake.lock.json;
      };
    in
    {
      packages.skills =
        pkgs.runCommand "skills" { } ''
          mkdir -p "$out"
        ''
        // skillsTree;
    };
}
