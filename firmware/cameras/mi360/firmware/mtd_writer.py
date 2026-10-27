#!/usr/bin/env python3
"""Pinned mechanical MTD-writer adaptation; local static build, never device I/O."""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import struct
import subprocess

SOURCE_SHA256 = "082a1a7cbe68c614022bdcdf0b5a9a0f3dd14f01b502f2d36dd85c17125e81c7"
BASE_BINARY_SHA256 = "1487ca86042d3f57c3882a1f5a78cfba4499749addaf6ee8553f09d50587665f"
# Official Zig differs from the historical worker only in a nonloaded compiler comment.
BASE_BINARY_SHA256S = frozenset({BASE_BINARY_SHA256, "67448de0266586f14617c6fefd961566ff5564f152fc9a088cecb28eec25e88d"})
CFLAGS = "-Oz -fstack-protector-strong -ffunction-sections -fdata-sections -fno-unwind-tables -fno-asynchronous-unwind-tables"
LDFLAGS = "-static -s -Wl,--gc-sections -Wl,-z,relro -Wl,-z,now"
TARGET = "mipsel-linux-musleabi"
CPU = "mips32r2"

GEOMETRY = r'''/* Exact qualified vendor NOR, not a general MTD writer. */
#include <stddef.h>
_Static_assert(sizeof(struct mtd_info_user) == 32, "MTD info ABI");
_Static_assert(offsetof(struct mtd_info_user,flags)==4 && offsetof(struct mtd_info_user,size)==8 &&
    offsetof(struct mtd_info_user,erasesize)==12 && offsetof(struct mtd_info_user,writesize)==16 &&
    offsetof(struct mtd_info_user,oobsize)==20 && offsetof(struct mtd_info_user,padding)==24,"MTD info offsets");
_Static_assert(sizeof(struct erase_info_user) == 8, "MTD erase ABI");
#if defined(__mips__)
_Static_assert(MEMGETINFO==0x40204d01U && MEMERASE==0x80084d02U,"MIPS O32 MTD ioctl encoding");
#endif
static int qualified_geometry(const struct mtd_info_user *info, uint32_t limit) {
    return info->type == 3 && info->flags == 0xc00 && info->size == limit &&
        info->erasesize == 32768 && info->writesize == 256 && info->oobsize == 0 &&
        info->size % info->erasesize == 0 && info->erasesize % info->writesize == 0;
}
static int erase_plan(const struct mtd_info_user *info, uint32_t limit,
                       uint32_t position, struct erase_info_user *erase) {
    if (!qualified_geometry(info, limit) || position >= info->size ||
        position % info->erasesize || info->erasesize > info->size - position) return 0;
    *erase = (struct erase_info_user){position, info->erasesize};
    return 1;
}

'''
OPEN_TARGET = r'''/* info/plan open O_RDONLY; only write is allowed to obtain O_RDWR. */
static int open_target(int kernel, int writable, const char *operation, size_t image_bytes,
                       uint32_t limit, char path[32], int *part, struct mtd_info_user *info) {
    if (geteuid() != 0) refuse("Root is required");
    *part = target_partition(kernel);
    snprintf(path,32,"/dev/mtd%d",*part);
    int fd = open(path,(writable ? O_RDWR|O_SYNC : O_RDONLY)|O_NOFOLLOW);
    if (fd < 0) die("open inactive MTD");
    struct stat st;
    if (fstat(fd,&st) || !S_ISCHR(st.st_mode)) refuse("MTD is not a character device");
    memset(info,0,sizeof(*info));
    if (ioctl(fd,MEMGETINFO,info)) die("MEMGETINFO");
    int qualified = qualified_geometry(info,limit);
    printf("{\"operation\":\"%s\",\"partition\":\"%s\",\"device\":\"%s\","
           "\"type\":%u,\"flags\":%u,\"size\":%u,\"erasesize\":%u,"
           "\"writesize\":%u,\"oobsize\":%u,\"image_bytes\":%zu,"
           "\"erase_blocks\":%u,\"qualified\":%s}\n",operation,names[*part],path,
           (unsigned)info->type,info->flags,info->size,info->erasesize,info->writesize,
           info->oobsize,image_bytes,qualified ? info->size/info->erasesize : 0,
           qualified ? "true" : "false");
    fflush(stdout);
    if (!qualified) refuse("Unsupported SPI NOR geometry");
    return fd;
}

'''


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def replace_once(source: str, old: str, new: str) -> str:
    if source.count(old) != 1:
        raise ValueError("pinned MTD-writer projection anchor differs")
    return source.replace(old, new, 1)


