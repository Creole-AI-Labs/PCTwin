/* How often does a deleted file's inode number get handed to the next new file?
 * PCTwin must never treat "same inode number" as "same file" without also checking more
 * (birth time, size). This makes 2000 files in a row (create, note the number, delete) and
 * reports how many numbers were seen more than once. It only REPORTS; it never fails the job.
 * Build: cc -o inode_reuse inode_reuse.c   Run: ./inode_reuse [folder]
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <fcntl.h>

#define N 2000

static int cmp(const void *a, const void *b) {
    unsigned long long x = *(const unsigned long long *)a, y = *(const unsigned long long *)b;
    return (x > y) - (x < y);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : (getenv("TMPDIR") ? getenv("TMPDIR") : "/tmp");
    static unsigned long long seen[N];
    int n = 0, back_to_back = 0;
    unsigned long long last = 0;
    for (int i = 0; i < N; i++) {
        char path[1024];
        snprintf(path, sizeof path, "%s/pctwin-inode-probe-%d", dir, (int)getpid());
        int fd = open(path, O_CREAT | O_EXCL | O_WRONLY, 0600);
        if (fd < 0) { perror("open"); break; }
        struct stat st;
        if (fstat(fd, &st) != 0) { perror("fstat"); close(fd); unlink(path); break; }
        if (i > 0 && (unsigned long long)st.st_ino == last) back_to_back++;
        last = st.st_ino;
        seen[n++] = last;
        close(fd);
        unlink(path);
    }
    qsort(seen, n, sizeof seen[0], cmp);
    int distinct = 0, repeated_numbers = 0;
    for (int i = 0; i < n; i++) {
        if (i == 0 || seen[i] != seen[i - 1]) distinct++;
        else if (i == 1 || seen[i - 1] != seen[i - 2]) repeated_numbers++;
    }
    printf("files made: %d, different inode numbers: %d, numbers that came back: %d, "
           "same number as the file just before: %d\n", n, distinct, repeated_numbers, back_to_back);
    return 0;
}
