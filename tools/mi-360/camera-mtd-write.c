/* Narrow SPI NOR writer for the exact chuangmi.camera.046a01 layout.
 * Only inactive KERNEL/USRFS partitions may be erased. No BOOT/DATA access.
 * MTD ABI: Linux include/uapi/mtd/mtd-abi.h (GPL-2.0 WITH Linux-syscall-note).
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <unistd.h>

struct mtd_info_user {
    uint8_t type;
    uint32_t flags, size, erasesize, writesize, oobsize;
    uint64_t padding;
};
struct erase_info_user { uint32_t start, length; };
#define MEMGETINFO _IOR('M', 1, struct mtd_info_user)
#define MEMERASE _IOW('M', 2, struct erase_info_user)
static const char *names[] = {"BOOT", "ROOTFS", "A_KERNEL", "A_USRFS", "B_KERNEL", "B_USRFS", "DATA", "SOUND", "FACTORY"};
static const uint32_t sizes[] = {0x50000, 0x280000, 0x1b0000, 0x470000, 0x1b0000, 0x470000, 0x40000, 0xa0000, 0x10000};

static void die(const char *message) { fprintf(stderr, "%s: %s\n", message, strerror(errno)); exit(1); }
static void refuse(const char *message) { fprintf(stderr, "%s\n", message); exit(1); }
static uint32_t be32(const unsigned char *p) { return (uint32_t)p[0]<<24 | (uint32_t)p[1]<<16 | (uint32_t)p[2]<<8 | p[3]; }
static uint32_t le32(const unsigned char *p) { return (uint32_t)p[3]<<24 | (uint32_t)p[2]<<16 | (uint32_t)p[1]<<8 | p[0]; }
static uint64_t le64(const unsigned char *p) { return (uint64_t)le32(p+4)<<32 | le32(p); }
static uint32_t crc32(const unsigned char *p, size_t n) {
    uint32_t crc = ~0u;
    while (n--) {
        crc ^= *p++;
        for (int bit=0; bit<8; bit++) crc = (crc>>1) ^ (0xedb88320u & (0u-(crc&1)));
    }
    return ~crc;
}
static int validate(const unsigned char *b, size_t size, int kernel) {
    if (kernel) {
        if (size < 64 || be32(b) != 0x27051956 || size > 0x1b0000) return 0;
        uint32_t n = be32(b+12);
        if (!n || n > size-64 || b[28] != 5 || b[29] != 5 || b[30] != 2 || b[31] != 3) return 0;
        unsigned char header[64]; memcpy(header,b,64); memset(header+4,0,4);
        if (crc32(header,64) != be32(b+4) || crc32(b+64,n) != be32(b+24)) return 0;
        for (size_t i=64+n; i<size; i++) if (b[i] != 255) return 0;
    } else {
        if (size < 96 || size > 0x470000 || memcmp(b,"hsqs",4)) return 0;
        /* SquashFS 4.0, XZ, block size 128 KiB, data fits carrier. */
        if (b[20] != 4 || b[21] || b[28] != 4 || b[29] || b[30] || b[31]) return 0;
        if (le32(b+12) != 131072 || le64(b+40) < 96 || le64(b+40) > size) return 0;
    }
    return 1;
}