def project(original: bytes) -> bytes:
    if sha(original) != SOURCE_SHA256:
        raise ValueError("MTD writer source hash differs")
    source = original.decode()
    source = replace_once(source, "#ifndef CAMERA_VALIDATION_ONLY\n", GEOMETRY + "#ifndef CAMERA_VALIDATION_ONLY\n")
    source = replace_once(source,
        '        if (erase<4096 || erase>65536 || (erase & (erase-1)) || size%erase) refuse("Unsupported erase geometry");',
        '        if (erase!=32768 || size%erase) refuse("Unsupported erase geometry");')
    source = replace_once(source, "int main(int argc, char **argv) {\n", OPEN_TARGET + "int main(int argc, char **argv) {\n")
    source = replace_once(source, '''    if (argc!=4 || (strcmp(argv[1],"check") && strcmp(argv[1],"write")) ||
            (strcmp(argv[2],"kernel") && strcmp(argv[2],"usrfs"))) {
        fprintf(stderr,"Usage: camera-mtd-write check|write kernel|usrfs IMAGE\\n"); return 1;
    }
    int kernel=!strcmp(argv[2],"kernel");''', '''    int inspect = argc==3 && !strcmp(argv[1],"info");
    if ((!inspect && (argc!=4 || (strcmp(argv[1],"check") && strcmp(argv[1],"plan") && strcmp(argv[1],"write")))) ||
            (strcmp(argv[2],"kernel") && strcmp(argv[2],"usrfs"))) {
        fprintf(stderr,"Usage: camera-mtd-write info kernel|usrfs | check|plan|write kernel|usrfs IMAGE\\n"); return 1;
    }
    int kernel=!strcmp(argv[2],"kernel");
    if (inspect) {
        struct mtd_info_user info; char path[32]; int part;
        int fd=open_target(kernel,0,"info",0,kernel ? 0x1b0000 : 0x470000,path,&part,&info);
        close(fd); return 0;
    }''')
    source = replace_once(source, '''    if(geteuid()!=0) refuse("Root is required");
    int part=target_partition(kernel); char path[32]; snprintf(path,sizeof path,"/dev/mtd%d",part);
    int fd=open(path,O_RDWR|O_SYNC|O_NOFOLLOW); if(fd<0) die("open inactive MTD");
    if(fstat(fd,&st) || !S_ISCHR(st.st_mode)) refuse("MTD is not a character device");
    struct mtd_info_user info; if(ioctl(fd,MEMGETINFO,&info)) die("MEMGETINFO");
    if(info.type!=3 || !(info.flags&0x400) || info.size!=limit || info.erasesize<4096 || info.erasesize>65536 ||
       (info.erasesize & (info.erasesize-1)) || info.size%info.erasesize || info.writesize!=1) refuse("Unsupported SPI NOR geometry");''', '''    int readonly=!strcmp(argv[1],"plan");
    int part; char path[32]; struct mtd_info_user info;
    int fd=open_target(kernel,!readonly,readonly ? "plan" : "write",n,(uint32_t)limit,path,&part,&info);
    if(readonly) { close(fd); free(buf); puts("Inactive partition plan validated; no erase or write performed"); return 0; }''')
    source = replace_once(source, '''        struct erase_info_user erase={pos,info.erasesize};''', '''        struct erase_info_user erase;
        if(!erase_plan(&info,(uint32_t)limit,pos,&erase)) refuse("Unaligned or out-of-bounds erase plan");''')
    source = replace_once(source, '    if(fsync(fd)) die("fsync MTD");', r'''    /* Exact B kernel mtd_fops.fsync isNULL; vfs_fsync_range returnsEINVAL.
     * Only that unsupported-operation result is tolerated, never I/O errors.
     * Vendor SFC writes wait for flash BUSY to clear; full readback is mandatory. */
    if(fsync(fd)) {
        if(errno!=EINVAL) die("fsync MTD");
        puts("MTD fsync unsupported by qualified kernel; full readback still required");
    }''')
    return source.encode()


