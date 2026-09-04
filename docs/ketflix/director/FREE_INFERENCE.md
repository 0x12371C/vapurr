# Ketflix free video inference (honest shortlist)

Relic ask: infinite ket slop machine, **reputable free only**, routed through director.

## Reality check
There is **no** reputable vendor offering unlimited free video API at trailer quality.
What exists:

| Source | Free? | Reputable? | Fit for infinite slop |
|---|---|---|---|
| **Local RTX 3090 + Comfy H3 turbo** (`generate_fast.py`) | Yes (owned GPU) | Yes | **Primary.** Only true infinite. ~40–90s/clip @ 0.5MP/4 |
| **HF ZeroGPU Spaces** (Gradio client) | Yes — daily quota | Yes (Hugging Face) | **Secondary overflow.** Free acct **5 min/day**, PRO **40 min/day**. Queues. |
| HF Inference Providers credits | $0.10/mo free | Yes | Useless for video volume |
| WaveSpeed / deAPI / Replicate signup credits | $1–$5 once | Mixed–yes | Burn-once trials, not infinite |
| ofox / aiapi-pro / random "free CogVideo" gates | Claims $0 | **No** (reseller / China-domestic proxy blogs) | **Do not use** |

## Chosen routing
Director backend order (`BACKENDS` env or `--backend`):
1. `local` — Comfy `generate_fast.py` I2V from poster (default)
2. `zerogpu` — HF Space via `gradio_client` (needs `HF_TOKEN` in user env)
3. `auto` — local if Comfy `:8188` up, else ZeroGPU

## ZeroGPU Spaces (curated, I2V-friendly where possible)
Pin stable Space IDs in `ketflix_free_backend.py` — swap if a Space dies:
- `zerogpu-aoti/wan2-2-fp8da-aoti` — Wan 2.2 T2V lightning (text; use when no poster)
- Prefer I2V Spaces when available; poster path is the Ketflix contract

Quota math (free HF): ~5 min GPU/day ≈ a few short clips. PRO ≈ a short overnight trickle. **Not** a replacement for the 3090.

## Setup
1. Keep ComfyUI on justin running for local.
2. For ZeroGPU: create free HF account, put token in **user** env `HF_TOKEN` (never paste into chat). Optional: HF PRO for 40 min/day.
3. `pip install gradio_client` into Comfy venv if missing.
4. Run: `python run_ketflix_slop_machine.py --backend auto`

## Out of scope
- Paying fal / Replicate / Luma / Kling for volume (unless Relic funds)
- Spawning new Grok CLI workers
