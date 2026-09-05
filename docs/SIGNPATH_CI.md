# SignPath CI wiring (vapurr)

Apply submitted 2026-09-04 (agent@thesecretlab.app). After approval:

1. SignPath: project `vapurr`, policy `release-signing`, link GitHub repo + `.signpath/artifact-configurations/default.xml`
2. GitHub secret `SIGNPATH_API_TOKEN`
3. GitHub vars: `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG=vapurr`, `SIGNPATH_POLICY_SLUG=release-signing`
4. Run Actions → `signpath-release` (workflow_dispatch). Packs on `windows-latest`, submits PE to SignPath, uploads signed artifact.
5. Copy signed `vapurr-setup.exe` to thesecretlab `public/vapurr/` and ship.

Download page already names SignPath Foundation (required by SignPath).
