/* What can this macOS drive do? Run on a FAT32 or exFAT disk image (and APFS to compare).
 * This only REPORTS. It never fails the CI job.
 *
 * Build: cc -o drive_caps drive_caps.c
 * Run:   ./drive_caps <folder> before   (makes its files, sets marks, prints what it sees)
 *        ./drive_caps <folder> after    (after the drive is ejected and attached again:
 *                                        re-reads the same files and prints what survived)
 *
 * Every line starts with "cap:" so the CI log can be read with grep.
 */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/attr.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <time.h>
#include <unistd.h>

static char dir[1024];

static const char *at(const char *name) {
    static char buf[4][1200];
    static int i;
    i = (i + 1) % 4;
    snprintf(buf[i], sizeof buf[i], "%s/%s", dir, name);
    return buf[i];
}

static const char *err(int rc) { return rc == 0 ? "ok" : strerror(errno); }

static void make(const char *name, const char *content) {
    int fd = open(at(name), O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) { write(fd, content, strlen(content)); close(fd); }
}

static void slurp(const char *name, char *out, size_t n) {
    out[0] = 0;
    int fd = open(at(name), O_RDONLY);
    if (fd < 0) { snprintf(out, n, "<missing>"); return; }
    ssize_t r = read(fd, out, n - 1);
    out[r > 0 ? r : 0] = 0;
    close(fd);
}

/* The volume's own claims: which rename flags it says it supports, hard links, hidden files. */
static void volume(void) {
    struct statfs sf;
    if (statfs(dir, &sf) == 0)
        printf("cap: f_fstypename=%s f_fsid=%08x:%08x mounted-on=%s\n", sf.f_fstypename,
               (unsigned)sf.f_fsid.val[0], (unsigned)sf.f_fsid.val[1], sf.f_mntonname);
    struct statvfs sv;
    if (statvfs(dir, &sv) == 0) printf("cap: statvfs f_fsid=%lx\n", (unsigned long)sv.f_fsid);

    struct attrlist al = {.bitmapcount = ATTR_BIT_MAP_COUNT,
                          .volattr = ATTR_VOL_INFO | ATTR_VOL_CAPABILITIES};
    struct { uint32_t len; vol_capabilities_attr_t caps; } __attribute__((packed)) buf;
    if (getattrlist(dir, &al, &buf, sizeof buf, 0) != 0) {
        printf("cap: getattrlist(ATTR_VOL_CAPABILITIES) failed: %s\n", strerror(errno));
        return;
    }
    uint32_t vi = buf.caps.valid[VOL_CAPABILITIES_INTERFACES];
    uint32_t ci = buf.caps.capabilities[VOL_CAPABILITIES_INTERFACES];
    uint32_t vf = buf.caps.valid[VOL_CAPABILITIES_FORMAT];
    uint32_t cf = buf.caps.capabilities[VOL_CAPABILITIES_FORMAT];
#define BIT(name, v, c, b) \
    printf("cap: %s valid=%d capable=%d\n", name, !!((v) & (b)), !!((c) & (b)))
    BIT("VOL_CAP_INT_RENAME_EXCL", vi, ci, VOL_CAP_INT_RENAME_EXCL);
    BIT("VOL_CAP_INT_RENAME_SWAP", vi, ci, VOL_CAP_INT_RENAME_SWAP);
    BIT("VOL_CAP_INT_EXCHANGEDATA", vi, ci, VOL_CAP_INT_EXCHANGEDATA);
    BIT("VOL_CAP_FMT_HARDLINKS", vf, cf, VOL_CAP_FMT_HARDLINKS);
    BIT("VOL_CAP_FMT_HIDDEN_FILES", vf, cf, VOL_CAP_FMT_HIDDEN_FILES);
    BIT("VOL_CAP_FMT_CASE_SENSITIVE", vf, cf, VOL_CAP_FMT_CASE_SENSITIVE);
    BIT("VOL_CAP_FMT_2TB_FILESIZE", vf, cf, VOL_CAP_FMT_2TB_FILESIZE);
}

/* What the drive actually does when asked to move or link without replacing. */
static void moves(void) {
    char got[64];
    make("r_a", "A");
    make("r_b", "B");
    unlink(at("r_free"));
    unlink(at("r_link"));
    printf("cap: renameatx_np(RENAME_EXCL) onto an existing name: %s\n",
           err(renameatx_np(AT_FDCWD, at("r_a"), AT_FDCWD, at("r_b"), RENAME_EXCL)));
    slurp("r_b", got, sizeof got);
    printf("cap:   ...then r_b holds \"%s\" (B = untouched)\n", got);
    int rc = renameatx_np(AT_FDCWD, at("r_a"), AT_FDCWD, at("r_free"), RENAME_EXCL);
    printf("cap: renameatx_np(RENAME_EXCL) onto a free name: %s\n", err(rc));
    if (rc == 0) rename(at("r_free"), at("r_a"));
    printf("cap: renameatx_np(RENAME_SWAP): %s\n",
           err(renameatx_np(AT_FDCWD, at("r_a"), AT_FDCWD, at("r_b"), RENAME_SWAP)));
    slurp("r_a", got, sizeof got);
    printf("cap:   ...then r_a holds \"%s\" (B = swapped)\n", got);
    make("r_a", "A");
    make("r_b", "B");
    printf("cap: link(): %s\n", err(link(at("r_a"), at("r_link"))));
    int fd = open(at("r_b"), O_WRONLY | O_CREAT | O_EXCL, 0644);
    printf("cap: open(O_CREAT|O_EXCL) on an existing name: %s\n", fd >= 0 ? "ok (BAD)" : strerror(errno));
    if (fd >= 0) close(fd);
    printf("cap: plain rename() onto an existing name: %s\n", err(rename(at("r_a"), at("r_b"))));
    slurp("r_b", got, sizeof got);
    printf("cap:   ...then r_b holds \"%s\" (A = silently replaced)\n", got);
}

