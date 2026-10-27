#!/usr/bin/env python3
"""Compile production O32 assertions and run isolated ASan/UBSan tuning faults.

No SDK library, hardware, process owner, audio, network or deployed state is used.
Only the host pointer-width assertion is bypassed in the host fixture; the
unchanged production file is also compiled for MIPS/O32 using pinned headers.
"""
import argparse
import os
from pathlib import Path
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = r'''
#define T31_TUNING_HOST_TEST
#define dlsym tuning_test_dlsym
#define dlerror tuning_test_dlerror
#include "imp_tuning.c"
#undef dlsym
#undef dlerror
#include <assert.h>
#include <stdlib.h>

static int values[5], getters, setters, lookups;
static int fail_get, fail_set, fail_after, hold, collateral, reenter, change_gains;
static uint32_t modules, last_module_set;
static IMPISPWB wb;
static IMPISPEVAttr ev;
static int luma;
static const char *missing, *loader_fault;
static int callback(void) {
 ++getters;
 if(reenter) {
  reenter=0; int32_t out=991; char error[64];
  assert(t31_tuning_get((void*)1,30,&out,error,sizeof(error))==-1 && out==991);
  assert(t31_tuning_set((void*)1,30,14,error,sizeof(error))==-1);
 }
 return fail_get || (fail_after && getters>=fail_after) ? -55:0;
}
int IMP_ISP_Tuning_GetBcshHue(unsigned char *v) { *v=(unsigned char)values[0]; return callback(); }
int IMP_ISP_Tuning_SetBcshHue(unsigned char v) {
 ++setters; if(!hold) values[0]=v; return fail_set ? -66:0;
}
int IMP_ISP_Tuning_GetAeComp(int *v) { *v=values[1]; return callback(); }
int IMP_ISP_Tuning_SetAeComp(int v) {
 ++setters; if(!hold) values[1]=v; return fail_set ? -66:0;
}
#define UNSIGNED(name,index) \
 int IMP_ISP_Tuning_Get##name(unsigned int *v) { *v=(unsigned)values[index]; return callback(); } \
 int IMP_ISP_Tuning_Set##name(unsigned int v) { ++setters; if(!hold) values[index]=(int)v; return fail_set ? -66:0; }
UNSIGNED(HiLightDepress,2) UNSIGNED(DPC_Strength,3) UNSIGNED(DRC_Strength,4)
int IMP_ISP_Tuning_GetWB(IMPISPWB *v) { *v=wb; return callback(); }
int IMP_ISP_Tuning_SetWB(IMPISPWB *v) {
 assert(v->rgain==wb.rgain && v->bgain==wb.bgain);
 ++setters;
 if(!hold) { wb=*v; if(change_gains) { wb.rgain=2222; wb.bgain=3333; } }
 return fail_set ? -66:0;
}
int IMP_ISP_Tuning_GetModuleControl(IMPISPModuleCtl *v) { v->key=modules; return callback(); }
int IMP_ISP_Tuning_SetModuleControl(IMPISPModuleCtl *v) {
 ++setters; last_module_set=v->key;
 if(!hold) modules=v->key ^ (collateral ? 1U:0U);
 /* Do not let an SDK in/out mutation change the expected comparison. */
 v->key=0;
 return fail_set ? -66:0;
}
int IMP_ISP_Tuning_GetAeLuma(int *v) { *v=luma; return callback(); }
int IMP_ISP_Tuning_GetEVAttr(IMPISPEVAttr *v) { *v=ev; return callback(); }
char *tuning_test_dlerror(void) { char *v=(char*)loader_fault; loader_fault=NULL; return v; }
void *tuning_test_dlsym(void *sdk,const char *name) {
 assert(sdk==(void*)1); ++lookups;
 if(missing && !strcmp(missing,name)) return NULL;
 if(missing && !strcmp(missing,"loader-error")) loader_fault="isolated loader failure";
#define FIND(symbol) if(!strcmp(name,#symbol)) return (void*)&symbol;
 FIND(IMP_ISP_Tuning_GetBcshHue) FIND(IMP_ISP_Tuning_SetBcshHue)
 FIND(IMP_ISP_Tuning_GetAeComp) FIND(IMP_ISP_Tuning_SetAeComp)
 FIND(IMP_ISP_Tuning_GetHiLightDepress) FIND(IMP_ISP_Tuning_SetHiLightDepress)
 FIND(IMP_ISP_Tuning_GetDPC_Strength) FIND(IMP_ISP_Tuning_SetDPC_Strength)
 FIND(IMP_ISP_Tuning_GetDRC_Strength) FIND(IMP_ISP_Tuning_SetDRC_Strength)
 FIND(IMP_ISP_Tuning_GetWB) FIND(IMP_ISP_Tuning_SetWB)
 FIND(IMP_ISP_Tuning_GetModuleControl) FIND(IMP_ISP_Tuning_SetModuleControl)
 FIND(IMP_ISP_Tuning_GetAeLuma) FIND(IMP_ISP_Tuning_GetEVAttr)
#undef FIND
 /* In particular EnableDRC, lifecycle and raw-fd APIs must never be resolved. */
 assert(!"unexpected SDK import"); return NULL;
}
static char error[256];
static void reset(void) {
 memset(values,0,sizeof(values)); modules=last_module_set=0;
 wb=(IMPISPWB){ISP_CORE_WB_MODE_AUTO,0x1234,0x4321};
 ev=(IMPISPEVAttr){1,40000,2,3,4,5}; luma=118;
 getters=setters=lookups=fail_get=fail_set=fail_after=hold=collateral=reenter=change_gains=0;
 missing=loader_fault=NULL;
}
static int read_control(unsigned id,int32_t *out) { return t31_tuning_get((void*)1,id,out,error,sizeof(error)); }
static int write_control(unsigned id,int32_t value) { return t31_tuning_set((void*)1,id,value,error,sizeof(error)); }
static void read_is(unsigned id,int32_t expected) {
 int32_t out=991; assert(read_control(id,&out)==0 && out==expected);
}
static void read_error(unsigned id,int expected) {
 int32_t out=991; assert(read_control(id,&out)==expected && out==991);
}
static const unsigned ids[]={30,31,32,33,34,35,36,37,38};
static const int requested[]={64,128,5,64,64,2,0,0,0};
static const char *get_names[]={"IMP_ISP_Tuning_GetBcshHue","IMP_ISP_Tuning_GetAeComp",
 "IMP_ISP_Tuning_GetHiLightDepress","IMP_ISP_Tuning_GetDPC_Strength","IMP_ISP_Tuning_GetDRC_Strength",
 "IMP_ISP_Tuning_GetWB","IMP_ISP_Tuning_GetModuleControl","IMP_ISP_Tuning_GetModuleControl","IMP_ISP_Tuning_GetModuleControl"};
static const char *set_names[]={"IMP_ISP_Tuning_SetBcshHue","IMP_ISP_Tuning_SetAeComp",
 "IMP_ISP_Tuning_SetHiLightDepress","IMP_ISP_Tuning_SetDPC_Strength","IMP_ISP_Tuning_SetDRC_Strength",
 "IMP_ISP_Tuning_SetWB","IMP_ISP_Tuning_SetModuleControl","IMP_ISP_Tuning_SetModuleControl","IMP_ISP_Tuning_SetModuleControl"};
int main(int argc,char **argv) {
 assert(argc==2); const char *test=argv[1]; reset();
 if(!strcmp(test,"happy")) {
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   reset(); assert(write_control(ids[i],requested[i])==0 && setters==1 && getters==2);
   read_is(ids[i],requested[i]); int calls=setters;
   assert(write_control(ids[i],requested[i])==0 && setters==calls);
  }
 } else if(!strcmp(test,"bounds")) {
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   int high=i<5 ? (i==2 ? 10:255):i==5 ? 8:1;
   assert(write_control(ids[i],-1)==-1); assert(write_control(ids[i],high+1)==(i==5 ? -3:-1));
   assert(write_control(ids[i],INT32_MIN)==-1); assert(write_control(ids[i],INT32_MAX)==-1);
  }
  assert(write_control(35,1)==-3); assert(write_control(35,9)==-3);
  assert(!getters && !setters && !lookups);
 } else if(!strcmp(test,"unsupported")) {
  unsigned other[]={0,29,39,49,52,UINT32_MAX};
  for(unsigned i=0;i<sizeof(other)/sizeof(*other);++i) {
   read_error(other[i],-3); assert(write_control(other[i],0)==-3);
  }
  assert(write_control(50,0)==-3 && write_control(51,0)==-3);
  assert(!getters && !setters && !lookups);
 } else if(!strcmp(test,"missing_symbols")) {
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   reset(); missing=set_names[i];
   assert(write_control(ids[i],requested[i])==-1 && !getters && !setters);
   reset(); missing=get_names[i]; read_error(ids[i],-1);
   assert(write_control(ids[i],requested[i])==-1 && !getters && !setters);
  }
 } else if(!strcmp(test,"lookup_failures")) {
  missing="loader-error"; read_error(30,-1); assert(!getters && !setters);
  reset(); loader_fault="stale prior dlerror"; read_is(30,0);
 } else if(!strcmp(test,"all_getter_failures")) {
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   reset(); fail_get=1; read_error(ids[i],-1); assert(strstr(error,"-55") && strstr(error,get_names[i]));
   assert(write_control(ids[i],requested[i])==-1 && !setters && getters==2);
   fail_get=0; read_is(ids[i],i>=6 ? 1:0); /* local gate released after fault */
  }
 } else if(!strcmp(test,"all_setter_failures")) {
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   reset(); fail_set=1; assert(write_control(ids[i],requested[i])==-1);
   assert(setters==1 && getters==1 && strstr(error,"-66"));
   fail_set=0; read_is(ids[i],requested[i]); /* mutation is not falsely undone */
  }
 } else if(!strcmp(test,"all_post_get_failures")) {
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   reset(); fail_after=2; assert(write_control(ids[i],requested[i])==-1);
   assert(setters==1 && getters==2 && strstr(error,"-55"));
   fail_after=0; read_is(ids[i],requested[i]);
  }
 } else if(!strcmp(test,"all_mismatches")) {
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   reset(); hold=1; assert(write_control(ids[i],requested[i])==-1);
   assert(setters==1 && getters==2 && strstr(error,"differs"));
  }
 } else if(!strcmp(test,"invalid_readbacks")) {
  values[1]=-1; read_error(31,-1); assert(write_control(31,10)==-1 && !setters);
  values[1]=256; read_error(31,-1);
  for(unsigned id=32;id<=34;++id) {
   reset(); values[id-30]=id==32 ? 11:256; read_error(id,-1);
   assert(write_control(id,10)==-1 && !setters); values[id-30]=-1; read_error(id,-1);
  }
  reset(); wb.mode=(enum isp_core_wb_mode)10; read_error(35,-1);
  assert(write_control(35,2)==-1 && !setters);
  for(int mode=1;mode<=9;mode+=8) {
   wb.mode=(enum isp_core_wb_mode)mode; read_error(35,-3);
   assert(write_control(35,2)==-3 && !setters);
  }
  reset(); modules=1U<<20;
  for(unsigned id=36;id<=38;++id) { read_error(id,-1); assert(write_control(id,0)==-1 && !setters); }
 } else if(!strcmp(test,"wb_presets_and_gains")) {
  int modes[]={2,3,4,5,6,7,8,0}; change_gains=1;
  for(unsigned i=0;i<sizeof(modes)/sizeof(*modes);++i) {
   assert(write_control(35,modes[i])==0); read_is(35,modes[i]);
   assert(wb.rgain==2222 && wb.bgain==3333);
  }
 } else if(!strcmp(test,"module_preservation")) {
  uint32_t patterns[]={0,0x15555,0x8002aaaa,0x8007ffff};
  uint32_t bits[]={1U<<7,1U<<17,1U<<31};
  for(unsigned p=0;p<sizeof(patterns)/sizeof(*patterns);++p) {
   for(unsigned n=0;n<3;++n) {
    reset(); modules=patterns[p]; int desired=(modules&bits[n]) ? 1:0;
    assert(write_control(36+n,desired)==0 && setters==1 && getters==2);
    assert(last_module_set==(patterns[p]^bits[n]) && modules==last_module_set);
    assert((modules&~bits[n])==(patterns[p]&~bits[n])); read_is(36+n,desired);
   }
  }
 } else if(!strcmp(test,"module_collateral_failure")) {
  for(unsigned id=36;id<=38;++id) {
   reset(); collateral=1; assert(write_control(id,0)==-1 && setters==1 && getters==2);
   read_is(id,0); /* target matches, but unrelated-bit damage was refused */
  }
 } else if(!strcmp(test,"statistics")) {
  read_is(50,118); read_is(51,40000);
  luma=INT32_MIN; read_is(50,INT32_MIN); /* no invented luma scale */
  ev.expr_us=0; read_is(51,0); ev.expr_us=INT32_MAX; read_is(51,INT32_MAX);
  ev.expr_us=(uint32_t)INT32_MAX+1U; read_error(51,-1);
  ev.expr_us=UINT32_MAX; read_error(51,-1);
  fail_get=1; read_error(50,-1); read_error(51,-1); assert(!setters);
  reset(); missing="IMP_ISP_Tuning_GetEVAttr"; read_error(51,-1);
  reset(); missing="IMP_ISP_Tuning_GetAeLuma"; read_error(50,-1);
 } else if(!strcmp(test,"reentry")) {
  reenter=1; read_is(30,0); assert(getters==1 && !setters); read_is(31,0);
 } else if(!strcmp(test,"arguments")) {
  int32_t out=991; char guard[]={'A','B','C'};
  assert(t31_tuning_get(NULL,30,&out,error,sizeof(error))==-1 && out==991);
  assert(t31_tuning_get((void*)1,30,NULL,error,sizeof(error))==-1);
  assert(t31_tuning_get((void*)1,30,&out,NULL,1)==-1 && out==991);
  assert(t31_tuning_set(NULL,30,0,error,sizeof(error))==-1);
  assert(t31_tuning_set((void*)1,30,0,NULL,1)==-1);
  assert(t31_tuning_get((void*)1,30,&out,NULL,0)==0 && out==0);
  assert(t31_tuning_set((void*)1,30,1,NULL,0)==0);
  assert(t31_tuning_get((void*)1,29,&out,&guard[1],1)==-3 && out==0);
  assert(guard[0]=='A' && guard[1]==0 && guard[2]=='C');
 } else assert(!"unknown scenario");
 assert(!busy); return 0;
}
'''

