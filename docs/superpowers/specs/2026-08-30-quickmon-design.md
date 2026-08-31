# quickmon — design

**Date:** 2026-08-30
**Status:** approved design, pre-implementation

## Purpose

A desktop application for writing 6510 assembly, assembling it to machine code, and
writing that machine code directly into the memory of a Commodore 64 Ultimate over
its REST API.

The user runs the code themselves on the machine (via the C64's own monitor, a `SYS`
from BASIC, or any other means). quickmon does not attempt to start execution. Its
job ends when the bytes are confirmed present at the requested address.

## Non-goals (v1)

- No BASIC stub generation, no `.prg` emission, no auto-run.
- No triggering execution on the C64 by any mechanism.
- No macros, includes, or expression arithmetic in the assembler.
- No debugger, breakpoints, or single-stepping.
- No emulator integration.

## Environment

- Rust 1.94.1, edition 2024.
- `iced` 0.14 (MSRV 1.88).
- Target device: Commodore 64 Ultimate on the local network, firmware exposing the
  1541U REST API v1.
- Development host: macOS (darwin).

## Architecture

```
src/
  main.rs          iced entry point, window configuration
  app.rs           State, Message, update(), view() composition
  config.rs        persisted settings (last-used file dialog directory only)
  asm/
    mod.rs         pub fn assemble(src: &str, org: u16) -> Result<Assembly, Vec<AsmError>>
    opcodes.rs     the (Mnemonic, AddrMode) -> (opcode, length) table
    parser.rs      source line -> Stmt { label, mnemonic, operand }
    encode.rs      iterative width resolution, then byte emission
    error.rs       AsmError { line, col, message }
  net/
    mod.rs
    client.rs      UltimateClient: writemem, readmem, version, info
    error.rs       NetError
  ui/
    hex.rs         hex formatting/parsing helpers (addresses, byte dumps)
    settings.rs    connection settings panel
    memview.rs     memory viewer pane
```

The source editor and assembled listing panes are not separate `ui/` modules;
they are composed directly in `app.rs::view`.

### Layering rule

`asm/` is a pure library. It has no dependency on `iced`, `reqwest`, or any I/O. It
takes a `&str` and a `u16` and returns bytes or errors. This is what allows the
assembler — the part with the most logic and the most ways to be subtly wrong — to be
tested exhaustively without a window or a C64.

`net/` is likewise independent of `iced`. It is testable against a mock HTTP server.

`app.rs` and `ui/` are the only modules that know about iced.

## User interface

Single window. Layout:

```
┌──────────────────────────────────────────────────────────┐
│ Target: $[C000]  [Assemble] [Send]  ● connected  [⚙]     │
├────────────────────────────┬─────────────────────────────┤
│ source editor              │ assembled listing           │
│                            │                             │
│ LDA #$08                   │ C000  A9 08     LDA #$08    │
│ STA $0400                  │ C002  8D 00 04  STA $0400   │
│ RTS                        │ C005  60        RTS         │
├────────────────────────────┴─────────────────────────────┤
│ Memory  $[0400] len [1000]  [Read]        (collapsible)  │
│ 0400  08 09 20 20 20 20 20 20  ..                        │
├──────────────────────────────────────────────────────────┤
│ status bar                                               │
└──────────────────────────────────────────────────────────┘
```

- **Target address field** — hex, no `$` required, validated on input. This single
  value feeds both the assembler's origin and the `writemem` address, so the two can
  never disagree.
- **Listing pane** — address, emitted bytes, and the originating source line. On a
  failed assemble this pane shows the error list instead, each entry carrying its
  line number.
- **Memory viewer** — collapsible, read-only, driven by `readmem` at an arbitrary
  address and length.
- **Settings panel** — host/IP and network password, both session-only and blank on
  every launch (noted as such in the panel), and a *Test connection* button that calls
  `GET /v1/version` and `GET /v1/info`.
- **Status bar** — the result of the last action.

## Assembler

### Grammar (v1)

```
line     := [label] [instruction | directive] [comment]
label    := identifier [':']
comment  := ';' .*
operand  := literal | identifier | addressing-mode form
literal  := '$' hex | decimal | '%' binary
```

Supported: all 56 documented 6510 mnemonics, all 13 addressing modes, labels,
`;` comments, the directives `.byte` / `.word` (accepting comma-separated lists), and
the optional `.b` / `.w` mnemonic width suffixes described under *Explicit width
override* below.

Case-insensitive mnemonics and width suffixes. Labels are case-sensitive.

### Addressing mode detection

Determined syntactically from the operand form:

| Operand form | Mode |
|---|---|
| *(none)* | implied |
| `A` or none, for ASL/LSR/ROL/ROR | accumulator |
| `#$08` | immediate |
| `$08` | zero page |
| `$08,X` / `$08,Y` | zero page,X / zero page,Y |
| `$0400` | absolute |
| `$0400,X` / `$0400,Y` | absolute,X / absolute,Y |
| `($08,X)` | indexed indirect |
| `($08),Y` | indirect indexed |
| `($0400)` | indirect (JMP only) |
| target of a branch mnemonic | relative |