def check_static_mips(data: bytes) -> dict:
    if (len(data)<52 or data[:7]!=b"\x7fELF\x01\x01\x01"
        or struct.unpack_from("<HHI",data,16)!=(2,8,1)
        or struct.unpack_from("<I",data,36)[0]&0xf000f000!=0x70001000):
        raise ValueError("writer must be static MIPS32r2 little-endian O32 ET_EXEC")
    offset=struct.unpack_from("<I",data,28)[0]
    width,count=struct.unpack_from("<HH",data,42)
    if width!=32 or not 1<=count<=64 or offset<52 or offset+width*count>len(data):
        raise ValueError("invalid writer program headers")
    for index in range(count):
        kind,start,_,_,length,*_=struct.unpack_from("<8I",data,offset+index*width)
        if kind in (2,3) or start+length>len(data):
            raise ValueError("writer has dynamic dependencies/interpreter or invalid segment")
    return {"bytes":len(data),"sha256":sha(data)}


def verify_artifact(writer: Path) -> dict:
    report=json.loads(writer.with_name("writer-manifest.json").read_text())
    binary=writer.read_bytes()
    source=writer.with_name("camera-mtd-write.c").read_bytes()
    if (report.get("schema")!=1 or report.get("original_source_sha256")!=SOURCE_SHA256
        or report.get("adapter_sha256")!=sha(Path(__file__).read_bytes())
        or report.get("projected_source_sha256")!=sha(source)
        or report.get("binary")!=check_static_mips(binary)
        or report.get("zig_version")!="0.17.0" or report.get("target")!=TARGET
        or report.get("cpu")!=CPU or report.get("cflags")!=CFLAGS or report.get("ldflags")!=LDFLAGS):
        raise ValueError("writer artifact/provenance differs from current pinned adapter")
    return report


def main() -> None:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source",type=Path,default=Path(__file__).resolve().parents[4]/"tools/mi-360/camera-mtd-write.c")
    parser.add_argument("--updater-source",type=Path,default=Path(__file__).resolve().parents[4]/"tools/mi-360/camera-update")
    parser.add_argument("--output",required=True,type=Path)
    parser.add_argument("--runtime-writer",default="/tmp/mi360-update-tools/camera-mtd-write")
    args=parser.parse_args()
    os.umask(0o077)
    projected=project(args.source.read_bytes())
    if subprocess.check_output(["zig","version"],text=True).strip()!="0.17.0":
        raise ValueError("writer recipe requires Zig0.17.0")
    out=args.output.resolve()
    if out.is_relative_to(Path(__file__).resolve().parents[4]):
        raise ValueError("writer output must be a new private directory outside this repository")
    # Imported only for CLI wiring; build_image imports this module's pure helpers.
    from build_image import camera_update_script
    updater=camera_update_script(args.updater_source.read_bytes(),args.runtime_writer)
    out.mkdir(mode=0o700)
    source=out/"camera-mtd-write.c"; source.write_bytes(projected)
    writer=out/"camera-mtd-write"
    command=["zig","cc","-target",TARGET,"-mcpu="+CPU,*shlex.split(CFLAGS),*shlex.split(LDFLAGS),
             "-Wall","-Wextra","-Werror",str(source),"-o",str(writer)]
    subprocess.run(command,check=True)
    writer.chmod(0o755)
    binary=check_static_mips(writer.read_bytes())
    helper=out/"camera-update";helper.write_bytes(updater);helper.chmod(0o700)
    report={"schema":1,"original_source_sha256":SOURCE_SHA256,
            "adapter_sha256":sha(Path(__file__).read_bytes()),"projected_source_sha256":sha(projected),
            "zig_version":"0.17.0","target":TARGET,"cpu":CPU,"cflags":CFLAGS,"ldflags":LDFLAGS,
            "binary":binary,"ram_helper_sha256":sha(updater),"runtime_writer":args.runtime_writer,
            "command":command,"device_io_performed":False}
    (out/"writer-manifest.json").write_text(json.dumps(report,indent=2)+"\n")
    verify_artifact(writer)
    print(json.dumps({"writer":binary,"ram_helper_sha256":sha(updater),"runtime_writer":args.runtime_writer},indent=2))


if __name__=="__main__":main()
