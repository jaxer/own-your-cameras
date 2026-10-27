/* Compatibility only for interfaces absent from stock uClibc 0.9.33.2.
 * No substitute pthread implementation; all thread operations use stock libpthread.
 */
typedef unsigned long word;
word *probe_auxv;
extern int *__errno_location(void);
extern word pthread_self(void);
extern int prctl(int option, ...);

word getauxval(word key) {
    if (probe_auxv) {
        word *aux = probe_auxv;
        for (; aux[0]; aux += 2) if (aux[0] == key) return aux[1];
    }
    *__errno_location() = 2; /* ENOENT */
    return 0;
}
int pthread_setname_np(word thread, const char *name) {
    if (thread != pthread_self()) return 22; /* EINVAL: foreign-thread naming unsupported */
    if (prctl(15, name, (word)0, (word)0, (word)0)) return *__errno_location();
    return 0;
}
