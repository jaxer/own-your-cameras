// Observe the real OS boundary in an isolated test process; never installed on the camera.
#include <dlfcn.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/wait.h>
#include <signal.h>
#include <errno.h>

static void record(const char *event, pid_t target) {
    int saved_errno = errno;
    const char *path = getenv("CAM_TEST_CHILD_TRACE");
    if (path) {
        int fd = open(path, O_WRONLY | O_CREAT | O_APPEND, 0600);
        if (fd >= 0) {
            char line[128];
            int n = snprintf(line, sizeof(line), "%d %s %d\n", getpid(), event, target);
            if (n > 0 && n < (int)sizeof(line)) { (void)write(fd, line, n); }
            close(fd);
        }
    }
    errno = saved_errno;
}

#ifdef __APPLE__
#define TRACE_KILL traced_kill
#define TRACE_WAITPID traced_waitpid
#else
#define TRACE_KILL kill
#define TRACE_WAITPID waitpid
#endif

int TRACE_KILL(pid_t pid, int sig) {
#ifdef __APPLE__
    int (*original)(pid_t, int) = kill;
#else
    int (*original)(pid_t, int) = dlsym(RTLD_NEXT, "kill");
#endif
    if (pid < 0 && sig == SIGKILL) { record("group-kill", -pid); }
    return original(pid, sig);
}
pid_t TRACE_WAITPID(pid_t pid, int *status, int options) {
#ifdef __APPLE__
    pid_t (*original)(pid_t, int *, int) = waitpid;
#else
    pid_t (*original)(pid_t, int *, int) = dlsym(RTLD_NEXT, "waitpid");
#endif
    pid_t result = original(pid, status, options);
    if (result > 0) { record("reaped", result); }
    return result;
}
#ifdef __APPLE__
__attribute__((used, section("__DATA,__interpose")))
static struct { const void *replacement; const void *original; } interpose[] = {
    { (const void *)traced_kill, (const void *)kill },
    { (const void *)traced_waitpid, (const void *)waitpid },
};
#endif
