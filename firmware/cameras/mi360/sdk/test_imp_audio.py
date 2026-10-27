#!/usr/bin/env python3
"""Offline O32 ABI compile and isolated audio ownership/fault checks.

No target library, microphone, speaker, network, clock waits or device is used.
Production source compiles unchanged for MIPS with all ABI assertions enabled.
Host fixture disables only O32 assertions: pointers/records use host layout in
both source and callbacks, so host tests certify policy/copies, not target ABI.
Each scenario has a fresh process and temporary build outputs. Synthetic memory
and callback failures are isolated fault fixtures, never deployed media.
"""
import argparse
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]

FIXTURE = r'''
#define _Static_assert(...)
#define dlsym audio_test_dlsym
#include "imp_audio.c"
#undef dlsym
#include <assert.h>

static const char *failure, *missing;
static int events, get_calls, sends, releases, disables_ai, disables_ao;
static int get_code, release_code, send_code, frame_kind, free_blocks=25;
static uint32_t pcm[320];
static int mark(const char *name) { ++events; return failure && !strcmp(name,failure) ? -55 : 0; }
static void attr(IMPAudioIOAttr *a) {
 assert(a->samplerate==16000 && a->bitwidth==16 && a->soundmode==1);
 assert(a->frmNum==25 && a->numPerFrm==640 && a->chnCnt==1);
}
int IMP_AI_SetPubAttr(int d, IMPAudioIOAttr *a) { assert(d==1); attr(a); return mark(__func__); }
int IMP_AO_SetPubAttr(int d, IMPAudioIOAttr *a) { assert(d==0); attr(a); return mark(__func__); }
int IMP_AI_Enable(int d) { assert(d==1); return mark(__func__); }
int IMP_AO_Enable(int d) { assert(d==0); return mark(__func__); }
int IMP_AI_Disable(int d) { assert(d==1); assert(!audio_owner->ai_ch && !audio_owner->ai_ch_unknown); return mark(__func__); }
int IMP_AO_Disable(int d) { assert(d==0); assert(!audio_owner->ao_ch && !audio_owner->ao_ch_unknown); return mark(__func__); }
int IMP_AI_SetChnParam(int d,int c,IMPAudioIChnParam *p) { assert(d==1 && c==0 && p->usrFrmDepth==25 && p->Rev==0); return mark(__func__); }
int IMP_AI_EnableChn(int d,int c) { assert(d==1 && c==0); return mark(__func__); }
int IMP_AO_EnableChn(int d,int c) { assert(d==0 && c==0); return mark(__func__); }
int IMP_AI_DisableChn(int d,int c) {
 assert(d==1 && c==0 && !audio_owner->borrow_unknown && !audio_owner->aec && !audio_owner->aec_unknown);
 ++disables_ai; return mark(__func__);
}
int IMP_AO_DisableChn(int d,int c) {
 assert(d==0 && c==0 && !audio_owner->aec && !audio_owner->aec_unknown);
 ++disables_ao; return mark(__func__);
}
int IMP_AI_SetVol(int d,int c,int v) { assert(d==1 && c==0 && v==55); return mark(__func__); }
int IMP_AI_SetGain(int d,int c,int v) { assert(d==1 && c==0 && v==20); return mark(__func__); }
int IMP_AO_SetVol(int d,int c,int v) { assert(d==0 && c==0 && v==54); return mark(__func__); }
int IMP_AO_SetGain(int d,int c,int v) { assert(d==0 && c==0 && v==28); return mark(__func__); }
int IMP_AI_EnableNs(IMPAudioIOAttr *a,int mode) { attr(a); assert(mode==2); return mark(__func__); }
int IMP_AI_DisableNs(void) { return mark(__func__); }
int IMP_AO_EnableAgc(IMPAudioIOAttr *a,IMPAudioAgcConfig config) {
 attr(a); assert(config.TargetLevelDbfs==6 && config.CompressionGaindB==15); return mark(__func__);
}
int IMP_AO_DisableAgc(void) { return mark(__func__); }
int IMP_AI_EnableAec(int a,int b,int c,int d) { assert(a==1 && b==0 && c==0 && d==0); return mark(__func__); }
int IMP_AI_DisableAec(int a,int b) { assert(a==1 && b==0); return mark(__func__); }
int IMP_AI_GetFrame(int d,int c,IMPAudioFrame *f,IMPBlock b) {
 assert(d==1 && c==0 && b==NOBLOCK); ++get_calls;
 if (get_code) { if(frame_kind==6) f->seq=123; return get_code; }
 memset(f,0,sizeof(*f)); f->bitwidth=16; f->soundmode=1;
 f->virAddr=pcm; f->len=1280; f->timeStamp=987654321;
 if(frame_kind==1) f->len=1281;
 if(frame_kind==2) f->bitwidth=8;
 if(frame_kind==3) f->virAddr=NULL;
 if(frame_kind==4) f->virAddr=(uint32_t *)((uint8_t *)pcm+1);
 if(frame_kind==5) f->virAddr=(uint32_t *)(UINTPTR_MAX & ~(uintptr_t)3);
 return 0;
}
int IMP_AI_ReleaseFrame(int d,int c,IMPAudioFrame *f) {
 assert(d==1 && c==0 && f==&audio_owner->borrowed && f->virAddr==pcm);
 assert(f->timeStamp==987654321); ++releases;
 f->timeStamp=0; /* prove caller saved metadata before releasing the original */
 return release_code;
}
int IMP_AO_QueryChnStat(int d,int c,IMPAudioOChnState *s) {
 assert(d==0 && c==0); s->chnTotalNum=25; s->chnFreeNum=free_blocks;
 s->chnBusyNum=25-free_blocks; return mark(__func__);
}
int IMP_AO_SendFrame(int d,int c,IMPAudioFrame *f,IMPBlock b) {
 assert(d==0 && c==0 && b==NOBLOCK && f->len==1280 && f->bitwidth==16 && f->soundmode==1);
 assert(f->virAddr==audio_owner->playback && !f->phyAddr && !f->timeStamp && !f->seq);
 assert(!memcmp(f->virAddr,pcm,1280)); ++sends; return send_code;
}
void *audio_test_dlsym(void *library,const char *name) {
 assert(library==(void *)1);
 if(missing && !strcmp(missing,name)) return NULL;
#define FIND(n) if(!strcmp(name,#n)) return (void *)&n;
 CORE(FIND) EFFECTS(FIND)
#undef FIND
 assert(!"unexpected SDK import"); return NULL;
}
static void *opened(unsigned effects) {
 void *h=NULL; char e[512];
 assert(t31_audio_open(&h,(void *)1,effects ? (void *)2 : NULL,effects,e,sizeof(e))==0);
 assert(h && h==audio_owner); return h;
}
int main(int argc,char **argv) {
 assert(argc==2); const char *test=argv[1]; char error[1024]; void *h=NULL;
 uint8_t output[1288]; uint32_t length=44; int64_t stamp=44;
 memset(pcm,0x53,sizeof(pcm)); memset(output,0xa6,sizeof(output));
 if(!strcmp(test,"missing")) {
  missing="IMP_AI_ReleaseFrame";
  assert(t31_audio_open(&h,(void *)1,NULL,0,error,sizeof(error))==-1 && !h && !events && !audio_owner);
 } else if(!strcmp(test,"dsp_required")) {
  assert(t31_audio_open(&h,(void *)1,NULL,1,error,sizeof(error))==-1 && !h && !events);
 } else if(!strncmp(test,"openfail:",9)) {
  failure=test+9; int rc=t31_audio_open(&h,(void *)1,(void *)2,3,error,sizeof(error));
  assert(rc<0 && !h);
  bool acquisition=strstr(failure,"Enable")!=NULL;
  assert(acquisition ? rc==-2 && audio_poisoned : rc==-1 && !audio_owner);
 } else if(!strncmp(test,"closefail:",10)) {
  h=opened(7); assert(t31_audio_aec(h,1,error,sizeof(error))==0);
  failure=test+10;
  assert(t31_audio_close(&h,error,sizeof(error))==-2 && !h && audio_poisoned);
  if(!strcmp(failure,"IMP_AI_DisableAec")) assert(!disables_ai && !disables_ao);
  if(!strcmp(failure,"IMP_AI_DisableNs")) assert(!disables_ai && disables_ao==1);
  if(!strcmp(failure,"IMP_AO_DisableAgc")) assert(disables_ai==1 && !disables_ao);
 } else if(!strcmp(test,"effects")) {
  h=opened(7); assert(!audio_owner->aec);
  assert(t31_audio_aec(h,1,error,sizeof(error))==0);
  int before=events; assert(t31_audio_aec(h,1,error,sizeof(error))==0 && before==events);
  assert(t31_audio_close(&h,error,sizeof(error))==0 && !h && !audio_owner);
 } else if(!strcmp(test,"aec_fail")) {
  h=opened(4); failure="IMP_AI_EnableAec";
  assert(t31_audio_aec(h,1,error,sizeof(error))==-2);
  assert(t31_audio_close(&h,error,sizeof(error))==-2 && !disables_ai && !disables_ao);
 } else {
  h=opened(0);
  if(!strcmp(test,"empty")) {
   get_code=-1; assert(t31_audio_next(h,output,1280,&length,&stamp,error,sizeof(error))==1);
   assert(!length && !stamp && !releases);
  } else if(!strcmp(test,"capture")) {
   assert(t31_audio_next(h,output,1280,&length,&stamp,error,sizeof(error))==0);
   assert(length==1280 && stamp==987654321 && releases==1 && !memcmp(output,pcm,1280));
   for(unsigned i=1280;i<sizeof(output);++i) assert(output[i]==0xa6);
  } else if(!strcmp(test,"capacity")) {
   assert(t31_audio_next(h,output,1279,&length,&stamp,error,sizeof(error))==-1 && !get_calls);
  } else if(!strncmp(test,"frame:",6)) {
   frame_kind=atoi(test+6);
   int rc=t31_audio_next(h,output,1280,&length,&stamp,error,sizeof(error));
   assert(rc==(frame_kind>=3 ? -2 : -1) && !length && !stamp);
   assert(releases==(frame_kind>=3 ? 0 : 1));
   for(unsigned i=0;i<sizeof(output);++i) assert(output[i]==0xa6);
  } else if(!strcmp(test,"partial_error")) {
   get_code=-1; frame_kind=6;
   assert(t31_audio_next(h,output,1280,&length,&stamp,error,sizeof(error))==-2 && !releases);
  } else if(!strcmp(test,"release_error")) {
   release_code=-7;
   assert(t31_audio_next(h,output,1280,&length,&stamp,error,sizeof(error))==-2);
   assert(!length && !stamp && releases==1);
  } else if(!strcmp(test,"play")) {
   assert(t31_audio_play(h,(uint8_t *)pcm,1280,error,sizeof(error))==0 && sends==1);
  } else if(!strcmp(test,"partial_play")) {
   int before=events;
   assert(t31_audio_play(h,(uint8_t *)pcm,1278,error,sizeof(error))==-1 && !sends && events==before);
  } else if(!strcmp(test,"full")) {
   free_blocks=0; assert(t31_audio_play(h,(uint8_t *)pcm,1280,error,sizeof(error))==1 && !sends);
  } else if(!strcmp(test,"send_error")) {
   send_code=-1; assert(t31_audio_play(h,(uint8_t *)pcm,1280,error,sizeof(error))==-1 && sends==1);
   assert(t31_audio_play(h,(uint8_t *)pcm,1280,error,sizeof(error))==-1 && sends==1);
  } else if(!strcmp(test,"bad_queue")) {
   free_blocks=26; assert(t31_audio_play(h,(uint8_t *)pcm,1280,error,sizeof(error))==-1 && !sends);
  } else if(!strcmp(test,"gate")) {
   audio_gate=1;
   assert(t31_audio_next(h,output,1280,&length,&stamp,error,sizeof(error))==-1 && !get_calls && length==44 && stamp==44);
   audio_gate=0;
  } else if(!strcmp(test,"open_twice")) {
   void *original=h; assert(t31_audio_open(&h,(void *)1,NULL,0,error,sizeof(error))==-1 && h==original);
  } else assert(!"unknown scenario");
  bool poisoned=audio_poisoned;
  int count=releases;
  assert(t31_audio_close(&h,error,sizeof(error))==(poisoned ? -2 : 0) && !h);
  assert(count==releases); /* cleanup must never release the same SDK frame again */
  assert(t31_audio_close(&h,error,sizeof(error))==-1);
 }
 if(audio_owner) { /* Only mock resources remain. Production intentionally retains them. */
  assert(audio_poisoned); void *other=NULL;
  assert(t31_audio_open(&other,(void *)1,NULL,0,error,sizeof(error))==-2 && !other);
  free(audio_owner); audio_owner=NULL;
 }
 return 0;
}
'''