#ifndef CAMERA_VALIDATION_ONLY
static void read_exact(int fd, void *buf, size_t n) {
    unsigned char *p=buf;
    while (n) { ssize_t r=read(fd,p,n); if (r<0 && errno==EINTR) continue; if (r<=0) refuse("Short input/readback"); p+=r; n-=(size_t)r; }
}
static int target_partition(int kernel) {
    FILE *f=fopen("/proc/mtd","r"); if (!f) die("/proc/mtd");
    char line[256], name[80]; unsigned i, size, erase; unsigned seen=0;
    while (fgets(line,sizeof line,f)) {
        if (sscanf(line,"mtd%u: %x %x \"%79[^\"]\"",&i,&size,&erase,name)!=4) continue;
        if (i>=9 || strcmp(name,names[i]) || size!=sizes[i] || (seen & (1u<<i))) refuse("Unexpected MTD layout");
        if (erase<4096 || erase>65536 || (erase & (erase-1)) || size%erase) refuse("Unsupported erase geometry");
        seen |= 1u<<i;
    }
    fclose(f); if (seen!=511) refuse("Incomplete MTD layout");
    f=fopen("/proc/mounts","r"); if (!f) die("/proc/mounts");
    char device[128], mountpoint[128], type[64]; int active=-1;
    while (fgets(line,sizeof line,f)) {
        if (sscanf(line,"%127s %127s %63s",device,mountpoint,type)!=3) continue;
        if (!strcmp(mountpoint,"/mnt/usr")) {
            if (strcmp(type,"squashfs")) refuse("USRFS is not mounted as SquashFS");
            if (!strcmp(device,"/dev/mtdblock3")) active=0;
            else if (!strcmp(device,"/dev/mtdblock5")) active=1;
            else refuse("Unknown active USRFS device");
        }
    }
    fclose(f); if (active<0) refuse("Cannot identify active slot");
    return (active ? 2 : 4) + (kernel ? 0 : 1);
}
int main(int argc, char **argv) {
    if (argc!=4 || (strcmp(argv[1],"check") && strcmp(argv[1],"write")) ||
            (strcmp(argv[2],"kernel") && strcmp(argv[2],"usrfs"))) {
        fprintf(stderr,"Usage: camera-mtd-write check|write kernel|usrfs IMAGE\n"); return 1;
    }
    int kernel=!strcmp(argv[2],"kernel");
    int input=open(argv[3],O_RDONLY|O_NOFOLLOW); if(input<0) die("open image");
    struct stat st; if(fstat(input,&st)) die("stat image");
    size_t limit=kernel ? 0x1b0000 : 0x470000;
    if (!S_ISREG(st.st_mode) || st.st_size<=0 || (uint64_t)st.st_size>limit) refuse("Image is not a bounded regular file");
    size_t n=(size_t)st.st_size;
    unsigned char *buf=malloc(limit); if(!buf) die("image allocation");
    memset(buf,255,limit); read_exact(input,buf,n); close(input);
    if(!validate(buf,n,kernel)) refuse("Image structure or kernel CRC is invalid");
    if(!strcmp(argv[1],"check")) { puts("Image validation passed"); free(buf); return 0; }
    if(geteuid()!=0) refuse("Root is required");
    int part=target_partition(kernel); char path[32]; snprintf(path,sizeof path,"/dev/mtd%d",part);
    int fd=open(path,O_RDWR|O_SYNC|O_NOFOLLOW); if(fd<0) die("open inactive MTD");
    if(fstat(fd,&st) || !S_ISCHR(st.st_mode)) refuse("MTD is not a character device");
    struct mtd_info_user info; if(ioctl(fd,MEMGETINFO,&info)) die("MEMGETINFO");
    if(info.type!=3 || !(info.flags&0x400) || info.size!=limit || info.erasesize<4096 || info.erasesize>65536 ||
       (info.erasesize & (info.erasesize-1)) || info.size%info.erasesize || info.writesize!=1) refuse("Unsupported SPI NOR geometry");
    printf("Writing inactive %s (%s); active slot is preserved\n",names[part],path); fflush(stdout);
    for(uint32_t pos=0; pos<info.size; pos+=info.erasesize) {
        struct erase_info_user erase={pos,info.erasesize};
        if(ioctl(fd,MEMERASE,&erase)) die("MEMERASE (inactive image incomplete)");
        if(lseek(fd,pos,SEEK_SET)<0) die("seek write");
        size_t left=info.erasesize; unsigned char *p=buf+pos;
        while(left) { ssize_t w=write(fd,p,left); if(w<0 && errno==EINTR) continue; if(w<=0) die("write (inactive image incomplete)"); p+=w; left-=(size_t)w; }
    }
    if(fsync(fd)) die("fsync MTD");
    if(lseek(fd,0,SEEK_SET)<0) die("seek verify");
    unsigned char block[4096];
    for(size_t pos=0; pos<limit; pos+=sizeof block) {
        read_exact(fd,block,sizeof block);
        if(memcmp(block,buf+pos,sizeof block)) refuse("Inactive MTD readback mismatch; do not activate");
    }
    close(fd); free(buf); puts("Complete partition readback matches, including FF padding"); return 0;
}
#endif
