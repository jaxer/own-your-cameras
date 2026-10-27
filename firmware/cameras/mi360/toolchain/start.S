/* Minimal independent O32 entry stub; calls the public uClibc startup ABI.
 * ET_EXEC only. The loader's $v0 finalizer is preserved as argument six.
 */
.set noreorder
.abicalls
.option pic0
.text
.globl __start
.type __start,@function
.ent __start
__start:
    lui $gp, %hi(_gp)
    addiu $gp, $gp, %lo(_gp)
    move $ra, $zero
    lui $a0, %hi(main)
    addiu $a0, $a0, %lo(main)
    lw $a1, 0($sp)
    addiu $a2, $sp, 4
    /* Preserve the original auxv before libc may replace environ. */
    addiu $t0, $a1, 1
    sll $t0, $t0, 2
    addu $t0, $a2, $t0
2:  lw $t1, 0($t0)
    bnez $t1, 2b
    addiu $t0, $t0, 4
    lui $t1, %hi(probe_auxv)
    sw $t0, %lo(probe_auxv)($t1)
    li $t0, -8
    and $sp, $sp, $t0
    addiu $sp, $sp, -32
    move $a3, $zero
    sw $zero, 16($sp)
    sw $v0, 20($sp)
    sw $sp, 24($sp)
.option pic2
    lw $t9, %call16(__uClibc_main)($gp)
    jalr $t9
    nop
1:  b 1b
    nop
.end __start
.size __start, .-__start
.section .note.GNU-stack,"",@progbits
