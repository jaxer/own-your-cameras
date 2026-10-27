/* Isolated production-projection test. No /dev,/proc,network or live flash I/O.
 * Only the explicit syscall edges below are replaced; all writer control flow,
 * geometry/image guards, erase planning and full readback are production code. */
#define _GNU_SOURCE
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
static int test_open(const char *,int,...);
static int test_fstat(int,struct stat *);
static int test_close(int);
static ssize_t test_read(int,void *,size_t);
static ssize_t test_write(int,const void *,size_t);
static off_t test_lseek(int,off_t,int);
static int test_fsync(int);
static int test_ioctl(int,unsigned long,...);
static FILE *test_fopen(const char *,const char *);
static uid_t test_geteuid(void);
#define open test_open
#define fstat test_fstat
#define close test_close
#define read test_read
#define write test_write
#define lseek test_lseek
#define fsync test_fsync
#define ioctl test_ioctl
#define fopen test_fopen
#define geteuid test_geteuid
#define main writer_main
#include "camera-mtd-write.c"
#undef main
#undef open
#undef fstat
#undef close
#undef read
#undef write
#undef lseek
#undef fsync
#undef ioctl
#undef fopen
#undef geteuid
static const char *scenario;
static unsigned char image[128],flash[0x470000];
static size_t input_size,input_at,device_at,limit;
static unsigned image_opens,mtd_opens,erase_calls,write_calls,verify_bytes,sync_calls;
static int mtd_flags,kind_kernel;
static struct mtd_info_user observed;
static int is(const char *value) { return !strcmp(scenario,value); }
static void report(void) {
    fprintf(stderr,"TEST_COUNTERS {\"image_opens\":%u,\"mtd_opens\":%u,\"erase_calls\":%u,"
            "\"write_calls\":%u,\"verify_bytes\":%u,\"sync_calls\":%u,\"readonly\":%s}\n",
            image_opens,mtd_opens,erase_calls,write_calls,verify_bytes,sync_calls,
            (mtd_flags&O_ACCMODE)==O_RDONLY ? "true":"false");
}
static uid_t test_geteuid(void) { return is("unprivileged") ? 1000:0; }
static int test_open(const char *path,int flags,...) {
    assert(flags&O_NOFOLLOW);
    if (!strcmp(path,"image")) { ++image_opens; assert((flags&O_ACCMODE)==O_RDONLY); return 11; }
    const char *target=kind_kernel ? "/dev/mtd2":"/dev/mtd3";
    if(is("active_a")) target=kind_kernel ? "/dev/mtd4":"/dev/mtd5";
    assert(!strcmp(path,target));
    ++mtd_opens; mtd_flags=flags;
    if(is("mtd_open_fail")) {errno=ELOOP;return -1;}
    return 12;
}
static int test_fstat(int fd,struct stat *st) {
    memset(st,0,sizeof(*st));
    if(fd==11) {st->st_mode=S_IFREG|0600;st->st_size=(off_t)input_size;}
    else {assert(fd==12);st->st_mode=(is("mtd_regular") ? S_IFREG:S_IFCHR)|0600;}
    return 0;
}
static int test_close(int fd) {assert(fd==11||fd==12);return 0;}
static FILE *test_fopen(const char *path,const char *mode) {
    assert(!strcmp(mode,"r"));FILE *f=tmpfile();assert(f);
    if(!strcmp(path,"/proc/mtd")) {
        fputs("dev: size erasesize name\n",f);
        for(unsigned i=0;i<9;i++) fprintf(f,"mtd%u: %08x %08x \"%s\"\n",i,sizes[i],
            is("layout_wrong_erase") ? 65536:32768,names[i]);
    } else {
        assert(!strcmp(path,"/proc/mounts"));
        if(is("bad_mount")) fputs("/dev/mtdblock6 /mnt/usr jffs2 ro 0 0\n",f);
        else fprintf(f,"/dev/mtdblock%d /mnt/usr squashfs ro 0 0\n",is("active_a") ? 3:5);
    }
    rewind(f);return f;
}
static int test_ioctl(int fd,unsigned long request,...) {
    assert(fd==12);va_list ap;va_start(ap,request);void *argument=va_arg(ap,void *);va_end(ap);
    if(request==MEMGETINFO) {memcpy(argument,&observed,sizeof(observed));return 0;}
    assert(request==MEMERASE && (mtd_flags&O_ACCMODE)==O_RDWR);
    struct erase_info_user *erase=argument;
    assert(qualified_geometry(&observed,(uint32_t)limit));
    assert(erase->length==32768 && erase->start%32768==0);
    assert(erase->start<limit && erase->length<=limit-erase->start);
    ++erase_calls;memset(flash+erase->start,255,erase->length);return 0;
}
static ssize_t test_read(int fd,void *out,size_t length) {
    if(fd==11) {assert(input_at+length<=input_size);memcpy(out,image+input_at,length);input_at+=length;}
    else {
        assert(fd==12 && device_at+length<=limit);
        if(is("verify_mismatch") && verify_bytes==0) flash[0]^=1;
        memcpy(out,flash+device_at,length);device_at+=length;verify_bytes+=(unsigned)length;
    }
    return (ssize_t)length;
}
static ssize_t test_write(int fd,const void *data,size_t length) {
    assert(fd==12 && (mtd_flags&O_ACCMODE)==O_RDWR && device_at%256==0 && length%256==0);
    if(is("partial_pages") && length>256) length=256;
    assert(device_at+length<=limit);memcpy(flash+device_at,data,length);device_at+=length;++write_calls;
    return (ssize_t)length;
}
static off_t test_lseek(int fd,off_t position,int whence) {
    assert(fd==12 && whence==SEEK_SET && position>=0 && (uint64_t)position<limit);
    device_at=(size_t)position;return position;
}
static int test_fsync(int fd) {
    assert(fd==12);++sync_calls;
    if(is("fsync_einval")) {errno=EINVAL;return -1;}
    if(is("fsync_eio")) {errno=EIO;return -1;}
    if(is("fsync_enospc")) {errno=ENOSPC;return -1;}
    return 0;
}
static void put_be(unsigned char *p,uint32_t n) {p[0]=n>>24;p[1]=n>>16;p[2]=n>>8;p[3]=n;}
static void put_le(unsigned char *p,uint32_t n) {p[0]=n;p[1]=n>>8;p[2]=n>>16;p[3]=n>>24;}
static void make_image(void) {
    memset(image,0,sizeof(image));input_size=96;
    if(kind_kernel) {
        put_be(image,0x27051956);put_be(image+12,32);
        image[28]=5;image[29]=5;image[30]=2;image[31]=3;
        put_be(image+24,crc32(image+64,32));put_be(image+4,crc32(image,64));
    } else {memcpy(image,"hsqs",4);put_le(image+12,131072);image[20]=4;image[28]=4;put_le(image+40,96);}
    if(is("invalid_image")) image[0]^=1;
}
static void guard_matrix(void) {
    struct mtd_info_user info={.type=3,.flags=0xc00,.size=0x1b0000,.erasesize=32768,.writesize=256};
    assert(qualified_geometry(&info,info.size));
    struct erase_info_user plan={123,456};
    assert(erase_plan(&info,info.size,0,&plan) && plan.start==0 && plan.length==32768);
    assert(erase_plan(&info,info.size,info.size-32768,&plan));
    const uint32_t bad[]={1,256,32767,info.size-256,info.size,UINT32_MAX};
    for(unsigned i=0;i<sizeof(bad)/sizeof(*bad);i++) {
        plan=(struct erase_info_user){123,456};
        assert(!erase_plan(&info,info.size,bad[i],&plan) && plan.start==123 && plan.length==456);
    }
    assert(!erase_plan(&info,info.size-1,0,&plan));
    info.erasesize=0;assert(!qualified_geometry(&info,info.size));assert(!erase_plan(&info,info.size,0,&plan));
}
int main(int argc,char **argv) {
    assert(argc==4);scenario=argv[1];kind_kernel=!strcmp(argv[3],"kernel");
    limit=kind_kernel ? 0x1b0000:0x470000;
    observed=(struct mtd_info_user){.type=3,.flags=0xc00,.size=(uint32_t)limit,.erasesize=32768,.writesize=256};
    if(is("guard_matrix")) {guard_matrix();return 0;}
    if(is("wrong_type")) observed.type=4;
    if(is("wrong_flags")) observed.flags=0x400;
    if(is("extra_flags")) observed.flags=0x1c00;
    if(is("wrong_size")) observed.size+=256;
    if(is("wrong_erase")) observed.erasesize=65536;
    if(is("wrong_write1")) observed.writesize=1;
    if(is("wrong_write512")) observed.writesize=512;
    if(is("wrong_oob")) observed.oobsize=1;
    make_image();atexit(report);
    char *args[]={"camera-mtd-write",argv[2],argv[3],"image",NULL};
    return writer_main(!strcmp(argv[2],"info") ? 3:4,args);
}
