# Build version progression

Track every pack/channel/exe change overnight.

| When (ET) | version | build/rev | artifact | notes |
|-----------|---------|-----------|----------|-------|
| 2026-09-04 01:51 ET | 1.1.0 | rev=223a962 sha=5c305fc359b4 | dist vapurr-1.1.0-windows-x64.zip (15.3 MB) + Install vapurr.exe (23.1 MB) | Cargo workspace bumped 0.1.0?1.1.0; release pack OK |
| 2026-09-04 01:25 ET | 0.1.0 | build=1788496196 sha=4bd126e36141â€¦ | channel + dist vapurr.next.exe (20.5 MB) | baseline at watch start; last pack ~12:29 AM |
| 2026-09-04 10:28 ET | 1.1.0 | rev=223a962 sha=947c585371b6 | dist vapurr-1.1.0-windows-x64.zip (50.6 MB) + Install vapurr.exe (58.7 MB) | House re-pack; zip/exe grew (trailers in pack tree); same rev |

| 2026-09-04 11:03 ET | 1.1.0 | rev=223a962 sha=75b0a6e26e9a | dist/vapurr/vapurr.exe (61.0 MB) | silent rebuild; no new zip - next House pack re-logs |
| 2026-09-04 11:15 ET | 1.1.0 | rev=223a962 sha=e7a381d13559 | dist/vapurr/vapurr.next.exe (61.1 MB) | next.exe drift after 11:03; still no zip |
| 2026-09-04 13:53 ET | 1.1.0 | rev=223a962 sha=2fe78275be53 | dist/vapurr/Install vapurr.exe (66.7 MB) | Install.exe refreshed; zip still 10:28 — next House pack re-logs |
