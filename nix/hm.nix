{
  lib,
  ...
}:
{
  flake.homeModules.default =
    {
      config,
      pkgs,
      ...
    }:
    let
      cfg = config.home.skillsFlake;
      effectiveSkillDirs = lib.unique cfg.skillDirs;
      mkSkillEntry =
        dir: skillName: skill:
        let
          target = "${dir}/${skillName}";
          isPathLike = lib.isPath skill || (lib.isString skill && builtins.pathExists skill);
          isDir = isPathLike && (builtins.tryEval (builtins.readDir skill)).success;
        in
        if lib.isDerivation skill || isDir then
          {
            name = target;
            value.source = skill;
          }
        else if isPathLike then
          {
            name = "${target}/SKILL.md";
            value.source = skill;
          }
        else
          {
            name = "${target}/SKILL.md";
            value.source = pkgs.writeText "skills-flake-${skillName}-SKILL.md" skill;
          };

      fileEntries = lib.listToAttrs (
        lib.flatten (
          map (
            dir: lib.mapAttrsToList (skillName: skill: mkSkillEntry dir skillName skill) cfg.skills
          ) effectiveSkillDirs
        )
      );
    in
    {
      options.home.skillsFlake = {
        enable = lib.mkEnableOption "install skills-flake skills";
        skillDirs = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          example = [
            ".agents/skills"
            ".local/share/skills"
          ];
          description = "Skill destination directories relative to home.";
        };
        skills = lib.mkOption {
          type = lib.types.attrsOf (
            lib.types.oneOf [
              lib.types.package
              lib.types.path
              lib.types.str
            ]
          );
          default = { };
          description = "Skills keyed by directory name. Value can be skill dir path, SKILL.md path, inline text, or package path.";
        };
      };

      config = {
        home.skillsFlake.skillDirs = [ ".agents/skills" ];
        home.file = lib.mkIf cfg.enable fileEntries;
      };
    };
}
