{
  lib,
  inputs,
  ...
}:
{
  flake-file.inputs.github-actions-nix = {
    url = "github:synapdeck/github-actions-nix";
    inputs.nixpkgs.follows = "nixpkgs";
    inputs.flake-parts.follows = "flake-parts";
  };

  imports = [ inputs.github-actions-nix.flakeModules.default ];

  perSystem =
    {
      config,
      ...
    }:
    {
      # Configure files module to sync generated workflows to .github/workflows/
      files.files = lib.mapAttrsToList (name: drv: {
        path = ".github/workflows/${name}";
        inherit drv;
      }) config.githubActions.workflowFiles;

      githubActions = {
        enable = true;

        workflows.update-skills = {
          name = "Update skills";

          permissions = {
            contents = "write";
            packages = "write";
          };
          on = {
            schedule = [
              { cron = "0 0 * * *"; } # Daily at midnight
            ];
            workflowDispatch = { };
          };

          jobs.build = {
            runsOn = "ubuntu-latest";

            steps = [
              {
                name = "Checkout";
                uses = "actions/checkout@v6";
              }
              {
                name = "nothing-but-nix";
                uses = "wimpysworld/nothing-but-nix@main";
                # MAXIMUM SPACE!!!!!
                with_.hatchet-protocol = "rampage";
              }
              {
                name = "Install Lix";
                uses = "samueldr/lix-gha-installer-action@latest";
                with_.extra_nix_config = ''
                  extra-experimental-features = flakes nix-command pipe-operator
                '';
              }
              # Use oranc to cache our nix stuff, https://github.com/linyinfeng/oranc
              # See https://github.com/phanirithvij/system/blob/06960a2/.github/workflows/build.yml#L53-L66 for why we need this hack
              {
                name = "Install and run oranc";
                run = ''
                  nix build github:linyinfeng/oranc/main \
                    --extra-substituters "https://linyinfeng.cachix.org" \
                    --extra-trusted-public-keys "linyinfeng.cachix.org-1:sPYQXcNrnCf7Vr7T0YmjXz5dMZ7aOKG3EqLja0xr9MM="
                  ./result/bin/oranc server --listen "127.0.0.1:9999" --repository-parts 3 &
                  rm result
                '';
              }
              {
                name = "Finish oranc setup";
                uses = "linyinfeng/oranc-action@main";
                with_ = {
                  repositoryPart1 = "\${{ github.repository_owner }}";
                  repositoryPart2 = "skills-flake/oranc";
                  orancServerType = "url";
                  orancServerUrl = "http://127.0.0.1:9999";
                  initialize = true;
                  username = "\${{ github.repository_owner }}";
                  password = "\${{ github.token }}";
                  signingKey = "\${{ secrets.ORANC_SIGNING_KEY }}";
                };
              }
              {
                name = "Run updater";
                run = ''
                  nix run ".#updater" -- update
                '';
              }
              {
                name = "Commit updates";
                run = ''
                  git config user.name "github-actions[bot]"
                  git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

                  git add -A

                  if git diff --cached --quiet; then
                    echo "No updates to commit."
                  else
                    git commit -m "chore: update skills"
                    git push
                  fi
                '';
              }
            ];
          };
        };
      };
    };
}