If a mnemonic does not support the detected mode, that is an `AsmError` naming both
the mnemonic and the mode.

### Encoding: iterative size resolution, then emit

A naive two-pass assembler has to guess the width of a forward-referenced operand
before it knows the symbol's value, and the conventional guess — always absolute —
silently costs a byte whenever the target turns out to be in zero page. quickmon
resolves the width correctly instead of guessing.

**Sizing loop (fixpoint).** Every operand whose width is ambiguous starts at its
*smallest* legal encoding (zero page). The assembler then repeats:

1. Walk the statements, assigning addresses using the current width assumptions and
   recording label definitions.
2. For each ambiguous operand whose symbol now resolves to a value `> $FF`, widen it
   to absolute.
3. If anything widened, discard the addresses and repeat.

Widths only ever grow, never shrink. That monotonicity is what guarantees termination
— each iteration either widens at least one instruction or ends the loop, and there
are finitely many instructions, so it converges in at most *N* iterations for *N*
instructions. (Allowing shrinking would let the assembler oscillate forever between
two states, which is why this direction is the standard one.)

**Emit pass.** Once widths are stable, emit bytes, resolve label references, and
compute branch offsets against the now-final addresses.

The result: a forward-referenced zero-page label assembles to a 2-byte zero-page
instruction, exactly as a backward reference to the same label would. Reference
direction does not affect output.

**Ambiguity is per-mnemonic.** The loop only considers addressing modes the mnemonic
actually supports. `JMP` and `JSR` have no zero-page form, so they are never
ambiguous. `STX` supports zero page,Y but not absolute,Y, so `STX label,Y` has exactly
one legal encoding. Only operands with two legal widths enter the loop.

**Explicit width override.** `.b` and `.w` mnemonic suffixes force an encoding:

```asm
LDA.b $10      ; force zero page   -> A5 10
LDA.w $10      ; force absolute    -> AD 10 00
```

This matters on real hardware — absolute addressing of a low address takes an extra
cycle, and some code depends on that timing or on the instruction's length. Forcing
`.b` on an operand that cannot fit in a byte is an error rather than a truncation.

**Branch range:** relative branches are range-checked. Out of range produces an error
naming the actual displacement and the limit — e.g. `line 7: branch to 'loop' is -142
bytes, limit is -128` — rather than a silently truncated offset.

Because branches are always two bytes, they do not participate in the sizing loop;
but their *targets* move as other instructions widen, so range checking happens in the
emit pass, after widths are final.

### Assembly output

```rust
pub struct Assembly {
    pub org: u16,
    pub bytes: Vec<u8>,
    pub lines: Vec<ListingLine>,  // address, byte span, source line index
    pub symbols: BTreeMap<String, u16>,
}
```

`lines` is what drives the listing pane; keeping it in the assembler's output means
the UI does no re-derivation.

## Network client

### Endpoints used

| Call | Purpose |
|---|---|
| `POST /v1/machine:writemem?address={:04X}` | send assembled bytes, `Content-Type: application/octet-stream` |
| `GET /v1/machine:readmem?address={:04X}&length={n}` | verification read-back and the memory viewer |
| `GET /v1/version` | connection test |
| `GET /v1/info` | connection test; reports product, firmware, hostname |

An optional `X-Password` header is attached to every request when a password is
configured (firmware 3.12+).

### The 200-with-errors contract

The API returns **HTTP 200 with a JSON body containing an `"errors"` array**. A 200
status alone does not mean the operation succeeded. The client parses the body and
treats a non-empty `errors` array as a failure, surfacing its contents.

This is the single most likely thing to get wrong in this integration and is called
out explicitly as a required test case.

### Send pipeline

1. Validate `org as usize + bytes.len() <= 0x10000`. The API documents that data must
   not wrap past `$FFFF`; a wrap is rejected locally with a clear message rather than
   sent.
2. `POST` the bytes.
3. Parse the response `errors` array; abort on non-empty.
4. `GET readmem` at the same address for the same length.
5. Byte-compare. Report either `N bytes verified at $ADDR` or the first differing
   offset with expected vs actual.

### Device constraints surfaced in the UI

Two documented constraints are shown to the user rather than left to be rediscovered:

- **`$0000` and `$0001` cannot be written.** These are the 6510's on-chip I/O port;
  DMA on the cartridge bus cannot reach them. A target address whose byte range covers
  either location produces a warning before sending.
- **Writes use the machine's currently-selected memory map.** What `$D800` resolves to
  depends on the C64's current banking state. This is the concrete reason
  verify-after-write is in v1 rather than deferred.

## Error handling

`thiserror` throughout. Three error domains, kept distinct because they need
different messages and different user actions:

