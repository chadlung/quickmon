; ------------------------------------------------------------------
; "HELLO" in random colours, Commodore 64
; Prints HELLO centred on row 12 and recolours each letter at random
; every ten frames.  Randomness comes from SID voice 3 running noise.
; Screen RAM row 12 col 17 = $0400 + 12*40 + 17 = $05f1
; Colour RAM row 12 col 17 = $d800 + 12*40 + 17 = $d9f1
; Press any key to finish.
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

; --- put the letters on the screen ----------------------------------

        ldx #$00
textloop:
        lda hello,x
        sta $05f1,x
        inx
        cpx #$05
        bne textloop

; --- recolour the five letters over and over ------------------------

mainloop:
        jsr waitdelay
        ldx #$00
colourloop:
        jsr random
        sta $d9f1,x
        inx
        cpx #$05
        bne colourloop

        jsr $ffe4          ; KERNAL GETIN
        cmp #$00
        beq mainloop

        lda #$0e
        sta $d020          ; light blue border
        lda #$06
        sta $d021          ; blue screen
        lda #$93
        jsr $ffd2          ; clear screen
        rts

; --- one random colour, 1 to 15 (never black on a black screen) -----

random:
        lda $d41b          ; SID oscillator 3 output
        and #$0f
        bne randomdone
        lda #$01           ; swap black for white
randomdone:
        rts

; --- wait ten video frames ------------------------------------------

waitdelay:
        ldy #$0a
frameloop:
        lda $d012
        cmp #$ff
        beq frameloop      ; first leave raster line 255
wait2:
        lda $d012
        cmp #$ff
        bne wait2          ; then wait until we hit it again
        dey
        bne frameloop
        rts

; --- screen codes: H E L L O ----------------------------------------

hello:
        .byte $08,$05,$0c,$0c,$0f
