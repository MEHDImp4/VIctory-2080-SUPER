# VIctory on an RX 7900 XTX

A fork of [VIctory](https://github.com/Marvisak/VIctory), a compatibility layer
that runs an Xbox Series title natively on Windows from files the user supplies.
This one is about **speed on AMD RDNA 3**: the world at 30 frames a second on
one PC, without giving up the picture. It is not there yet.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Where it stands (2026-11-25)

Tested on one machine only: Windows 11, Intel i9-12900K, 64 GB RAM, Radeon
RX 7900 XTX, 1440p, the Series X profile.

| | |
|---|---|
| Menus, videos, loading screens | 30 fps |
| Prologue, indoors | 50-75 ms a frame (13-20 fps) |
| First mission with the boat | 46-53 ms (19-22 fps) |
| Street chase with many cars and people | 100-165 ms (6-10 fps), single frames far longer |
| Outdoors on foot, the camp | 60-90 ms |
| Flying | 42-57 ms |
| Longest session so far | 41 minutes, no crash |
| Memory | 40-46 GB of private memory: **64 GB of RAM recommended** |

It started at 280 ms a frame in the same scenes two days earlier.

What is known to be wrong:

- Trees, plants and far buildings flicker outdoors in some places.
- Rooms with little direct light are too dark without `-RayTracing`.
- Textures of what just came into view appear late while the title streams.
- The first run with a new driver or a new build stutters while the driver makes
  pipelines (`victory warm` makes them ahead).
- Recording a frame in the world makes the title's own watchdog end it.

## How to run it

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone https://github.com/avatarmanz/VIctory-7900XTX
cd VIctory-7900XTX
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play.ps1 -Exe <path of your copy's executable>
```

`scripts\play.ps1 -Safe` leaves out the newest, faster ways; try it first when
something is wrong. `-RayTracing` turns the title's ray-traced indirect light on.

### RTX 2080 SUPER profile

```powershell
scripts\play.ps1 -Profile Rtx2080Super -Exe <path of your copy's executable>
```

This experimental profile stops adding new jobs to the speculative texture
upload queue when local VRAM usage reaches 85% of the budget reported by
Windows or when the budget cannot be read. Existing jobs finish and new uploads
use the regular command lists. The threshold is a starting point for testing
on an 8 GiB RTX 2080 SUPER. The profile reports a 1080p display mode to the
title, which can still choose its own render resolution. It releases idle
staging and shadow buffers sooner when Windows is short of commit. The
texture pool limits empty reserve blocks using the remaining DXGI budget
and trims empty blocks even when the loader stops requesting new space.
The profile limits waits for regular pipeline compilation to 120 ms per frame. Pending
draws may be omitted temporarily, so prewarming is recommended. Screen
passes keep their separate wait budget.

The indexed texture queue prioritizes retries, then alternates recent and
old writes. Each frame it counts the backlog in the same pass as it chooses
the uploads and sorts only the entries it will visit. Stencil copies share
one temporary buffer per power-of-two size on the direct queue, with the
copy layouts cached at creation time. These changes adapt the
[texture queue work in HrHd/VIctory](https://github.com/HrHd/victory-fork/commit/f24b62b7db87b47b97502371a1a3398a39c58e00)
and [stencil buffer reuse from upstream](https://github.com/Marvisak/VIctory/commit/9a12a45ed98e000ebd03245538678720dbd022a9).

On the RTX 2080 SUPER test system the title still requested a 3840×2160
swap chain, even with 1080p in a copy of the saved video settings. Running
with `--console series-s` requested 1920×1080, and the fork's presenter
showed a black screen. The profile uses Series X. See the upstream
[Series S presentation work](https://github.com/Marvisak/VIctory/pull/1997).

For comparison, run the same checkpoint and camera path with `-Profile Default`
and `-Profile Rtx2080Super`, with ray tracing off and a warm shader cache for
both. Compare frame times, visible texture pop-in, and local video memory usage.
Use `-More @{VICTORY_UPLOAD_AHEAD_VRAM_PERCENT='80'}` to test another
threshold. A value of `0` disables the guard. With `-Safe`, speculative uploads
are already disabled.
Use `-More @{VICTORY_POOL_HEADROOM='0'}` to compare the previous pool policy.

### Prepare the shader and pipeline caches

```powershell
scripts\play.ps1 -Profile Rtx2080Super -Exe <path of your copy's executable> -Prepare
```

Preparation scans the executable, `shaders.rpf`, and `Bink2Scarlett.dll` when
available beside it. It uses four shader workers, then a maximum of two
workers for recorded Direct3D pipelines. It prints how many shaders are
ready and how many could not be translated. Preparation can be repeated
and uses the same cache as play.

The first pipeline warm-up can have nothing to do. Add `-RecordPipelines`
to a game launch to record the pipelines requested by draws for the next
`-Prepare`. Texture streaming and unsupported shaders still need attention
during play. Add `-LogPath ./victory-run.log` to save frame and GPU diagnostics.

Before the first run, ahead of time (both can be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # pipelines earlier runs drew with, for a new driver or build
```

Commands, debugging switches and upstream's own description:
[docs/README-fxceless.md](docs/README-fxceless.md). Every `VICTORY_...` switch is
described in a comment where it is read.

## What is different here

In short; the reasons and the measurements are in [docs/findings.md](docs/findings.md).

- **List calls are written down, not carried out, on the title's threads**
  (`VICTORY_THIN`): eight threads of ours translate them.
- **Submissions are prepared by a thread of ours and joined**
  (`VICTORY_ASYNC_SUBMIT`, `VICTORY_JOIN_PARTS`), with what can be spread over
  threads spread: refreshes, compares (AVX2), descriptor records, landings.
- **What shaders wrote comes back beside the write guards**, by the dwords that
  changed, on a thread of its own (`VICTORY_RELAY`, `VICTORY_LAND`).
- **Buffer fills that need bytes still on the card are done on the card**
  (`VICTORY_CARD_STEPS`) instead of waiting for it.
- **The title's writes are caught more cheaply**: one view opened at a fault,
  blocks that rest, places in the title's code that look before they write.
- **Textures go up on a copy queue of their own** and a slot shows one only after
  its upload ran, which also ended a device loss that came every 15 minutes or so.
- **Boats float**: textures that shaders write and the title's CPU reads go back
  to title memory (from fxceless and Geckomacabre, on by default here).
- **Frame record and replay** with per-pass times and shader variants compared
  turn by turn ([docs/replay.md](docs/replay.md)).

## Credits

This is a tree of other people's work with more on top. The history is kept, and
commits taken from elsewhere name the commit and its author.

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, the runtime services, the Direct3D 12 back end and
  the shader recompiler.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**: the fork
  this one started from, and the people whose pull requests it carries.
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**: ray tracing fixes,
  cube gradients, sparse textures.
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**:
  the ambient light idea, pipeline workers, and work by romainhedouin.
- **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**.
- **[myokex/victory-amd](https://github.com/myokex/victory-amd)**: the guard over
  every view of a page (with Spexivus, joroyt and tightfall), and Joshua Odom's
  reads into title memory.
- **Geckomacabre** and **cxy0072019-sketch**: the texture read-back.
- **[XCL](https://github.com/ivall/xcl)** (GPL-2.0-only) was read for how the same
  title behaves under another layer; no code of it is here.
- This fork's changes were made with [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not
covered is named in [docs/asset-policy.md](docs/asset-policy.md).
