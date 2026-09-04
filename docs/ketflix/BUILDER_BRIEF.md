# Ketflix builder brief (Relic via vapurrbot) — overnight

You are the dedicated **Ketflix** builder for vapurr (on-chain browser). Repo: C:\Users\jfren\vapurr. Follow AGENTS.md, docs/V1.md, docs/STATUS.md. Read .grok/rules/404-is-not-payments.md (404 = load-fail page only; NOT payments).

## Mission
1. Open `frontend/ketflix.png` (title wall) and inventory every movie/show tile individually (name + grid position). Write `docs/ketflix/TITLES.md`.
2. Produce a Netflix-like catalog plan: per-title high-res poster art, backdrop, logo treatment, metadata stub. Assets under `frontend/ketflix/` (create). Do not replace ketflix.png until per-title assets exist.
3. Evolve `frontend/ketflix.html` toward a Netflix-style browse surface (rows, hero, title detail) using vapurr tokens (lime/void/cat — or Ketflix red accent only if already in ketflix chrome; prefer tokens.css consistency unless DESIGN says otherwise). Keep it chrome at vapurr://ketflix.
4. Design and scaffold the **Ketflix director** workflow for short **~10s** demo trailers (ComfyUI / MiniMax H3 on local RTX 5090). Workflow does NOT exist yet — build it. Benchmark notes: `.grok/inbox/ketflix-h3-benchmark.md` and HF https://huggingface.co/drbaph/MiniMax-H3-Turbo-Lora-ComfyUI. Practical default for iteration: 0.5MP / 4 steps ~42s; better preview 0.5MP / 8 steps.
5. Generate at least one title's higher-res poster pass and document the pipeline so overnight can continue. Trailers can start after director scaffold exists.

## Constraints
- Do not pack.exe wars; leave pack.ps1 to House.
- Do not invent RPC/secrets. No live Rain/zer0ID.
- Prefer target/ for cargo if you touch shell; Ketflix is mostly frontend + Comfy assets.
- Commit nothing unless Relic asked (he did not overnight).
- Update docs/OVERNIGHT.md and docs/ketflix/ as you go.
- Title your session work clearly as Ketflix.

## First hour success
TITLES.md inventory from ketflix.png + folder scaffold + director workflow design doc started + one poster pipeline path proven or blocked with exact next step.
## Token budget
Prove the pipeline; do not burn the 5090 on full catalogs overnight. Inventory all titles, high-res for a few heroes, scaffold director, one 10s test if path works. Short status in docs/OVERNIGHT.md.

