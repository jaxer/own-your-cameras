#!/usr/bin/env python3
"""Offline pinned O32 compilation and scalar-control fault fixtures.

Uses only synthetic process-local records and callbacks. No SDK DSO, device,
network, microphone or speaker is opened. Host tests disable only the pointer
width assertion; unchanged production source also compiles for O32.
"""
import argparse
import os
from pathlib import Path
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = r'''
#define T31_CONTROLS_HOST_TEST
#define dlsym controls_test_dlsym
#define dlerror controls_test_dlerror
#include "imp_controls.c"
#undef dlsym
#undef dlerror
#include <assert.h>
#include <stdlib.h>

static int image[6]={0,128,128,128,128,0}, volume[2]={55,54};
static IMPEncoderFrmRate fps[2]={{20,1},{20,1}}, queued_fps[2];
static IMPEncoderAttrRcMode rc[2];
static IMPEncoderGopAttr gop[2], queued_gop[2];
static int getters,setters,lookups, fail_get,fail_set,fail_after,hold,reenter;
static const char *missing,*loader_fault;
static int callback(void) {
 ++getters;
 if(reenter) {
  int32_t out=991; char error[64]; reenter=0;
  assert(t31_controls_get((void*)1,1,&out,error,sizeof(error))==-1 && out==991);
 }
 return fail_get || (fail_after && getters>=fail_after) ? -55 : 0;
}
#define BYTE(name,index) \
 int IMP_ISP_Tuning_Get##name(unsigned char *v) { *v=(unsigned char)image[index]; return callback(); } \
 int IMP_ISP_Tuning_Set##name(unsigned char v) { ++setters; if(!hold) image[index]=v; return fail_set ? -55:0; }
BYTE(Brightness,1) BYTE(Contrast,2) BYTE(Saturation,3) BYTE(Sharpness,4)
int IMP_ISP_Tuning_GetHVFlip(IMPISPHVFLIP *v) { *v=(IMPISPHVFLIP)image[0]; return callback(); }
int IMP_ISP_Tuning_SetHVFLIP(IMPISPHVFLIP v) { ++setters; if(!hold) image[0]=v; return fail_set ? -55:0; }
int IMP_ISP_Tuning_GetAntiFlickerAttr(IMPISPAntiflickerAttr *v) { *v=(IMPISPAntiflickerAttr)image[5]; return callback(); }
int IMP_ISP_Tuning_SetAntiFlickerAttr(IMPISPAntiflickerAttr v) { ++setters; if(!hold) image[5]=v; return fail_set ? -55:0; }
#define AUDIO(prefix,index,device) \
 int IMP_##prefix##_GetVol(int d,int c,int *v) { assert(d==device && c==0); *v=volume[index]; return callback(); } \
 int IMP_##prefix##_SetVol(int d,int c,int v) { assert(d==device && c==0); ++setters; if(!hold) volume[index]=v; return fail_set ? -55:0; } \
 int IMP_##prefix##_SetVolMute(int d,int c,int v) { assert(d==device && c==0 && (v==0||v==1)); ++setters; return fail_set ? -55:0; }
AUDIO(AI,0,1) AUDIO(AO,1,0)
static void channel(int c) { assert(c==0 || c==1); }
int IMP_Encoder_GetChnFrmRate(int c,IMPEncoderFrmRate *a) { channel(c); *a=fps[c]; return callback(); }
int IMP_Encoder_SetChnFrmRate(int c,const IMPEncoderFrmRate *a) {
 channel(c); assert(a->frmRateDen==1); ++setters; queued_fps[c]=*a;
 if(!hold) fps[c]=*a; return fail_set ? -55:0;
}
int IMP_Encoder_GetChnAttrRcMode(int c,IMPEncoderAttrRcMode *a) { channel(c); *a=rc[c]; return callback(); }
int IMP_Encoder_SetChnBitRate(int c,int target,int max) {
 channel(c); assert(target>0 && target%1000==0 && max==(int)rc[c].attrVbr.uMaxBitRate*1000);
 ++setters; if(!hold) rc[c].attrVbr.uTargetBitRate=(unsigned)(target/1000); return fail_set ? -55:0;
}
int IMP_Encoder_GetChnGopAttr(int c,IMPEncoderGopAttr *a) { channel(c); *a=gop[c]; return callback(); }
int IMP_Encoder_SetChnGopLength(int c,int length) {
 channel(c); assert(length>0 && length<=65535); ++setters; queued_gop[c]=gop[c];
 queued_gop[c].uGopLength=(uint16_t)length; if(!hold) gop[c]=queued_gop[c]; return fail_set ? -55:0;
}
int IMP_Encoder_SetChnGopAttr(int c,const IMPEncoderGopAttr *a) {
 channel(c); assert(a!=&gop[c] && a->uMaxSameSenceCnt>=1 && a->uMaxSameSenceCnt<=3);
 IMPEncoderGopAttr expected=gop[c]; expected.uMaxSameSenceCnt=a->uMaxSameSenceCnt;
 assert(!memcmp(a,&expected,sizeof(expected))); ++setters; queued_gop[c]=*a;
 if(!hold) gop[c]=*a; return fail_set ? -55:0;
}
char *controls_test_dlerror(void) { char *v=(char*)loader_fault; loader_fault=NULL; return v; }
void *controls_test_dlsym(void *sdk,const char *name) {
 assert(sdk==(void*)1); ++lookups;
 if(missing && !strcmp(missing,name)) return NULL;
 if(missing && !strcmp(missing,"loader-error")) loader_fault="fixture lookup failure";
#define FIND(name) if(!strcmp(#name,name_string)) return (void*)&name;
 const char *name_string=name;
 FIND(IMP_ISP_Tuning_GetHVFlip) FIND(IMP_ISP_Tuning_SetHVFLIP)
 FIND(IMP_ISP_Tuning_GetBrightness) FIND(IMP_ISP_Tuning_SetBrightness)
 FIND(IMP_ISP_Tuning_GetContrast) FIND(IMP_ISP_Tuning_SetContrast)
 FIND(IMP_ISP_Tuning_GetSaturation) FIND(IMP_ISP_Tuning_SetSaturation)
 FIND(IMP_ISP_Tuning_GetSharpness) FIND(IMP_ISP_Tuning_SetSharpness)
 FIND(IMP_ISP_Tuning_GetAntiFlickerAttr) FIND(IMP_ISP_Tuning_SetAntiFlickerAttr)
 FIND(IMP_AI_GetVol) FIND(IMP_AI_SetVol) FIND(IMP_AI_SetVolMute)
 FIND(IMP_AO_GetVol) FIND(IMP_AO_SetVol) FIND(IMP_AO_SetVolMute)
 FIND(IMP_Encoder_GetChnFrmRate) FIND(IMP_Encoder_SetChnFrmRate)
 FIND(IMP_Encoder_GetChnAttrRcMode) FIND(IMP_Encoder_SetChnBitRate)
 FIND(IMP_Encoder_GetChnGopAttr) FIND(IMP_Encoder_SetChnGopLength) FIND(IMP_Encoder_SetChnGopAttr)
#undef FIND
 assert(!"unexpected import"); return NULL;
}
static char error[256];
static int read_control(unsigned id,int32_t *out) { return t31_controls_get((void*)1,id,out,error,sizeof(error)); }
static int write_control(unsigned id,int32_t v) { return t31_controls_set((void*)1,id,v,error,sizeof(error)); }
static void read_is(unsigned id,int expected) { int32_t out=991; assert(read_control(id,&out)==0 && out==expected); }
static void read_error(unsigned id,int expected) { int32_t out=991; assert(read_control(id,&out)==expected && out==991); }
int main(int argc,char **argv) {
 assert(argc==2); const char *test=argv[1];
 for(int c=0;c<2;++c) {
  rc[c].rcMode=IMP_ENC_RC_MODE_VBR; rc[c].attrVbr.uTargetBitRate=c?240:1400;
  rc[c].attrVbr.uMaxBitRate=c?300:2400;
  gop[c].uGopCtrlMode=IMP_ENC_GOP_CTRL_MODE_DEFAULT; gop[c].uGopLength=20; gop[c].uMaxSameSenceCnt=3;
 }
 if(!strcmp(test,"happy")) {
  unsigned ids[]={0,1,2,3,4,5,10,11,20,21,22,23,24,25,26,27};
  int values[]={3,0,255,1,254,2,-30,120,10,5,2000,300,3,15,1,2};
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   assert(write_control(ids[i],values[i])==0); read_is(ids[i],values[i]);
   int before=setters; assert(write_control(ids[i],values[i])==0 && setters==before);
  }
 } else if(!strcmp(test,"bounds")) {
  unsigned ids[]={0,1,2,3,4,5,10,11,14,15,20,21,22,23,24,25,26,27};
  int low[]={0,0,0,0,0,0,-30,-30,0,0,1,1,1,1,1,1,1,1};
  int high[]={3,255,255,255,255,2,120,120,1,1,20,20,INT32_MAX/1000,INT32_MAX/1000,15,15,3,3};
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   assert(write_control(ids[i],low[i]-1)==-1); assert(write_control(ids[i],high[i]+1)==-1);
   assert(write_control(ids[i],INT32_MIN)==-1); assert(write_control(ids[i],INT32_MAX)==-1);
  } assert(!getters && !setters && !lookups);
 } else if(!strcmp(test,"unsupported")) {
  unsigned ids[]={6,9,12,13,16,19,28,UINT32_MAX};
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) { read_error(ids[i],-3); assert(write_control(ids[i],1)==-3); }
  read_error(14,-3); read_error(15,-3); assert(!getters && !setters && !lookups);
 } else if(!strcmp(test,"mute")) {
  for(unsigned id=14;id<=15;++id) {
   assert(write_control(id,1)==2 && strstr(error,"no mute readback"));
   assert(write_control(id,0)==2); read_error(id,-3);
  } assert(setters==4 && !getters);
 } else if(!strcmp(test,"missing_set")) {
  missing="IMP_ISP_Tuning_SetBrightness"; assert(write_control(1,12)==-1 && !getters && !setters);
 } else if(!strcmp(test,"missing_get")) {
  missing="IMP_ISP_Tuning_GetBrightness"; assert(write_control(1,12)==-1 && !getters && !setters); read_error(1,-1);
 } else if(!strcmp(test,"loader_error")) {
  missing="loader-error"; read_error(1,-1); assert(!getters && !setters);
 } else if(!strcmp(test,"getter_failure")) {
  fail_get=1; read_error(1,-1); assert(write_control(1,12)==-1 && !setters);
  fail_get=0; read_is(1,128); /* failure released the local gate */
 } else if(!strcmp(test,"setter_failure")) {
  fail_set=1; assert(write_control(1,12)==-1 && setters==1 && getters==1);
  fail_set=0; read_is(1,12); /* uncertain mutation is never rolled back or acknowledged */
 } else if(!strcmp(test,"post_get_failure")) {
  fail_after=2; assert(write_control(1,12)==-1 && setters==1 && getters==2);
 } else if(!strcmp(test,"readback_mismatch")) {
  hold=1; assert(write_control(1,12)==-1 && setters==1 && getters==2); read_is(1,128);
 } else if(!strcmp(test,"pending")) {
  hold=1;
  for(unsigned id=20;id<=25;++id) { if(id==22||id==23) continue;
   int requested=id<22 ? 10:2; assert(write_control(id,requested)==1);
   read_is(id,id<22 ? 20:1); int c=(int)(id&1);
   if(id<22) fps[c]=queued_fps[c]; else gop[c]=queued_gop[c]; read_is(id,requested);
   if(id<22) fps[c]=(IMPEncoderFrmRate){20,1};
  } assert(setters==4);
 } else if(!strcmp(test,"fractional_fps")) {
  fps[0]=(IMPEncoderFrmRate){30000,1001}; read_error(20,-3); assert(write_control(20,10)==-3 && !setters);
 } else if(!strcmp(test,"invalid_fps")) {
  fps[0].frmRateDen=0; read_error(20,-1); fps[0]=(IMPEncoderFrmRate){0,1}; read_error(20,-1);
  fps[0]=(IMPEncoderFrmRate){21,1}; read_error(20,-1); fps[0]=(IMPEncoderFrmRate){UINT32_MAX,1}; read_error(20,-1);
 } else if(!strcmp(test,"normalized_fps")) {
  fps[0]=(IMPEncoderFrmRate){40,2}; read_is(20,20); assert(write_control(20,20)==0 && !setters);
 } else if(!strcmp(test,"bitrate_ceiling")) {
  assert(write_control(22,2401)==-1 && !setters); read_is(22,1400);
  assert(write_control(23,301)==-1 && !setters); assert(write_control(23,300)==0);
  assert(rc[1].attrVbr.uMaxBitRate==300);
 } else if(!strcmp(test,"bitrate_modes")) {
  int modes[]={IMP_ENC_RC_MODE_VBR,IMP_ENC_RC_MODE_CAPPED_VBR,IMP_ENC_RC_MODE_CAPPED_QUALITY};
  for(unsigned i=0;i<sizeof(modes)/sizeof(*modes);++i) {
   rc[0].rcMode=modes[i]; rc[0].attrVbr.uTargetBitRate=1400;
   assert(write_control(22,2000)==0); read_is(22,2000); assert(rc[0].attrVbr.uMaxBitRate==2400);
  }
  rc[0].rcMode=IMP_ENC_RC_MODE_CBR; rc[0].attrCbr.uTargetBitRate=1400; read_is(22,1400);
  int before=setters; assert(write_control(22,2000)==-3 && setters==before);
  rc[0].rcMode=IMP_ENC_RC_MODE_FIXQP; read_error(22,-3); assert(write_control(22,2000)==-3 && setters==before);
 } else if(!strcmp(test,"invalid_bitrate")) {
  rc[0].attrVbr.uTargetBitRate=0; read_error(22,-1);
  rc[0].attrVbr.uTargetBitRate=2401; read_error(22,-1);
  rc[0].attrVbr.uTargetBitRate=UINT32_MAX; read_error(22,-1);
  rc[0].attrVbr.uTargetBitRate=1400; rc[0].attrVbr.uMaxBitRate=UINT32_MAX; read_error(22,-1);
 } else if(!strcmp(test,"gop_graph")) {
  gop[0].uNumB=1; assert(write_control(24,2)==-3 && !setters); gop[0].uNumB=0;
  gop[0].bEnableLT=1; assert(write_control(24,2)==-3 && !setters); gop[0].bEnableLT=0;
  gop[0].bLTRC=1; assert(write_control(24,2)==-3 && !setters); gop[0].bLTRC=0;
  gop[0].uGopCtrlMode=0; assert(write_control(24,2)==-3 && !setters);
  gop[0].uGopCtrlMode=IMP_ENC_GOP_CTRL_MODE_DEFAULT;
  gop[0].uMaxSameSenceCnt=2; read_is(24,1); read_is(26,2);
  gop[0].uMaxSameSenceCnt=0; read_error(24,-3); read_error(26,-3);
  gop[0].uMaxSameSenceCnt=4; read_error(24,-3); read_error(26,-3);
  gop[0].uMaxSameSenceCnt=3; gop[0].uGopLength=0; read_error(24,-1);
 } else if(!strcmp(test,"gop_seconds")) {
  read_is(24,1); assert(write_control(24,2)==0 && gop[0].uGopLength==40); read_is(24,2);
  assert(write_control(24,15)==0 && gop[0].uGopLength==300); read_is(24,15);
  assert(gop[0].uMaxSameSenceCnt==3);
  fps[1]=(IMPEncoderFrmRate){10,1}; gop[1].uGopLength=10;
  assert(write_control(25,2)==0 && gop[1].uGopLength==20); read_is(25,2);
  assert(gop[1].uMaxSameSenceCnt==3);
 } else if(!strcmp(test,"gop_nonintegral")) {
  gop[0].uGopLength=30; read_error(24,-3); assert(write_control(24,2)==-3 && !setters);
  gop[0].uGopLength=20; fps[0]=(IMPEncoderFrmRate){30000,1001};
  read_error(24,-3); assert(write_control(24,2)==-3 && !setters);
  fps[0]=(IMPEncoderFrmRate){20,1}; gop[0].uGopLength=320; read_error(24,-1);
 } else if(!strcmp(test,"gop_fps_faults")) {
  missing="IMP_Encoder_GetChnFrmRate"; read_error(24,-1);
  assert(write_control(24,2)==-1 && !setters); missing=NULL;
  for(int after=2;after<=6;after+=2) {
   gop[0].uGopLength=20; getters=setters=0; fail_after=after;
   assert(write_control(24,2)==-1 && getters==after && setters==(after==6));
  }
  fail_after=0;
 } else if(!strcmp(test,"multiplier")) {
  gop[0].uFreqLT=77; gop[1].uFreqLT=88; /* opaque inactive metadata must survive */
  for(unsigned id=26;id<=27;++id) {
   int c=(int)(id&1); read_is(id,3);
   assert(write_control(id,1)==0); read_is(id,1); read_is(id-2,1);
   assert(gop[c].uGopLength==20 && gop[c].uFreqLT==(c?88U:77U));
   assert(write_control(id-2,2)==0 && gop[c].uMaxSameSenceCnt==1);
   assert(write_control(id,3)==0 && gop[c].uGopLength==40); read_is(id,3);
  }
 } else if(!strcmp(test,"multiplier_pending")) {
  hold=1;
  for(unsigned id=26;id<=27;++id) {
   assert(write_control(id,1)==1); read_is(id,3);
   gop[id&1]=queued_gop[id&1]; read_is(id,1); read_is(id-2,1);
  }
 } else if(!strcmp(test,"multiplier_preflight")) {
  missing="IMP_Encoder_SetChnGopAttr";
  assert(write_control(26,1)==-1 && !getters && !setters); missing=NULL;
  gop[0].uNumB=1; read_error(26,-3); assert(write_control(26,1)==-3 && !setters);
  gop[0].uNumB=0; gop[0].bEnableLT=1; read_error(26,-3);
  gop[0].bEnableLT=0; gop[0].bLTRC=1; read_error(26,-3);
 } else if(!strcmp(test,"invalid_image")) {
  image[0]=4; read_error(0,-1); image[5]=3; read_error(5,-1);
  volume[0]=121; read_error(10,-1); volume[1]=-31; read_error(11,-1);
 } else if(!strcmp(test,"reentry")) {
  reenter=1; read_is(1,128); assert(getters==1); read_is(2,128);
 } else if(!strcmp(test,"all_sdk_faults")) {
  unsigned ids[]={0,1,2,3,4,5,10,11,20,21,22,23,24,25,26,27};
  int values[]={3,0,255,1,254,2,-30,120,10,5,2000,300,3,15,1,2};
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   fail_get=1; read_error(ids[i],-1); int before=setters;
   assert(write_control(ids[i],values[i])==-1 && setters==before);
   fail_get=0; fail_set=1; getters=0;
   assert(write_control(ids[i],values[i])==-1 && setters==before+1);
   assert(getters==(ids[i]>=24 && ids[i]<=25 ? 4:ids[i]>=22 ? 2:1)); /* no post-get after failed mutation */
   fail_set=0;
  }
  for(unsigned id=14;id<=15;++id) {
   fail_set=1; assert(write_control(id,1)==-1); fail_set=0;
  }
 } else if(!strcmp(test,"all_post_get_faults")) {
  unsigned ids[]={0,1,2,3,4,5,10,11,20,21,22,23,24,25,26,27};
  int values[]={3,0,255,1,254,2,-30,120,10,5,2000,300,3,15,1,2};
  for(unsigned i=0;i<sizeof(ids)/sizeof(*ids);++i) {
   getters=0; fail_after=ids[i]>=24 && ids[i]<=25 ? 5:ids[i]>=22 ? 3:2; int before=setters;
   assert(write_control(ids[i],values[i])==-1 && setters==before+1 && getters==fail_after);
   fail_after=0; read_is(ids[i],values[i]);
  }
 } else if(!strcmp(test,"arguments")) {
  int32_t out=991; char guard[3]={'A','B','C'};
  assert(t31_controls_get(NULL,1,&out,error,sizeof(error))==-1 && out==991);
  assert(t31_controls_get((void*)1,1,NULL,error,sizeof(error))==-1);
  assert(t31_controls_get((void*)1,1,&out,NULL,1)==-1 && out==991);
  assert(t31_controls_set(NULL,1,2,error,sizeof(error))==-1);
  assert(t31_controls_set((void*)1,1,2,NULL,1)==-1);
  assert(t31_controls_get((void*)1,1,&out,NULL,0)==0 && out==128);
  assert(t31_controls_set((void*)1,1,2,NULL,0)==0);
  assert(t31_controls_get((void*)1,12,&out,&guard[1],1)==-3 && out==128);
  assert(guard[0]=='A' && guard[1]==0 && guard[2]=='C');
 } else assert(!"unknown scenario");
 assert(!busy); return 0;
}
'''

