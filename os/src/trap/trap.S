.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm

.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    # sp -> user stack, sscratch -> kernel stack
    csrrw sp, sscratch, sp
    # now sp-> kernel stack, sscratch -> user stack
    addi sp, sp, -8*34
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    csrr t0, sstatus
    csrr t1, sepc
    csrr t2, sscratch
    sd t0, 8*32(sp)
    sd t1, 8*33(sp)
    sd t2, 8*2(sp)
    mv a0, sp
    call trap_handler

__restore:
    # a0 is the return value to trap_handler
    mv sp, a0
    ld t2, 8*2(sp)
    ld t1, 8*33(sp)
    ld t0, 8*32(sp)
    csrw sscratch, t2
    csrw sepc, t1
    csrw sstatus, t0
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # release trap_context on kernel stack
    addi sp, sp, 8*34

    # sp-> kernel stack, sscratch -> user stack
    csrrw sp, sscratch, sp
    # now sp -> user stack, sscratch -> kernel stack
    sret