SCENARIOS = ["missing", "dsp_required", "effects", "aec_fail", "empty", "capture", "capacity",
             "frame:1", "frame:2", "frame:3", "frame:4", "frame:5", "partial_error", "release_error",
             "play", "partial_play", "full", "send_error", "bad_queue", "gate", "open_twice"]
SCENARIOS += ["openfail:" + name for name in [
    "IMP_AI_SetPubAttr", "IMP_AI_Enable", "IMP_AI_SetChnParam", "IMP_AI_EnableChn",
    "IMP_AI_SetVol", "IMP_AI_SetGain", "IMP_AI_EnableNs", "IMP_AO_SetPubAttr", "IMP_AO_Enable",
    "IMP_AO_EnableChn", "IMP_AO_SetVol", "IMP_AO_SetGain", "IMP_AO_EnableAgc"]]
SCENARIOS += ["closefail:" + name for name in [
    "IMP_AI_DisableAec", "IMP_AI_DisableNs", "IMP_AO_DisableAgc", "IMP_AI_DisableChn",
    "IMP_AO_DisableChn", "IMP_AI_Disable", "IMP_AO_Disable"]]


def run(command):
    subprocess.run([str(part) for part in command], cwd=ROOT, check=True, timeout=90)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--headers", type=Path)
    args = parser.parse_args()
    configured = os.environ.get("MI360_HEADERS") or os.environ.get("MI360_HEADER_SOURCE")
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    source = (args.headers or (Path(configured) if configured else ROOT / "sdk")).resolve()
    with tempfile.TemporaryDirectory(prefix="mi360-audio-test-") as directory:
        temporary = Path(directory)
        headers = temporary / "headers"
        fetch = [sys.executable, ROOT / "scripts/headers.py", "--out", headers]
        if source.is_dir():
            fetch += ["--from-directory", source]
        elif args.headers or configured:
            raise SystemExit(f"Specified SDK headers missing: {source}")
        run(fetch)
        common = ["-std=c11", "-Wall", "-Wextra", "-Werror", "-Wno-comment"]
        run(["zig", "cc", "-target", "mipsel-linux-gnueabihf", "-mcpu=mips32r2", *common,
             "-O2", "-fPIC", "-I", ROOT / "sdk/include-compat", "-I", headers,
             "-c", ROOT / "sdk/imp_audio.c", "-o", temporary / "audio.o"])
        fixture = temporary / "audio-test.c"
        fixture.write_text(FIXTURE)
        executable = temporary / "audio-test"
        run([*shlex.split(os.environ.get("CC", "clang")), *common, "-Wno-keyword-macro",
             "-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-g",
             "-I", ROOT / "sdk", "-I", headers, fixture, "-o", executable])
        for scenario in SCENARIOS:
            run([executable, scenario])
        print(f"PASS: production O32 assertions and {len(SCENARIOS)} isolated ASan/UBSan audio scenarios")


if __name__ == "__main__":
    main()
