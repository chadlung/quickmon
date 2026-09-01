; ------------------------------------------------------------------
; White line across the middle of the screen, Commodore 64
; Hi-res bitmap mode, bitmap at $2000, colour cells at $0400.
; Draws a 320-pixel horizontal line at y = 100.
; Press any key to return to text mode.
; Assemble anywhere in free RAM (e.g. $C000) and SYS to "start".
; ------------------------------------------------------------------

start:
        lda #$00
        sta $d020          ; border = black

; --- clear the bitmap, $2000 - $3fff --------------------------------

        lda #$00
        sta $fb            ; pointer low
        lda #$20
        sta $fc            ; pointer high
        ldx #$20           ; 32 pages
        ldy #$00
        lda #$00
clearloop:
        sta ($fb),y
        iny
        bne clearloop
        inc $fc
        dex
        bne clearloop

; --- colour cells: white foreground on black background -------------
; in bitmap mode $0400 holds the colours, high nybble = pixels on

        lda #$10
        ldx #$00
colourloop:
        sta $0400,x
        sta $0500,x
        sta $0600,x
        sta $06e8,x
        inx
        bne colourloop

; --- switch the VIC-II into hi-res bitmap mode ----------------------

        lda #$3b
        sta $d011          ; bitmap on, screen on, 25 rows
        lda #$c8
        sta $d016          ; single colour, 40 columns
        lda #$18
        sta $d018          ; screen $0400, bitmap $2000

; --- draw the line --------------------------------------------------
; y=100 -> cell row 12, sub-row 4
; $2000 + 12*320 + 4 = $2f04, then +8 for each cell across

        lda #$04
        sta $fb
        lda #$2f
        sta $fc
        ldx #$28           ; 40 cells wide
        ldy #$00
lineloop:
        lda #$ff           ; all 8 pixels of this cell lit
        sta ($fb),y
        clc
        lda $fb
        adc #$08           ; step to the next cell
        sta $fb
        bcc nocarry
        inc $fc
nocarry:
        dex
        bne lineloop

; --- hold until a key is pressed ------------------------------------

waitkey:
        jsr $ffe4          ; KERNAL GETIN
        cmp #$00
        beq waitkey

; --- back to text mode ----------------------------------------------

        lda #$1b
        sta $d011          ; bitmap off
        lda #$15
        sta $d018          ; screen $0400, charset $1000
        lda #$0e
        sta $d020          ; light blue border
        lda #$06
        sta $d021          ; blue screen
        lda #$93
        jsr $ffd2          ; clear screen
        rts