/* Creation times to set: 2s-aligned, 10 ms precision, and a recent one with nanoseconds. */
static const struct { const char *file; struct timespec t; int via_fd; } CRT[] = {
    {"crt_a", {981173106, 0}, 0},           /* 2001-02-03 04:05:06.00 UTC */
    {"crt_b", {981173106, 70000000}, 0},    /* 2001-02-03 04:05:06.07 UTC */
    {"crt_c", {1792067697, 123456789}, 0},  /* 2026-10-15 12:34:57.123456789 UTC */
    {"crt_f", {981173106, 70000000}, 1},    /* as crt_b, set through fsetattrlist */
};

static void show_crt(const char *file, struct timespec want) {
    struct attrlist al = {.bitmapcount = ATTR_BIT_MAP_COUNT, .commonattr = ATTR_CMN_CRTIME};
    struct { uint32_t len; struct timespec t; } __attribute__((packed)) buf;
    struct stat st;
    if (getattrlist(at(file), &al, &buf, sizeof buf, 0) != 0 || stat(at(file), &st) != 0) {
        printf("cap: crtime %s: cannot read: %s\n", file, strerror(errno));
        return;
    }
    long long d = ((long long)buf.t.tv_sec - want.tv_sec) * 1000000000LL + (buf.t.tv_nsec - want.tv_nsec);
    printf("cap: crtime %s set=%ld.%09ld got=%ld.%09ld birthtime=%ld.%09ld diff_ns=%lld %s\n", file,
           (long)want.tv_sec, want.tv_nsec, (long)buf.t.tv_sec, buf.t.tv_nsec,
           (long)st.st_birthtimespec.tv_sec, st.st_birthtimespec.tv_nsec, d, d == 0 ? "EXACT" : "DIFFERS");
}

static void crtimes(int before) {
    for (size_t i = 0; i < sizeof CRT / sizeof CRT[0]; i++) {
        if (before) {
            make(CRT[i].file, "c");
            struct attrlist al = {.bitmapcount = ATTR_BIT_MAP_COUNT, .commonattr = ATTR_CMN_CRTIME};
            struct timespec t = CRT[i].t;
            int rc;
            if (CRT[i].via_fd) {
                int fd = open(at(CRT[i].file), O_RDWR);
                rc = fsetattrlist(fd, &al, &t, sizeof t, 0);
                close(fd);
            } else {
                rc = setattrlist(at(CRT[i].file), &al, &t, sizeof t, 0);
            }
            printf("cap: set crtime %s via %s: %s\n", CRT[i].file,
                   CRT[i].via_fd ? "fsetattrlist" : "setattrlist", err(rc));
        }
        show_crt(CRT[i].file, CRT[i].t);
    }
}

/* The hidden flag, set by name (chflags) and through an open file (fchflags). */
static void hidden(int before) {
    const char *f[] = {"hid_path", "hid_fd"};
    if (before) {
        make(f[0], "h");
        make(f[1], "h");
        printf("cap: chflags(UF_HIDDEN): %s\n", err(chflags(at(f[0]), UF_HIDDEN)));
        int fd = open(at(f[1]), O_RDONLY);
        printf("cap: fchflags(UF_HIDDEN): %s\n", err(fchflags(fd, UF_HIDDEN)));
        close(fd);
    }
    for (int i = 0; i < 2; i++) {
        struct stat st;
        if (stat(at(f[i]), &st) == 0)
            printf("cap: %s st_flags=0x%x hidden=%s\n", f[i], st.st_flags,
                   (st.st_flags & UF_HIDDEN) ? "yes" : "no");
        else
            printf("cap: %s: %s\n", f[i], strerror(errno));
    }
}

/* File identity: does the inode number, device and volume id stay the same across a remount? */
static void identity(int before) {
    struct stat st;
    struct statvfs sv;
    if (stat(at("crt_a"), &st) != 0 || statvfs(dir, &sv) != 0) return;
    printf("cap: identity crt_a ino=%llu dev=%d statvfs_fsid=%lx\n", (unsigned long long)st.st_ino,
           st.st_dev, (unsigned long)sv.f_fsid);
    FILE *s = fopen(at("caps_state.txt"), before ? "w" : "r");
    if (!s) return;
    if (before) {
        fprintf(s, "%llu %d %lx\n", (unsigned long long)st.st_ino, st.st_dev, (unsigned long)sv.f_fsid);
    } else {
        unsigned long long ino; int dev; unsigned long fsid;
        if (fscanf(s, "%llu %d %lx", &ino, &dev, &fsid) == 3)
            printf("cap: identity across remount: ino %s, dev %s, fsid %s\n",
                   ino == st.st_ino ? "same" : "CHANGED", dev == st.st_dev ? "same" : "CHANGED",
                   fsid == sv.f_fsid ? "same" : "CHANGED");
    }
    fclose(s);
}

int main(int argc, char **argv) {
    if (argc < 3) { printf("cap: usage: drive_caps <folder> before|after\n"); return 0; }
    int before = strcmp(argv[2], "before") == 0;
    snprintf(dir, sizeof dir, "%s/pctwin-caps", argv[1]);
    if (before) mkdir(dir, 0755);
    printf("cap: ==== %s (%s)\n", argv[1], argv[2]);
    volume();
    if (before) moves();
    hidden(before);
    crtimes(before);
    identity(before);
    return 0;
}
