; ------------------------------------------------------------------
; Random coloured dots, Commodore 64
; Sprinkles "." characters at random screen positions in random
; colours until a key is pressed.
; Randomness comes from SID voice 3 running noise: once the voice is
; set to a high frequency with the noise waveform, $d41b holds a fresh
; pseudo-random byte every time you read it.
; Screen RAM is $0400-$07e7, colour RAM is $d800-$dbe7 - the same
; offset into both, so one random offset drives a pair of pointers.
; Screen code $2e = "."
; Assemble anywhere in free RAM (e.g. $C000) and SYS to "start".
; ------------------------------------------------------------------

start:
        lda #$00
        sta $d020          ; border = black
        sta $d021          ; screen = black
        lda #$93
        jsr $ffd2          ; KERNAL CHROUT: clear screen

; --- turn SID voice 3 into a random number generator ----------------

        lda #$ff
        sta $d40e          ; voice 3 frequency low
        sta $d40f          ; voice 3 frequency high
        lda #$80
        sta $d412          ; noise waveform, gate off
        sta $d418          ; volume 0, voice 3 disconnected from output

; --- drop one dot per pass until a key is pressed -------------------

mainloop:
        jsr pickspot       ; $fb/$fc -> screen, $fd/$fe -> colour
        ldy #$00
        lda #$2e
        sta ($fb),y        ; the dot itself
        jsr randomcolour
        sta ($fd),y        ; its colour
        jsr delay

        jsr $ffe4          ; KERNAL GETIN
        cmp #$00
        beq mainloop       ; no key yet, keep sprinkling

; --- tidy up and hand control back to BASIC -------------------------

        lda #$0e
        sta $d020          ; light blue border
        lda #$06
        sta $d021          ; blue screen
        lda #$93
        jsr $ffd2          ; clear screen
        rts

; --- random screen position -----------------------------------------
; Builds a pointer to $0400-$07e7 in $fb/$fc and the matching colour
; RAM pointer in $fd/$fe.

pickspot:
        lda $d41b          ; SID oscillator 3 output
        sta $fb            ; low byte of both pointers
        sta $fd
        lda $d41b
        and #$03           ; one of the four screen pages
        clc
        adc #$04
        sta $fc            ; high byte $04-$07
        cmp #$07
        bne spotok
        lda $fb
        cmp #$e8
        bcs pickspot       ; $07e8-$07ff are sprite pointers, re-roll
spotok:
        lda $fc
        clc
        adc #$d4           ; $04+$d4 = $d8, so page $04-$07 -> $d8-$db
        sta $fe
        rts

; --- one random colour, 1 to 15 (never black on a black screen) -----

randomcolour:
        lda $d41b          ; SID oscillator 3 output
        and #$0f
        bne colourdone
        lda #$01           ; swap black for white
colourdone:
        rts

; --- short pause so the dots appear one at a time -------------------

delay:
        ldx #$04
outerdelay:
        ldy #$ff
innerdelay:
        dey
        bne innerdelay
        dex
        bne outerdelay
        rts