- `AsmError { line, col, message }` — carries a line number so the listing pane can
  point at the offending source line.
- `NetError::Transport` — host unreachable, DNS failure, timeout. User action: check
  the device is on and the host is right.
- `NetError::Http { status }` — a non-200 response, including 403 for a bad or missing
  password.
- `NetError::Api { errors: Vec<String> }` — HTTP 200 with a non-empty `errors` array.
  User action depends on the message from the device.

## Configuration

**No connection settings are persisted.** Neither the host nor the password ever
reaches disk; both are typed each run and held only in memory for the lifetime of the
process. The only thing persisted to a config file in the platform config directory
(`directories` crate) is the last-used file dialog directory — a UI convenience for
Open/Save, unrelated to connection state.

This removes the question of at-rest storage for connection settings entirely — there
is nothing on disk to protect, so no keychain integration is needed and no plaintext
secret exists in the config file. It also removes an entire failure mode: a settings
write that fails silently and leaves the user wondering why their host vanished
between runs. Nothing is written, so nothing can fail to be written.

Consequences, accepted deliberately:

- Both the host and password fields start empty on every launch. If the device has a
  network password set, the first request of a session fails with 403 until it is
  entered. The settings panel says so.
- The password is still transmitted as a plain `X-Password` header over HTTP, which is
  what the device's API requires. Not persisting it does not change that, and this is
  a closed-network tool where that is acceptable.

## File operations

Open and Save for the editor buffer, plain UTF-8 text, `.asm` extension by default,
via a native file dialog (`rfd`). No project format, no session restore beyond the
config values above.

## Testing

### Assembler (no hardware, no window)

- One test per addressing mode asserting exact emitted bytes.
- Label resolution: backward reference, forward reference, undefined symbol error.
- **Reference-direction symmetry.** The same program with a zero-page label defined
  before vs. after its use must produce byte-identical output. This is the direct
  regression test for the sizing loop.
- **Sizing-loop convergence.** A case where widening one instruction pushes a later
  label past `$FF` and forces a second widening, asserting the loop settles and the
  final addresses are correct. Plus a case that requires more than two iterations.
- **Width overrides.** `LDA.b $10` emits `A5 10`; `LDA.w $10` emits `AD 10 00`;
  `LDA.b $1234` is an error, not a truncation.
- Per-mnemonic ambiguity: `JMP label` is never zero-page; `STX label,Y` has one legal
  encoding and does not enter the loop.
- Branch offsets: forward, backward, both range boundaries, and out-of-range error.
  Includes a branch whose displacement changes because an intervening instruction
  widened during the sizing loop.
- `.byte` / `.word` including multi-value lists.
- Error cases: unknown mnemonic, mode not supported by mnemonic, malformed operand,
  literal out of range.
- **Golden test.** The known-good program below must assemble at `$C000` to exactly:

  ```
  A9 93 20 D2 FF A9 08 8D 00 04 A9 09 8D 01 04
  A9 01 8D 00 D8 8D 01 D8 A9 0D 20 D2 FF 60
  ```

  ```asm
  LDA #$93
  JSR $FFD2
  LDA #$08
  STA $0400
  LDA #$09
  STA $0401
  LDA #$01
  STA $D800
  STA $D801
  LDA #$0D
  JSR $FFD2
  RTS
  ```

  These bytes were verified running on a real C64 (VICE, x64sc) before this project
  existed, which makes them a trustworthy regression anchor rather than a
  self-referential expectation.

### Network client (mock server)

Using `wiremock`:

- `writemem` sends the correct method, path, query, `Content-Type`, and body.
- `X-Password` is attached when configured and omitted when not.
- **HTTP 200 with a non-empty `errors` array is treated as a failure.**
- HTTP 403 maps to `NetError::Http`.
- Connection refused maps to `NetError::Transport`.
- `readmem` returns the binary body unchanged.
- The wrap-past-`$FFFF` guard rejects locally without issuing a request.

### Configuration

- A round-trip save/load of the config preserves only `last_dir` and **contains
  neither a host nor a password field**, asserted against the serialized output.

### Not covered by automated tests

The real device round-trip. Verified manually against the hardware once the app runs.

## Dependencies

| Crate | Purpose |
|---|---|
| `iced` 0.14 | GUI |
| `reqwest` | HTTP client |
| `tokio` | async runtime for reqwest |
| `serde` / `serde_json` | API response parsing |
| `thiserror` | error types |
| `directories` | config file location |
| `rfd` | native file dialogs |
| `wiremock` (dev) | HTTP mocking in tests |

## Build order

1. `asm/` with its full test suite — the golden test passing is the first milestone.
2. `net/` with its mock-server tests.
3. iced shell: editor, listing, target address, assemble.
4. Send + verify pipeline wired to the UI.
5. Settings panel and config persistence.
6. Memory viewer.
7. File open/save.

Each step is independently verifiable. Steps 1 and 2 are complete and proven before
any UI code exists.