SCENARIOS = ["happy", "bounds", "unsupported", "missing_symbols", "lookup_failures",
             "all_getter_failures", "all_setter_failures", "all_post_get_failures",
             "all_mismatches", "invalid_readbacks", "wb_presets_and_gains",
             "module_preservation", "module_collateral_failure", "statistics", "reentry", "arguments"]


def run(command):
    subprocess.run([str(part) for part in command], cwd=ROOT, check=True, timeout=90)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--headers", type=Path)
    args = parser.parse_args()
    configured = os.environ.get("MI360_HEADERS") or os.environ.get("MI360_HEADER_SOURCE")
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    source = args.headers or (Path(configured) if configured else target / "sdk-headers")
    if not source.is_dir():
        raise SystemExit("Pinned SDK headers required locally; supply --headers")
    with tempfile.TemporaryDirectory(prefix="mi360-tuning-test-") as directory:
        temporary = Path(directory)
        headers = temporary / "headers"
        run(["python3", ROOT / "scripts/headers.py", "--out", headers, "--from-directory", source])
        common = ["-std=c11", "-Wall", "-Wextra", "-Werror", "-Wno-comment"]
        run(["zig", "cc", "-target", "mipsel-linux-gnueabihf", "-mcpu=mips32r2", *common,
             "-O2", "-fPIC", "-I", ROOT / "sdk/include-compat", "-I", headers,
             "-c", ROOT / "sdk/imp_tuning.c", "-o", temporary / "tuning.o"])
        fixture = temporary / "tuning-test.c"
        fixture.write_text(FIXTURE)
        executable = temporary / "tuning-test"
        run([*shlex.split(os.environ.get("CC", "clang")), *common,
             "-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-g",
             "-I", ROOT / "sdk", "-I", ROOT / "sdk/include-compat", "-I", headers,
             fixture, "-o", executable])
        for scenario in SCENARIOS:
            run([executable, scenario])
        print(f"PASS: production O32 assertions and {len(SCENARIOS)} isolated ASan/UBSan tuning scenarios")


if __name__ == "__main__":
    main()
