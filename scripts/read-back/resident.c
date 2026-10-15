/* How much of a file sits in the page cache right now (mincore).
   usage: resident <file>   prints: resident_pages=N total_pages=M percent=P */
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc != 2) { fprintf(stderr, "usage: resident <file>\n"); return 2; }
    int fd = open(argv[1], O_RDONLY);
    if (fd < 0) { perror("open"); return 2; }
    struct stat st;
    if (fstat(fd, &st) != 0 || st.st_size == 0) { perror("fstat"); return 2; }
    long page = sysconf(_SC_PAGESIZE);
    size_t pages = ((size_t)st.st_size + page - 1) / page;
    void *map = mmap(NULL, (size_t)st.st_size, PROT_READ, MAP_SHARED, fd, 0);
    if (map == MAP_FAILED) { perror("mmap"); return 2; }
    char *vec = malloc(pages);
    if (!vec || mincore(map, (size_t)st.st_size, (void *)vec) != 0) { perror("mincore"); return 2; }
    size_t resident = 0;
    for (size_t i = 0; i < pages; i++) resident += vec[i] & 1;
    printf("resident_pages=%zu total_pages=%zu percent=%.1f\n", resident, pages,
           100.0 * (double)resident / (double)pages);
    return 0;
}
