/* RAM-only experiment, never linked into the product.
 * Run ONLY with vendor device stopped; reboot the camera afterwards.
 * The ioctl seam changes one payload pointer, then calls the REAL kernel.
 * This reproduces EFAULT/no-result-write, not allocation exhaustion.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>
#include "hdal.h"

struct event {
    uint32_t command, unit, item, length;
    int before, after, rc, error, injected;
};
static struct event events[32];
static unsigned count, fail_at;
static int recording;
static void *unreadable;

/* libhdal uses a five-word, 32-bit envelope. Only record while inside
 * DEV_CONFIG set. Do no logging or extra libc calls before capturing errno.
 */
int ioctl(int fd, unsigned long command, ...)
{
    va_list ap;
    va_start(ap, command);
    void *arg = va_arg(ap, void *);
    va_end(ap);
    if (!recording || command != 0xc0144905UL)
        return (int)syscall(SYS_ioctl, fd, command, arg);
    if (count == 32) _exit(90);
    uint32_t *words = arg;
    struct event *e = &events[count++];
    e->command = (uint32_t)command;
    e->unit = words[1];
    e->item = words[2];
    e->length = words[4];
    e->before = (int)words[0];
    uint32_t saved = words[3];
    if (count == fail_at) {
        /* Refuse to turn an immediate scalar into a pointer experiment. */
        if (!e->length) _exit(91);
        words[3] = (uint32_t)(uintptr_t)unreadable;
        e->injected = 1;
    }
    e->rc = (int)syscall(SYS_ioctl, fd, command, arg);
    e->error = errno;
    e->after = (int)words[0];
    words[3] = saved;
    errno = e->error;
    return e->rc;
}

static int require(const char *name, int rc)
{
    printf("setup %s=%d\n", name, rc);
    return rc != 0;
}

int main(int argc, char **argv)
{
    if (argc != 2 || (strcmp(argv[1], "0") && strcmp(argv[1], "1") && strcmp(argv[1], "5")))
        return 2;
    fail_at = (unsigned)atoi(argv[1]);
    setbuf(stdout, NULL);
    unreadable = mmap(NULL, 4096, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (unreadable == MAP_FAILED) return 3;
    _Static_assert(sizeof(void *) == 4, "ARM32 only");
    _Static_assert(sizeof(HD_AUDIOCAP_DEV_CONFIG) == 172, "pinned layout");
    if (require("common_init", hd_common_init(0))) return 4;
    if (require("audiocap_init", hd_audiocap_init())) return 5;
    HD_PATH_ID ctrl = 0;
    if (require("audiocap_open", hd_audiocap_open(0, (HD_OUT_ID)HD_AUDIOCAP_0_CTRL, &ctrl))) return 6;
    HD_AUDIOCAP_DEV_CONFIG cfg = {0}, readback = {0};
    cfg.in_max.sample_rate = 16000;
    cfg.in_max.sample_bit = 16;
    cfg.in_max.mode = 1;
    cfg.in_max.frame_sample = 1024;
    cfg.out_max.sample_rate = 16000;
    cfg.frame_num_max = 10;
    cfg.aec_max = (HD_AUDIOCAP_AEC){1, 0, 99, -60, -60, 1024, 128, 992, HD_AUDIOCAP_LB_CH_LEFT};
    cfg.anr_max = (HD_AUDIOCAP_ANR){1, 30, 160, 9};
    errno = 0;
    recording = 1;
    int result = hd_audiocap_set(ctrl, HD_AUDIOCAP_PARAM_DEV_CONFIG, &cfg);
    int final_errno = errno;
    recording = 0;
    for (unsigned i = 0; i < count; ++i) {
        struct event *e = &events[i];
        printf("ioctl=%u cmd=%08x unit=%08x item=%08x len=%u injected=%d rc=%d errno=%d result_before=%d result_after=%d\n",
               i + 1, e->command, e->unit, e->item, e->length, e->injected,
               e->rc, e->error, e->before, e->after);
    }
    int get_result = hd_audiocap_get(ctrl, HD_AUDIOCAP_PARAM_DEV_CONFIG, &readback);
    printf("SUMMARY fail_at=%u calls=%u HD_RESULT=%d final_errno=%d get_result=%d cache_matches=%d\n",
           fail_at, count, result, final_errno, get_result, !memcmp(&cfg, &readback, sizeof cfg));
    /* No pipeline is started. Cleanup does not certify driver rollback. */
    int cleanup = require("close", hd_audiocap_close(ctrl));
    cleanup |= require("audiocap_uninit", hd_audiocap_uninit());
    cleanup |= require("common_uninit", hd_common_uninit());
    munmap(unreadable, 4096);
    if (cleanup) return 7;
    if (!fail_at) return result || get_result || memcmp(&cfg, &readback, sizeof cfg);
    if (count != fail_at || events[fail_at - 1].rc != -1 ||
        events[fail_at - 1].error != EFAULT || events[fail_at - 1].after != events[fail_at - 1].before)
        return 8;
    for (unsigned i = 0; i + 1 < fail_at; ++i)
        if (events[i].rc || events[i].after) return 9;
    return 0;
}
