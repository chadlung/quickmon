; ------------------------------------------------------------------
; White line across the middle of the screen, Commodore 64
; Text mode version: 40 horizontal-line characters on row 12.
; Screen RAM row 12 = $0400 + 12*40 = $05e0
; Colour RAM row 12 = $d800 + 12*40 = $d9e0
; Press any key to finish.
; Assemble anywhere in free RAM (e.g. $C000) and SYS to "start".
; ------------------------------------------------------------------

start:
        lda #$00
        sta $d021          ; screen = black
        sta $d020          ; border = black
        lda #$93
        jsr $ffd2          ; KERNAL CHROUT: clear screen

        ldx #$00
lineloop:
        lda #$40
        sta $05e0,x        ; screen code 64 = horizontal line
        lda #$01
        sta $d9e0,x        ; colour = white
        inx
        cpx #$28           ; 40 columns
        bne lineloop

waitkey:
        jsr $ffe4          ; KERNAL GETIN
        cmp #$00
        beq waitkey

        lda #$0e
        sta $d020          ; light blue border
        lda #$06
        sta $d021          ; blue screen
        lda #$93
        jsr $ffd2          ; clear screen
        rts
