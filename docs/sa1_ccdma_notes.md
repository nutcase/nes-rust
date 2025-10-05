# SA-1 CC-DMA Research Notes

## Implementation Status (September 26, 2025)
- Type-1 character conversion DMA is now modelled in `Bus::sa1_ccdma_type1`, and the semi-automatic Type-2 path shares the same bitplane converter (treating the BRF/descriptor data as 64 unpacked pixels per tile). Both modes expand into SNES bitplanes stored in the SA-1 BW-RAM window, so the S-CPU MDMA handler can pull the converted tiles without relying on the old mailbox hack.
- The helper accepts 2/4/8bpp descriptors (reserved mode `3` is treated as 4bpp, matching DQ3's usage) and honours the virtual width hint from `$2231`. Output is laid out sequentially, mirroring hardware's streaming behaviour.
- Compatibility shim `Sa1::initialize_bwram_for_dq3` has been removed. Successful headless runs (`HEADLESS=1 HEADLESS_FRAMES=40 QUIET=1 cargo run --release -- roms/dq3.sfc`) now produce non-zero VRAM/OAM activity from the real CC-DMA→MDMA chain.
- Type-2 (semi-automatic) conversion now runs through the shared bitplane converter, so remaining follow-up is limited to buffer toggling fidelity if we ever capture carts that interleave the double-buffer handshake.

## Observations from Emulator Traces
- During the first 200 headless frames (`HEADLESS_FRAMES=200`) Dragon Quest III never writes non-zero values to `$2230-$2239`; the CC-DMA control register `$2231` stays at 0x00. All logged traffic to those addresses comes from our own debug prints inside the emulator.
- The S-CPU still performs the heavy lifting via regular MDMA:
  - Channel 2 runs a 64 KiB DMA from `C0:05DE` into `$2118/$2119` (VRAM tile upload).
  - Channel 0 follows with a large transfer from `7E:DB1F` into `$2104` (OAM upload). The run summary shows `OAM=66128`, matching the expected sprite data upload once the SA-1 handshake completes.
  - Multiple subsequent CPU-initiated DMAs drive other PPU ports (`$2100`, `$2140`, etc.), consistent with the game’s frame setup.
- SA-1 firmware block at `C0:05B0` (see `TRACE_SA1_AFTER_LOOP` output) prepares the S-CPU DMA registers directly after the SA-1 signals completion via the BW-RAM mailbox (`30:6030`/`30:6031`).

## Implications for Stage 5 (CC-DMA implementation)
1. **Register semantics**: We need confirmed documentation for `$2230-$223F`. Candidates include the SA-1 datasheet and Vitor Vilela’s notes; link placeholders live in `docs/sa1_sources.md`.
2. **Pipeline modeling**:
   - ROM → (optional) BW-RAM or I-RAM staging via SA-1 internal DMA.
   - Character conversion (bitplane interleaving) before the S-CPU sees the buffer.
   - Shared handshake so the S-CPU DMA copies from the converted buffer into VRAM/OAM/CGRAM.
3. **Mapping requirements**:
   - SA-1 BW-RAM/I-RAM windows must mirror hardware so that addresses like `7E:DB1F` during the S-CPU DMA correspond to the data the SA-1 prepared.
   - The emulator should only signal completion IRQs (`$2202` clears) once the converted buffer is ready, avoiding forced writes to `30:6030`.
4. **Open questions**:
   - Does DQIII rely on CC-DMA for additional resources later (after the initial 200 frames)? Need longer trace runs once logging is lighter.
   - Which conversion mode (`$2231` bits) does it actually expect? We should capture a playthrough on real hardware or a reference emulator to compare.

## Next Instrumentation Steps
- Add targeted logging for any writes to `$2230-$2239` coming from the S-CPU (not just the SA-1 helper), but guard with an env flag to keep noise low.
- Dump the BW-RAM pages around `30:6000` before and after the SA-1 handshake to verify what data the S-CPU DMA reads.
- Once register semantics are confirmed, sketch the real CC-DMA implementation flow inside `Sa1::execute_ccdma` and `Bus::sa1_cc_dma_transfer`.

## Instrumentation
- Set `TRACE_SA1_CCDMA=1` to log every S-CPU/SA-1 write to `$2230-$2239` along with the derived source/dest/length and handshake state.
- The logs also annotate when `sa1_cc_dma_transfer` runs and when the completion IRQ fires, helping confirm whether games actually trigger CC-DMA.

### First non-zero CC-DMA writes
- With `TRACE_SA1_CCDMA=1` and a 200-frame headless run, the game eventually programs `$2230-$2239` with non-zero values (e.g., `$2230=0x57`, `$2231=0x01`, `$2232-34` forming source `0xF840BA`, `$2235-37` forming destination `0x221A1B`, length `0x0137`).
- The handshake state was still `1` when these writes landed, implying the SA-1 firmware prepares the descriptor before re-arming the buffer and IRQ.

- Sample descriptor (after ~200 frames with `TRACE_SA1_CCDMA=1`):
  - $2230=0x57, $2231=0x01
  - Source: $F8:40BA (first bytes `1D 38 5F 23 ...`)
  - Destination: $C0:FFB3 (first bytes `00 83 00 E1 ...`)
  - Length: 0x1D9D
  - Handshake state transitions to 1 and `ccdma_pending` becomes 1 just before the bus invokes `sa1_cc_dma_transfer`.
  - Another descriptor shows different source/dest pairs (`$85:C0FF`, `$9D:C0FF`, etc.), implying the firmware chains multiple CC-DMA jobs.
