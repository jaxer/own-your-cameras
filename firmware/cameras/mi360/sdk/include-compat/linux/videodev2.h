/* The pinned IMP headers include this Linux header but do not use its declarations
 * in the selected sensor/FS/encoder types or calls. No V4L2 ABI is supplied here.
 * This avoids borrowing unrelated host kernel headers during the Mac cross-build.
 */
