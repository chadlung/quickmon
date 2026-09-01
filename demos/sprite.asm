; ------------------------------------------------------------------
; Animated sprite for the Commodore 64
; Sprite 0, two-frame "jumping jack" animation, bouncing left/right.
; Press any key to stop.
; Assemble anywhere in free RAM (e.g. $C000) and SYS to "start".
; ------------------------------------------------------------------

start:
        lda #$00
        sta $d020          ; border  = black
        sta $d021          ; screen  = black
        lda #$93
        jsr $ffd2          ; KERNAL CHROUT: clear screen

; --- copy the two 64-byte shape blocks into the cassette buffer ----
; block 13 = 13*64 = $0340   block 14 = 14*64 = $0380

        ldx #$00
copyloop:
        lda frameone,x
        sta $0340,x
        lda frametwo,x
        sta $0380,x
        inx
        cpx #$40
        bne copyloop

; --- variables ------------------------------------------------------

        lda #$00
        sta dir            ; 0 = moving right, 1 = moving left
        sta xhi            ; high bit of the 9-bit X position
        sta $d010          ; clear all sprite X MSBs
        lda #$18
        sta xlo            ; start at X = 24
        lda #$08
        sta delay          ; frames between animation steps

; --- set up sprite 0 ------------------------------------------------

        lda #$0d
        sta $07f8          ; sprite 0 pointer -> block 13 ($0340)
        lda #$01
        sta $d015          ; enable sprite 0
        sta $d017          ; expand Y (2x)
        sta $d01d          ; expand X (2x)
        lda #$07
        sta $d027          ; sprite 0 colour = yellow
        lda #$82
        sta $d001          ; Y = 130

; --- main loop ------------------------------------------------------

mainloop:
        jsr waitframe
        jsr movesprite
        jsr animate
        jsr $ffe4          ; KERNAL GETIN
        cmp #$00
        beq mainloop

quit:
        lda #$00
        sta $d015          ; turn the sprite off
        lda #$0e
        sta $d020          ; restore light blue border
        lda #$06
        sta $d021          ; restore blue screen
        rts

; --- wait for one full video frame ----------------------------------

waitframe:
        lda $d012
        cmp #$ff
        beq waitframe      ; first leave raster line 255
wait2:
        lda $d012
        cmp #$ff
        bne wait2          ; then wait until we hit it again
        rts

; --- swap the shape every "delay" frames ----------------------------

animate:
        dec delay
        bne animdone
        lda #$08
        sta delay
        lda $07f8
        eor #$03           ; $0d <-> $0e : toggles block 13 / 14
        sta $07f8
animdone:
        rts

; --- move one pixel, bounce between X=24 and X=330 ------------------

movesprite:
        lda dir
        bne moveleft

        inc xlo
        bne checkright
        inc xhi
checkright:
        lda xhi
        beq putx           ; still under 256, nothing to test
        lda xlo
        cmp #$4a           ; 256 + 74 = 330
        bcc putx
        lda #$01
        sta dir            ; hit the right edge, turn around
        jmp putx

moveleft:
        lda xlo
        bne skiphi
        dec xhi            ; borrow into the high byte
skiphi:
        dec xlo
        lda xhi
        bne putx
        lda xlo
        cmp #$18           ; 24
        bcs putx
        lda #$00
        sta dir            ; hit the left edge, turn around

putx:
        lda xlo
        sta $d000          ; low 8 bits of X
        lda xhi
        beq clearmsb
        lda $d010
        ora #$01           ; set sprite 0's 9th X bit
        sta $d010
        rts
clearmsb:
        lda $d010
        and #$fe           ; clear sprite 0's 9th X bit
        sta $d010
        rts

; --- variables ------------------------------------------------------

dir:    .byte $00
delay:  .byte $08
xlo:    .byte $18
xhi:    .byte $00

; --- shape data, 24 x 21 pixels, 63 bytes + 1 pad byte --------------

frameone:                  ; arms up
        .byte $00,$3c,$00
        .byte $00,$7e,$00
        .byte $00,$5a,$00
        .byte $00,$7e,$00
        .byte $00,$42,$00
        .byte $0c,$7e,$30
        .byte $06,$3c,$60
        .byte $03,$18,$c0
        .byte $01,$ff,$80
        .byte $00,$ff,$00
        .byte $00,$ff,$00
        .byte $00,$ff,$00
        .byte $00,$ff,$00
        .byte $00,$ff,$00
        .byte $00,$ff,$00
        .byte $00,$e7,$00
        .byte $00,$e7,$00
        .byte $00,$c3,$00
        .byte $00,$c3,$00
        .byte $01,$c3,$80
        .byte $03,$c3,$c0
        .byte $00

frametwo:                  ; arms down
        .byte $00,$3c,$00
        .byte $00,$7e,$00
        .byte $00,$5a,$00
        .byte $00,$7e,$00
        .byte $00,$42,$00
        .byte $00,$7e,$00
        .byte $00,$3c,$00
        .byte $00,$18,$00
        .byte $00,$ff,$00
        .byte $01,$ff,$80
        .byte $03,$ff,$c0
        .byte $06,$ff,$60
        .byte $0c,$ff,$30
        .byte $00,$ff,$00
        .byte $00,$ff,$00
        .byte $00,$e7,$00
        .byte $00,$e7,$00
        .byte $00,$c3,$00
        .byte $00,$c3,$00
        .byte $01,$c3,$80
        .byte $03,$c3,$c0
        .byte $00
