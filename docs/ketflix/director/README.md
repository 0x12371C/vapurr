# Ketflix director workflow (scaffold)

Status: **unbuilt** — design + I/O contract tonight. Goal: ~10s demo trailer per title.
First dry-run title: **the-ketrix**. Do not burn the 3090 on the catalog.

## Stack
- ComfyUI on justin (RTX 3090)
- MiniMax H3 + Turbo/Acc LoRAs — see `.grok/inbox/ketflix-h3-benchmark.md`
- HF: https://huggingface.co/drbaph/MiniMax-H3-Turbo-Lora-ComfyUI

## Defaults (iteration)
- Draft: 0.5MP / 4 steps (~42s) FastVideo VSA INT8 + INT8 VAE + Turbo
- Preview: 0.5MP / 8 steps (~67–76s)
- Hero pass: 1MP / 4–8 only when draft locks

## I/O contract
| Role | Path |
|---|---|
| Poster still | `frontend/ketflix/posters/{slug}.png` |
| Catalog / logline | `frontend/ketflix/catalog.json` |
| Shot list | `docs/ketflix/director/SHOTS.md` |
| Draft workflow | `docs/ketflix/director/workflows/h3_ref2va_draft.json` |
| Trailer out | `frontend/ketflix/trailers/{slug}.mp4` |

Chrome Play (`ketflix.html`) loads `/ketflix/trailers/{slug}.mp4`. Missing file → toast, not a fake player.

## Pipeline (to implement)
1. Inputs: title slug, poster still, 1-line logline, style refs (cat consistency)
2. Shot list: 3–4 beats ≤10s total (hook → title card → one gag → end card)
3. Gen clips via H3 workflow JSON in `docs/ketflix/director/workflows/`
4. Stitch + brand sting (Ketflix red + cat) → `frontend/ketflix/trailers/{slug}.mp4`
5. Wire is already in `ketflix.html` hero **Play**

## Next build step
Import a real Comfy graph into `h3_ref2va_draft.json` matching the 0.5MP/4 FastVideo stack. The JSON in tree is a **parameter stub**, not a runnable graph. Dry-run **the-ketrix** only.