SCENARIOS = ["happy", "bounds", "unsupported", "mute", "missing_set", "missing_get",
             "loader_error", "getter_failure", "setter_failure", "post_get_failure",
             "readback_mismatch", "pending", "fractional_fps", "invalid_fps", "normalized_fps",
             "bitrate_ceiling", "bitrate_modes", "invalid_bitrate", "gop_graph", "invalid_image",
             "reentry", "all_sdk_faults", "all_post_get_faults", "gop_seconds",
             "gop_nonintegral", "gop_fps_faults", "multiplier", "multiplier_pending",
             "multiplier_preflight", "arguments"]


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
    with tempfile.TemporaryDirectory(prefix="mi360-controls-test-") as directory:
        temporary = Path(directory)
        headers = temporary / "headers"
        run(["python3", ROOT / "scripts/headers.py", "--out", headers, "--from-directory", source])
        common = ["-std=c11", "-Wall", "-Wextra", "-Werror", "-Wno-comment"]
        run(["zig", "cc", "-target", "mipsel-linux-gnueabihf", "-mcpu=mips32r2", *common,
             "-O2", "-fPIC", "-I", ROOT / "sdk/include-compat", "-I", headers,
             "-c", ROOT / "sdk/imp_controls.c", "-o", temporary / "controls.o"])
        fixture = temporary / "controls-test.c"
        fixture.write_text(FIXTURE)
        executable = temporary / "controls-test"
        run([*shlex.split(os.environ.get("CC", "clang")), *common,
             "-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-g",
             "-I", ROOT / "sdk", "-I", ROOT / "sdk/include-compat", "-I", headers,
             fixture, "-o", executable])
        for scenario in SCENARIOS:
            run([executable, scenario])
        print(f"PASS: production O32 assertions and {len(SCENARIOS)} isolated ASan/UBSan controls scenarios")


if __name__ == "__main__":
    main()
