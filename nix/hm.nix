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

      codexPackage = lib.attrByPath [ "programs" "codex" "package" ] null config;
      codexPackageVersion = if codexPackage != null then lib.getVersion codexPackage else "0.94.0";
      codexIsTomlConfig = lib.versionAtLeast codexPackageVersion "0.2.0";
      codexUseXdgDir = config.home.preferXdgDirectories && codexIsTomlConfig;
      codexSkillDir = if codexUseXdgDir then "${config.xdg.configHome}/codex/skills" else ".codex/skills";

      agentNames = [
        "aider-desk"
        "amp"
        "antigravity"
        "augment"
        "bob"
        "claude-code"
        "openclaw"
        "cline"
        "codearts-agent"
        "codebuddy"
        "codemaker"
        "codestudio"
        "codex"
        "command-code"
        "continue"
        "cortex"
        "crush"
        "cursor"
        "deepagents"
        "devin"
        "dexto"
        "droid"
        "firebender"
        "forgecode"
        "gemini-cli"
        "github-copilot"
        "goose"
        "hermes-agent"
        "junie"
        "iflow-cli"
        "kilo"
        "kimi-cli"
        "kiro-cli"
        "kode"
        "mcpjam"
        "mistral-vibe"
        "mux"
        "opencode"
        "openhands"
        "pi"
        "qoder"
        "qwen-code"
        "replit"
        "rovodev"
        "roo"
        "tabnine-cli"
        "trae"
        "trae-cn"
        "warp"
        "windsurf"
        "zencoder"
        "neovate"
        "pochi"
        "adal"
        "universal"
      ];

      # Source for agent skill directory conventions:
      # https://github.com/vercel-labs/skills/blob/c5ad3a8/src/agents.ts
      #
      # HM-specific overrides where available:
      # - codex: https://github.com/nix-community/home-manager/blob/bd868f7/modules/programs/codex.nix
      # - claude-code: https://github.com/nix-community/home-manager/blob/bd868f7/modules/programs/claude-code.nix
      # - github-copilot-cli: https://github.com/nix-community/home-manager/blob/bd868f7/modules/programs/github-copilot-cli.nix
      agentSkillDirs = {
        aider-desk = ".aider-desk/skills";
        amp = "${config.xdg.configHome}/agents/skills";
        antigravity = ".gemini/antigravity/skills";
        augment = ".augment/skills";
        bob = ".bob/skills";
        claude-code = "${
          lib.attrByPath [
            "programs"
            "claude-code"
            "configDir"
          ] ".claude" config
        }/skills";
        openclaw = ".openclaw/skills";
        cline = ".agents/skills";
        codearts-agent = ".codeartsdoer/skills";
        codebuddy = ".codebuddy/skills";
        codemaker = ".codemaker/skills";
        codestudio = ".codestudio/skills";
        codex = codexSkillDir;
        command-code = ".commandcode/skills";
        continue = ".continue/skills";
        cortex = ".snowflake/cortex/skills";
        crush = "${config.xdg.configHome}/crush/skills";
        cursor = ".cursor/skills";
        deepagents = ".deepagents/agent/skills";
        devin = "${config.xdg.configHome}/devin/skills";
        dexto = ".agents/skills";
        droid = ".factory/skills";
        firebender = ".firebender/skills";
        forgecode = ".forge/skills";
        gemini-cli = ".gemini/skills";
        github-copilot = "${
          lib.attrByPath [
            "programs"
            "github-copilot-cli"
            "configDir"
          ] ".copilot" config
        }/skills";
        goose = "${config.xdg.configHome}/goose/skills";
        hermes-agent = ".hermes/skills";
        junie = ".junie/skills";
        iflow-cli = ".iflow/skills";
        kilo = ".kilocode/skills";
        kimi-cli = "${config.xdg.configHome}/agents/skills";
        kiro-cli = ".kiro/skills";
        kode = ".kode/skills";
        mcpjam = ".mcpjam/skills";
        mistral-vibe = ".vibe/skills";
        mux = ".mux/skills";
        opencode = "${config.xdg.configHome}/opencode/skills";
        openhands = ".openhands/skills";
        pi = ".pi/agent/skills";
        qoder = ".qoder/skills";
        qwen-code = ".qwen/skills";
        replit = "${config.xdg.configHome}/agents/skills";
        rovodev = ".rovodev/skills";
        roo = ".roo/skills";
        tabnine-cli = ".tabnine/agent/skills";
        trae = ".trae/skills";
        trae-cn = ".trae-cn/skills";
        warp = ".warp/skills";
        windsurf = ".codeium/windsurf/skills";
        zencoder = ".zencoder/skills";
        neovate = ".neovate/skills";
        pochi = ".pochi/skills";
        adal = ".adal/skills";
        universal = ".agents/skills";
      };

      progEnabled = name: lib.attrByPath [ "programs" name "enable" ] false config;

      agentEnableDefaults = {
        codex = progEnabled "codex";
        claude-code = progEnabled "claude-code";
        amp = progEnabled "amp";
        antigravity = progEnabled "antigravity";
        cursor = progEnabled "cursor";
        gemini-cli = progEnabled "gemini-cli";
        mistral-vibe = progEnabled "mistral-vibe";
        opencode = progEnabled "opencode";
        windsurf = progEnabled "windsurf";
        github-copilot = progEnabled "github-copilot-cli";
        kiro-cli = progEnabled "kiro";
        universal = true;
      };

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

      enabledAgentSkillDirs = lib.concatMap (
        name: lib.optionals (lib.attrByPath [ name "enable" ] false cfg.agents) [ agentSkillDirs.${name} ]
      ) agentNames;
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
        agents = lib.genAttrs agentNames (name: {
          enable = lib.mkEnableOption "install skills into ${name}" // {
            default = lib.attrByPath [ name ] false agentEnableDefaults;
          };
        });
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
        home.skillsFlake.skillDirs = enabledAgentSkillDirs;
        home.file = lib.mkIf cfg.enable fileEntries;
      };
    };
}
