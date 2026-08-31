# quickmon manual hardware check

Run against a real Commodore 64 Ultimate on the local network.

## Step 1: Connection and settings

Launch `cargo run`. Open settings. Note that both the device IP and password fields are blank on every launch (expected behavior—no credentials are persisted between sessions). Enter the device IP, click **Test connection**. Expect the product name and firmware version. If it reports 403 (unauthorized), enter the network password in the same panel (both fields are visible at once — no need to close and reopen settings) and retry **Test connection**.

**Expected output:** The panel shows product name and firmware version. The exact strings depend on your device and firmware revision. What matters is that you see sensible product and firmware information, not an error message.

## Step 2: Clear the screen and prepare the C64

Put the Commodore 64 at the BASIC READY prompt. If the screen contains previous output, press SHIFT+CLR/HOME to clear it.

## Step 3: Assemble the HI program

Enter this source in the assembler pane, set the target address to `C000`, and click **Assemble**:

```
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

**Expected results:**
- Status bar shows: `29 bytes assembled at $C000`
- Listing pane shows the first line as: `C000  A9 93     LDA #$93`
- No assembler errors

If you see a different byte count, an error message, or the listing's first line does not start with `C000  A9 93`, the assembler or the typed source is incorrect — the source is far likelier. Copy the source above exactly, including spacing.

## Step 4: Send the program to the C64

Click **Send**.

**Expected result:** Status bar shows `29 bytes written to $C000 — verified`

If you see `verify FAILED at $XXXX: sent XX, read back XX`, the program reached the device but the verification read-back disagreed at that offset. This indicates a network or device issue, not an assembler problem. If you see a different byte count or a network error, check your connection settings.

## Step 5: Verify the bytes in device memory

In the memory viewer pane, enter address `C000` and length `32`, then click **Read**.

**Expected bytes (first 10 bytes):** `A9 93 20 D2 FF A9 08 8D 00 04`

If these bytes do not match, the send or verify step failed at the device level. If the bytes do match but the C64 does not run the program (next step), the problem is in the Commodore 64 itself, not in quickmon.

## Step 6: Execute the program on the C64

On the Commodore 64, type `SYS 49152` and press RETURN.

**Expected result:** The screen clears and the text `HI` appears in white at the top-left corner of the screen.

If the screen does not clear or `HI` does not appear:
- If you see nothing at all or a syntax error, check that you typed `SYS 49152` correctly (not a different number).
- If the screen clears but `HI` does not appear, check that color RAM was written correctly; the program sets both $D800 and $D801 to $01 (white). If the characters are invisible, the color is wrong, which means the bytes were not written as expected—go back to Step 5 and verify.
- If `HI` appears in a color other than white, the assembler, network, or device likely failed to set color RAM. Check Step 5 again.

## Step 7: Regression check on the sizing loop

This step confirms that the assembler correctly chooses absolute vs. zero-page addressing based on the origin address.

Assemble this source at origin `C000`:

```
LDA target
target: .byte $AA
```

**Expected result:** The listing shows:
```
C000  AD 03 C0  LDA target
C003  AA        target: .byte $AA
```

The `LDA` is 3 bytes (opcode `AD` plus 16-bit address `$C003`). The reason: the assembler assumes zero-page initially (which would be 2 bytes, placing `target` at `$C002`), but `$C002` exceeds the zero-page limit of `$FF`, so the instruction widens to absolute addressing and `target` lands at `$C003`.

Now assemble the exact same source at origin `0010`:

**Expected result:** The listing shows:
```
0010  A5 12     LDA target
0012  AA        target: .byte $AA
```

The `LDA` is 2 bytes (opcode `A5` for zero-page). At this origin, zero-page addressing is sufficient because `target` sits at `$0012`, within `$FF`.

If both origins produce different-sized instructions as shown above, the sizing loop is working correctly. If both produce the same size (both `AD` or both `A5`), the sizing loop is broken.

---

## Troubleshooting summary

| Symptom | Layer |
|---------|-------|
| Assembler error or wrong byte count | Assembler |
| `verify FAILED at $XXXX: sent .., read back ..` | Network or device |
| Bytes in device memory do not match Step 5 | Network or device |
| Program runs but `HI` does not appear | C64 or assembler (color RAM) |
| Wrong opcode in sizing loop | Assembler |
