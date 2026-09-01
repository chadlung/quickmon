; ------------------------------------------------------------------
; Background colour toggler, Commodore 64
; RETURN steps the background through the 16 VIC-II colours.
; Any other key restores the normal blue screen and exits.
; Keys are read with KERNAL GETIN ($ffe4), which returns $00 when the
; keyboard buffer is empty, so the wait loop just spins on zero.
; RETURN is PETSCII $0d.
; Assemble anywhere in free RAM (e.g. $C000) and SYS to "start".
; ------------------------------------------------------------------

start:
        lda #$00
        sta $d020          ; border = black
        sta $d021          ; screen = black
        lda #$93
        jsr $ffd2          ; KERNAL CHROUT: clear screen

; --- print the instructions -----------------------------------------

        ldx #$00
printloop:
        lda message,x
        beq getkey         ; $00 marks the end of the text
        jsr $ffd2          ; KERNAL CHROUT: print one character
        inx
        bne printloop

; --- wait for a key and act on it -----------------------------------

getkey:
        jsr $ffe4          ; KERNAL GETIN
        cmp #$00
        beq getkey         ; nothing pressed yet, keep waiting
        cmp #$0d           ; RETURN?
        bne quit           ; no - any other key ends the program

; --- next background colour -----------------------------------------

        inc $d021          ; step to the following colour
        lda $d021
        and #$0f           ; keep 0-15, wrap 15 back round to 0
        sta $d021
        jmp getkey

; --- tidy up and hand control back to BASIC -------------------------

quit:
        lda #$0e
        sta $d020          ; light blue border
        lda #$06
        sta $d021          ; blue screen
        lda #$93
        jsr $ffd2          ; clear screen
        rts

; --- PETSCII text: "RETURN = COLOR" / "ANY KEY = QUIT" --------------

message:
        .byte $52,$45,$54,$55,$52,$4e   ; RETURN
        .byte $20,$3d,$20                ; " = "
        .byte $43,$4f,$4c,$4f,$52        ; COLOR
        .byte $0d                        ; carriage return
        .byte $41,$4e,$59,$20            ; "ANY "
        .byte $4b,$45,$59,$20            ; "KEY "
        .byte $3d,$20                    ; "= "
        .byte $51,$55,$49,$54            ; QUIT
        .byte $0d
        .byte $00                        ; end marker
