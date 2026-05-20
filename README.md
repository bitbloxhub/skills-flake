# skills-flake

Nix flake that packages AI agent skills from many upstream repos and exposes them as installable paths.

## What you get

- `packages.<system>.skills`: nested attrset of fetched skills
- `packages.<system>.updater`: CLI that updates `skills-flake.lock.json` from `skill-list.kdl`
- `homeModules.default`: Home Manager module that installs selected skills into agent-specific skill directories

## Usage

### 1) Consume skills directly from flake output

Skill paths are exposed under:

`inputs.skills-flake.packages.${pkgs.stdenv.hostPlatform.system}.skills.<source>.<url-section>.<skill>`

`<url-section>` depends on source parser (for GitHub, it is usually `<owner>.<repo>`).

Example:
`inputs.skills-flake.packages.${pkgs.stdenv.hostPlatform.system}.skills.github.NousResearch.hermes-agent.apple.apple-notes`

### 2) Install skills with Home Manager

Add this flake to your inputs, then import its module and enable it:

```nix
{
  imports = [ inputs.skills-flake.homeModules.default ];

  home.skillsFlake = {
    enable = true;

    # Optional: explicit directories (relative to $HOME)
    # skillDirs = [ ".agents/skills" ];

    # Auto-install into enabled agent dirs via `agents.<name>.enable = true`
    agents.pi.enable = true;

    # Skills keyed by output directory name
    skills = {
      agent-browser = inputs.skills-flake.packages.${pkgs.stdenv.hostPlatform.system}.skills.github.vercel-labs.agent-browser.agent-browser;
    };
  };
}
```

Supported agents:

<details>
<summary>Full supported agent list, install directories, and websites</summary>

- [aider-desk](https://aiderdesk.hotovo.com/): `.aider-desk/skills`
- [amp](https://ampcode.com/): `${config.xdg.configHome}/agents/skills`
- [antigravity](https://github.com/google-antigravity/antigravity-cli): `.gemini/antigravity/skills`
- [augment](https://www.augmentcode.com/): `.augment/skills`
- [bob](https://bob.ibm.com/): `.bob/skills`
- [claude-code](https://claude.com/product/claude-code): `<claude configDir>/skills`
- [openclaw](https://openclaw.ai/): `.openclaw/skills`
- [cline](https://cline.bot/): `.agents/skills`
- [codearts-agent](https://codearts.huaweicloud.com/): `.codeartsdoer/skills`
- [codebuddy](https://www.codebuddy.ai/): `.codebuddy/skills`
- [codemaker](https://marketplace.visualstudio.com/items?itemName=codemakerai.codemakerai): `.codemaker/skills`
- [codestudio](https://www.mycodestudio.ai/): `.codestudio/skills`
- [codex](https://openai.com/codex/): `${config.xdg.configHome}/codex/skills` or `.codex/skills` (version/config-dependent)
- [command-code](https://commandcode.ai/): `.commandcode/skills`
- [continue](https://www.continue.dev/): `.continue/skills`
- [cortex](https://www.snowflake.com/en/product/features/cortex-code/): `.snowflake/cortex/skills`
- [crush](https://github.com/charmbracelet/crush): `${config.xdg.configHome}/crush/skills`
- [cursor](https://cursor.com/): `.cursor/skills`
- [deepagents](https://docs.langchain.com/oss/python/deepagents/overview): `.deepagents/agent/skills`
- [devin](https://devin.ai/): `${config.xdg.configHome}/devin/skills`
- [dexto](https://docs.dexto.ai/docs/getting-started/intro): `.agents/skills`
- [droid](https://factory.ai/): `.factory/skills`
- [firebender](https://firebender.com/): `.firebender/skills`
- [forgecode](https://forgecode.dev/): `.forge/skills`
- [gemini-cli](https://github.com/google-gemini/gemini-cli): `.gemini/skills`
- [github-copilot](https://github.com/features/copilot): `<copilot configDir>/skills`
- [goose](https://goose-docs.ai/): `${config.xdg.configHome}/goose/skills`
- [hermes-agent](https://hermes-agent.org/): `.hermes/skills`
- [junie](https://www.jetbrains.com/junie/): `.junie/skills`
- [iflow-cli](https://github.com/iflow-ai/iflow-cli): `.iflow/skills`
- [kilo](https://kilo.ai/): `.kilocode/skills`
- [kimi-cli](https://www.kimi.com/code): `${config.xdg.configHome}/agents/skills`
- [kiro-cli](https://kiro.dev/): `.kiro/skills`
- [kode](https://kode.ai/): `.kode/skills`
- [mcpjam](https://www.mcpjam.com/): `.mcpjam/skills`
- [mistral-vibe](https://mistral.ai/products/vibe): `.vibe/skills`
- [mux](https://github.com/coder/mux): `.mux/skills`
- [opencode](https://opencode.ai/): `${config.xdg.configHome}/opencode/skills`
- [openhands](https://www.openhands.dev/): `.openhands/skills`
- [pi](https://pi.dev/): `.pi/agent/skills`
- [qoder](https://qoder.com/): `.qoder/skills`
- [qwen-code](https://qwenlm.github.io/qwen-code-docs/en/): `.qwen/skills`
- [replit](https://replit.com/): `${config.xdg.configHome}/agents/skills`
- [rovodev](https://www.atlassian.com/software/rovo-dev): `.rovodev/skills`
- [roo](https://roocodeinc.github.io/Roo-Code/): `.roo/skills`
- [tabnine-cli](https://www.tabnine.com/): `.tabnine/agent/skills`
- [trae](https://www.trae.ai/): `.trae/skills`
- [trae-cn](https://www.trae.cn/): `.trae-cn/skills`
- [warp](https://www.warp.dev/): `.warp/skills`
- [windsurf](https://windsurf.com/): `.codeium/windsurf/skills`
- [zencoder](https://zencoder.ai/): `.zencoder/skills`
- [neovate](https://neovateai.dev/): `.neovate/skills`
- [pochi](https://github.com/TabbyML/pochi): `.pochi/skills`
- [adal](https://docs.sylph.ai/): `.adal/skills`
- [universal](https://github.com/vercel-labs/skills): `.agents/skills`

</details>

## Updating skill sources

Edit `skill-list.kdl`, then run:

```bash
nix run .#updater -- sort-skill-list
nix run .#updater -- update
```

This updates `skills-flake.lock.json` with pinned revisions and hashes.

## Development

```bash
direnv allow
```

Useful commands:

```bash
nix run .#updater -- --help
```
