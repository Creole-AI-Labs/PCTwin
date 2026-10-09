/* Does this system have file leases (F_SETLEASE / F_GETLEASE)?
 * Linux has them; macOS is expected not to. This only REPORTS. It never fails the CI job.
 * Build: cc -o lease_probe lease_probe.c   Run: ./lease_probe [folder]
 */
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : (getenv("TMPDIR") ? getenv("TMPDIR") : "/tmp");
    char path[1024];
    snprintf(path, sizeof path, "%s/pctwin-lease-probe-XXXXXX", dir);

#ifdef F_SETLEASE
    printf("F_SETLEASE defined in this SDK: yes (%d)\n", F_SETLEASE);
#else
    printf("F_SETLEASE defined in this SDK: no\n");
#endif
#ifdef F_GETLEASE
    printf("F_GETLEASE defined in this SDK: yes (%d)\n", F_GETLEASE);
#else
    printf("F_GETLEASE defined in this SDK: no\n");
#endif

    int fd = mkstemp(path);
    if (fd < 0) { printf("could not make a probe file in %s: %s\n", dir, strerror(errno)); return 0; }

#if defined(F_SETLEASE) && defined(F_WRLCK)
    if (fcntl(fd, F_SETLEASE, F_WRLCK) == 0) {
        printf("fcntl(F_SETLEASE, F_WRLCK) on a fresh file: succeeded\n");
#ifdef F_GETLEASE
        printf("fcntl(F_GETLEASE) then says: %d\n", fcntl(fd, F_GETLEASE));
#endif
    } else {
        printf("fcntl(F_SETLEASE, F_WRLCK) on a fresh file: failed (%s)\n", strerror(errno));
    }
#else
    printf("fcntl(F_SETLEASE, F_WRLCK): not tried (not defined here)\n");
#endif

    close(fd);
    unlink(path);
    return 0;
}
