# SignPath CI wiring (vapurr)

After OSS approval:

1. In SignPath: create project slug `vapurr`, link `https://github.com/0x12371C/vapurr`, attach `.signpath/artifact-configurations/default.xml`.
2. Create signing policy `release-signing` (origin verification on `master` / tags).
3. Create API token (submitter) → GitHub secret `SIGNPATH_API_TOKEN`.
4. Set GitHub Actions variables: `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG=vapurr`, `SIGNPATH_POLICY_SLUG=release-signing`.
5. Extend `signpath-release.yml` to run `pack.ps1` on `windows-latest` (or a self-hosted justin runner), upload `dist/vapurr-setup.exe`, submit signing request, then deploy signed files to thesecretlab.

Until then, strangers still need a signed binary — unsigned stays blocked by Defender Wacatac.
