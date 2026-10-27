/* Isolated host algorithm/lifecycle checks. Never load IMP or call hardware.
 * Disable O32 assertions only in this host translation unit; target assertions
 * are separately compiled in the real object. SDK record sizes here are host64.
 * The host fixture header widens only stream.virAddr to uintptr_t; this tests
 * copying/lifecycle logic, not target pointer-width arithmetic or SDK ABI.
 * Synthetic image-base/GOT reads also isolate loader preflight from host mappings;
 * production O32 code reads the real pinned SDK GOT.
 */
#include <stdint.h>
static uintptr_t fake_got_value(uintptr_t base, uint32_t slot);
#define BRIDGE_SDK_IMAGE_BASE(address) ((void)(address), (uintptr_t)0x10000000U)
#define BRIDGE_SDK_GOT_VALUE(base, slot) fake_got_value(base, slot)
#define _Static_assert(...)
/* Loader calls resolve only to the local fake callbacks below. */
#define dlopen test_dlopen
#define dlclose test_dlclose
#define dlsym test_dlsym
#define dlerror test_dlerror
#include "imp_bridge.c"
#undef dlopen
#undef dlclose
#undef dlsym
#undef dlerror
#include <assert.h>

static struct video *test_v;
static const char *fail_op;
static const char *second_fail_op;
static const char *missing_symbol;
static char *loader_message;
static int unloads;
static int fake_library;
static int fake_math;
static int fake_logger;
static int fake_import;
static unsigned loaded;
static unsigned import_checks;
static unsigned got_checks;
static const char *null_got_symbol;
static const char *failed_library;
static const char *failed_unload;
static char load_order[16];
static char close_order[16];
static char events[2048];
static int releases;
static int poll_result;
static int get_result;
static int release_result;
static IMPEncoderStream supplied;
static bool dual_fixture;
static struct t31_video_spec expected_spec = T31_VIDEO_SPEC_DEFAULT;
static int configuring_channel;
static IMPEncoderGopAttr created_gop[2];
static IMPEncoderFrmRate created_fps[2];
static unsigned expected_poll_ms = 100;
static int failed_release_channel = -1;
static int released_channels[2];
static struct video *channel_video(int channel) {
    assert(channel == 0 || (channel == 1 && dual_fixture && test_v->sub != NULL));
    return channel == 0 ? test_v : test_v->sub;
}
static int record(const char *op)
{
    size_t n = strlen(events);
    (void)snprintf(events + n, sizeof(events) - n, "%s;", op);
    if (fail_op != NULL && strcmp(fail_op, op) == 0) return -55;
    if (second_fail_op != NULL && strcmp(second_fail_op, op) == 0) return -66;
    return 0;
}
static int record_channel(int channel, const char *op) {
    if (channel == 0) return record(op);
    (void)channel_video(channel);
    char name[64]; (void)snprintf(name, sizeof(name), "Sub%s", op); return record(name);
}
static int stop(int n) { (void)channel_video(n); return record_channel(n, "Stop"); }
static int disable_fs(int n) { (void)channel_video(n); return record_channel(n, "DisableFS"); }
static int unbind(IMPCell *a, IMPCell *b) {
    struct video *v = channel_video(a->groupID);
    assert(a->deviceID == DEV_ID_FS && b->deviceID == DEV_ID_ENC);
    assert(a->groupID == b->groupID && !v->receiving && !v->fs_enabled && !v->stream_held);
    return record_channel(a->groupID, "Unbind");
}
static int unregister_enc(int n) {
    struct video *v = channel_video(n);
    assert(!v->bound && !v->receiving && !v->stream_held);
    return record_channel(n, "Unregister");
}
static int destroy_enc(int n) {
    struct video *v = channel_video(n);
    assert(!v->encoder_registered && !v->bound && !v->receiving);
    return record_channel(n, "DestroyEnc");
}
static int destroy_group(int n) {
    struct video *v = channel_video(n);
    assert(!v->encoder_created && !v->encoder_registered && !v->bound);
    return record_channel(n, "DestroyGroup");
}
static int destroy_fs(int n) {
    struct video *v = channel_video(n);
    assert(!v->bound && !v->fs_enabled);
    return record_channel(n, "DestroyFS");
}
static int disable_tuning(void) { assert(pipeline_empty(test_v)); return record("DisableTuning"); }
static int disable_sensor(void) { assert(!test_v->tuning_enabled); return record("DisableSensor"); }
static int del_sensor(IMPSensorInfo *s) {
    assert(s == &test_v->sensor && !test_v->sensor_enabled && !test_v->tuning_enabled);
    return record("DelSensor");
}
static int close_isp(void) { assert(!test_v->sensor_added); return record("CloseISP"); }
static int system_exit(void) { assert(!test_v->isp_open); return record("SystemExit"); }
static int poll_stream(int n, uint32_t timeout) {
    (void)channel_video(n); assert(timeout == expected_poll_ms); return poll_result;
}
static int get_stream(int n, IMPEncoderStream *s, bool blocking) {
    assert(s == &channel_video(n)->stream && !blocking); *s = supplied; return get_result;
}
static int put_stream(int n, IMPEncoderStream *s) {
    assert(s == &channel_video(n)->stream && s->pack == supplied.pack);
    ++releases; ++released_channels[n]; return n == failed_release_channel ? -77 : release_result;
}

static int open_isp(void) {
    assert(loaded == 7 && import_checks == 116 && got_checks == 116);
    return record("OpenISP");
}
static int add_sensor(IMPSensorInfo *sensor) {
    assert(sensor == &test_v->sensor);
    assert(strcmp(sensor->name, "sc2336") == 0 && strcmp(sensor->i2c.type, "sc2336") == 0);
    assert(sensor->cbus_type == 1 && sensor->i2c.addr == 0x30 && sensor->i2c.i2c_adapter_id == 0);
    assert(sensor->rst_gpio == 0 && sensor->pwdn_gpio == 0 && sensor->power_gpio == 0);
    return record("AddSensor");
}
static int enable_sensor(void) { return record("EnableSensor"); }
static int init_system(void) { return record("SystemInit"); }
static int enable_tuning(void) { return record("EnableTuning"); }
static int tune_value(unsigned char value) { assert(value == 128); return record("TuningValue"); }
static int day_mode(IMPISPRunningMode mode) { assert(mode == 0 || mode == 1); return record(mode ? "NightMode" : "DayMode"); }
static void check_fs(int channel, const IMPFSChnAttr *fs) {
    IMPFSChnAttr expected;
    memset(&expected, 0, sizeof(expected));
    expected.picWidth = channel ? expected_spec.sub_width : expected_spec.main_width;
    expected.picHeight = channel ? expected_spec.sub_height : expected_spec.main_height;
    expected.pixFmt = PIX_FMT_NV12;
    expected.crop = (IMPFSChnCrop){1,0,0,1920,1080};
    expected.outFrmRateNum = channel ? expected_spec.sub_fps : expected_spec.main_fps;
    expected.outFrmRateDen = 1;
    expected.nrVBs = 1; expected.type = FS_PHY_CHANNEL;
    if (channel == 1) expected.nrVBs = 2;
    if (expected.picWidth != 1920) {
        expected.crop = (IMPFSChnCrop){0,0,0,expected.picWidth,expected.picHeight};
        expected.scaler = (IMPFSChnScaler){1,expected.picWidth,expected.picHeight};
    }
    /* Exact kernel ordering: scaler first, ordinary crop second. The old
     * native-sized main crop passed mirrored fixtures but exceeded the actual
     * post-scale image, so enforce this independent pipeline invariant too. */
    unsigned post_width = fs->scaler.enable ? (unsigned)fs->scaler.outwidth : 1920;
    unsigned post_height = fs->scaler.enable ? (unsigned)fs->scaler.outheight : 1080;
    if (fs->crop.enable) {
        assert(fs->crop.left >= 0 && fs->crop.top >= 0);
        assert((unsigned)fs->crop.left + (unsigned)fs->crop.width <= post_width);
        assert((unsigned)fs->crop.top + (unsigned)fs->crop.height <= post_height);
    }
    assert(memcmp(&expected, fs, sizeof(expected)) == 0);
}
static int create_fs(int channel, IMPFSChnAttr *fs) {
    (void)channel_video(channel); check_fs(channel, fs); return record_channel(channel, "CreateFS");
}
static int set_fs(int channel, const IMPFSChnAttr *fs) {
    (void)channel_video(channel); check_fs(channel, fs); return record_channel(channel, "SetFS");
}
static int create_group(int group) {
    (void)channel_video(group); configuring_channel = group; return record_channel(group, "CreateGroup");
}
static int set_max_stream_count(int channel, int count) {
    assert(count == 1 && !channel_video(channel)->encoder_created);
    return record_channel(channel, "SetMaxStreamCnt");
}
static int default_param(IMPEncoderChnAttr *attr, IMPEncoderProfile profile,
    IMPEncoderRcMode rc, uint16_t width, uint16_t height, uint32_t num,
    uint32_t den, uint32_t gop, int max_same, int qp, uint32_t bitrate) {
    int channel = configuring_channel;
    (void)channel_video(channel);
    assert((unsigned)profile == (channel ? expected_spec.sub_profile : expected_spec.main_profile) && rc == 4);
    assert(width == (channel ? expected_spec.sub_width : expected_spec.main_width));
    assert(height == (channel ? expected_spec.sub_height : expected_spec.main_height));
    assert(bitrate == (channel ? expected_spec.sub_bitrate : expected_spec.main_bitrate));
    assert(num == (channel ? expected_spec.sub_fps : expected_spec.main_fps));
    assert(den == 1 && gop == num && max_same == 3 && qp == -1);
    memset(attr, 0, sizeof(*attr)); attr->encAttr.eProfile = profile;
    attr->rcAttr.attrRcMode.rcMode = rc;
    attr->rcAttr.outFrmRate = (IMPEncoderFrmRate){num,den};
    attr->gopAttr.uGopCtrlMode = IMP_ENC_GOP_CTRL_MODE_DEFAULT;
    attr->gopAttr.uGopLength = gop;
    attr->gopAttr.uMaxSameSenceCnt = max_same;
    return record_channel(channel, "DefaultParam");
}
static int create_enc(int channel, const IMPEncoderChnAttr *attr) {
    (void)channel_video(channel);
    assert((unsigned)attr->encAttr.eProfile == (channel ? expected_spec.sub_profile : expected_spec.main_profile));
    assert(attr->rcAttr.attrRcMode.rcMode == 4);
    assert(attr->rcAttr.attrRcMode.attrCappedVbr.iMinQP == 32);
    assert(attr->rcAttr.attrRcMode.attrCappedVbr.iMaxQP == 51);
    created_gop[channel] = attr->gopAttr;
    created_fps[channel] = attr->rcAttr.outFrmRate;
    return record_channel(channel, "CreateEnc");
}
static int register_enc(int group, int channel) {
    assert(group == channel && channel_video(channel)->encoder_created); return record_channel(channel, "Register");
}
static int bind_cells(IMPCell *src, IMPCell *sink) {
    assert(src->deviceID == 0 && src->outputID == 0);
    assert(sink->deviceID == 1 && sink->groupID == src->groupID && sink->outputID == 0);
    assert(channel_video(src->groupID)->encoder_registered); return record_channel(src->groupID, "Bind");
}
static int enable_fs(int channel) { assert(channel_video(channel)->bound); return record_channel(channel, "EnableFS"); }
static int start(int channel) { assert(channel_video(channel)->fs_enabled); return record_channel(channel, "Start"); }

static void reset_loader(void) {
    assert(owner == NULL);
    loaded = 0; import_checks = got_checks = 0; unloads = 0;
    loader_message = NULL; load_order[0] = close_order[0] = 0;
}
static void loader_event(char *log, char event) {
    size_t n = strlen(log); assert(n + 2 < 16); log[n] = event; log[n + 1] = 0;
}
void *test_dlopen(const char *name, int flags) {
    test_v = owner;
    if (strcmp(name, MATH_LIBRARY) == 0) {
        assert(loaded == 0 && flags == (RTLD_NOW | RTLD_GLOBAL));
        if (failed_library != NULL && strcmp(name, failed_library) == 0) {
            loader_message = "injected dlopen failure"; return NULL;
        }
        loaded = 1; loader_event(load_order, 'M'); return &fake_math;
    }
    if (strcmp(name, LOGGER_LIBRARY) == 0) {
        assert(loaded == 1 && flags == (RTLD_NOW | RTLD_GLOBAL));
        if (failed_library != NULL && strcmp(name, failed_library) == 0) {
            loader_message = "injected dlopen failure"; return NULL;
        }
        loaded = 3; loader_event(load_order, 'L'); return &fake_logger;
    }
    assert(strcmp(name, SDK_LIBRARY) == 0 && loaded == 3 && flags == (RTLD_NOW | RTLD_LOCAL));
    if (failed_library != NULL && strcmp(name, failed_library) == 0) {
        loader_message = "injected dlopen failure"; return NULL;
    }
    loaded = 7; loader_event(load_order, 'S'); return &fake_library;
}
int test_dlclose(void *library) {
    const char *name;
    unsigned expected;
    char event;
    if (library == &fake_library) { expected = 7; name = SDK_LIBRARY; event = 'S'; }
    else if (library == &fake_logger) { expected = 3; name = LOGGER_LIBRARY; event = 'L'; }
    else { assert(library == &fake_math); expected = 1; name = MATH_LIBRARY; event = 'M'; }
    assert(loaded == expected && sdk_owners_empty(test_v));
    ++unloads; loader_event(close_order, event);
    if (failed_unload != NULL && strcmp(failed_unload, name) == 0) {
        loader_message = "injected dlclose failure"; return -77;
    }
    loaded >>= 1;
    return 0;
}
char *test_dlerror(void) { char *result = loader_message; loader_message = NULL; return result; }
static uintptr_t fake_got_value(uintptr_t base, uint32_t slot) {
    assert(base == 0x10000000U && loaded == 7);
    for (size_t i = 0; i < sizeof(sdk_imports)/sizeof(sdk_imports[0]); ++i) {
        if (sdk_imports[i].got_vaddr == slot) {
            ++got_checks;
            return null_got_symbol != NULL && strcmp(null_got_symbol, sdk_imports[i].name) == 0 ? 0 : 1;
        }
    }
    assert(!"unexpected GOT slot"); return 0;
}
void *test_dlsym(void *library, const char *name) {
    assert(loaded == 7);
    if (missing_symbol != NULL && strcmp(missing_symbol, name) == 0) {
        loader_message = "injected missing symbol"; return NULL;
    }
    if (library == RTLD_DEFAULT) {
        for (size_t i = 0; i < sizeof(sdk_imports)/sizeof(sdk_imports[0]); ++i) {
            if (strcmp(name, sdk_imports[i].name) == 0) { ++import_checks; return &fake_import; }
        }
        assert(!"unexpected global import"); return NULL;
    }
    assert(library == &fake_library);
    if (strcmp(name, "IMP_ISP_Open") == 0) return (void *)&open_isp;
    if (strcmp(name, "IMP_ISP_Close") == 0) return (void *)&close_isp;
    if (strcmp(name, "IMP_ISP_AddSensor") == 0) return (void *)&add_sensor;
    if (strcmp(name, "IMP_ISP_DelSensor") == 0) return (void *)&del_sensor;
    if (strcmp(name, "IMP_ISP_EnableSensor") == 0) return (void *)&enable_sensor;
    if (strcmp(name, "IMP_ISP_DisableSensor") == 0) return (void *)&disable_sensor;
    if (strcmp(name, "IMP_System_Init") == 0) return (void *)&init_system;
    if (strcmp(name, "IMP_System_Exit") == 0) return (void *)&system_exit;
    if (strcmp(name, "IMP_ISP_EnableTuning") == 0) return (void *)&enable_tuning;
    if (strcmp(name, "IMP_ISP_DisableTuning") == 0) return (void *)&disable_tuning;
    if (strcmp(name, "IMP_ISP_Tuning_SetContrast") == 0) return (void *)&tune_value;
    if (strcmp(name, "IMP_ISP_Tuning_SetSharpness") == 0) return (void *)&tune_value;
    if (strcmp(name, "IMP_ISP_Tuning_SetSaturation") == 0) return (void *)&tune_value;
    if (strcmp(name, "IMP_ISP_Tuning_SetBrightness") == 0) return (void *)&tune_value;
    if (strcmp(name, "IMP_ISP_Tuning_SetISPRunningMode") == 0) return (void *)&day_mode;
    if (strcmp(name, "IMP_FrameSource_CreateChn") == 0) return (void *)&create_fs;
    if (strcmp(name, "IMP_FrameSource_SetChnAttr") == 0) return (void *)&set_fs;
    if (strcmp(name, "IMP_FrameSource_DestroyChn") == 0) return (void *)&destroy_fs;
    if (strcmp(name, "IMP_FrameSource_EnableChn") == 0) return (void *)&enable_fs;
    if (strcmp(name, "IMP_FrameSource_DisableChn") == 0) return (void *)&disable_fs;
    if (strcmp(name, "IMP_Encoder_CreateGroup") == 0) return (void *)&create_group;
    if (strcmp(name, "IMP_Encoder_DestroyGroup") == 0) return (void *)&destroy_group;
    if (strcmp(name, "IMP_Encoder_SetDefaultParam") == 0) return (void *)&default_param;
    if (strcmp(name, "IMP_Encoder_SetMaxStreamCnt") == 0) return (void *)&set_max_stream_count;
    if (strcmp(name, "IMP_Encoder_CreateChn") == 0) return (void *)&create_enc;
    if (strcmp(name, "IMP_Encoder_DestroyChn") == 0) return (void *)&destroy_enc;
    if (strcmp(name, "IMP_Encoder_RegisterChn") == 0) return (void *)&register_enc;
    if (strcmp(name, "IMP_Encoder_UnRegisterChn") == 0) return (void *)&unregister_enc;
    if (strcmp(name, "IMP_System_Bind") == 0) return (void *)&bind_cells;
    if (strcmp(name, "IMP_System_UnBind") == 0) return (void *)&unbind;
    if (strcmp(name, "IMP_Encoder_StartRecvPic") == 0) return (void *)&start;
    if (strcmp(name, "IMP_Encoder_StopRecvPic") == 0) return (void *)&stop;
    if (strcmp(name, "IMP_Encoder_PollingStream") == 0) return (void *)&poll_stream;
    if (strcmp(name, "IMP_Encoder_GetStream") == 0) return (void *)&get_stream;
    if (strcmp(name, "IMP_Encoder_ReleaseStream") == 0) return (void *)&put_stream;
    assert(!"unexpected SDK symbol"); return NULL;
}

static void configure(struct video *v)
{
    memset(v, 0, sizeof(*v));
    v->sdk.IMP_Encoder_StopRecvPic = stop;
    v->sdk.IMP_FrameSource_DisableChn = disable_fs;
    v->sdk.IMP_System_UnBind = unbind;
    v->sdk.IMP_Encoder_UnRegisterChn = unregister_enc;
    v->sdk.IMP_Encoder_DestroyChn = destroy_enc;
    v->sdk.IMP_Encoder_DestroyGroup = destroy_group;
    v->sdk.IMP_FrameSource_DestroyChn = destroy_fs;
    v->sdk.IMP_ISP_DisableTuning = disable_tuning;
    v->sdk.IMP_ISP_DisableSensor = disable_sensor;
    v->sdk.IMP_ISP_DelSensor = del_sensor;
    v->sdk.IMP_ISP_Close = close_isp;
    v->sdk.IMP_System_Exit = system_exit;
    v->sdk.IMP_Encoder_PollingStream = poll_stream;
    v->sdk.IMP_Encoder_GetStream = get_stream;
    v->sdk.IMP_Encoder_ReleaseStream = put_stream;
    v->source = (IMPCell){DEV_ID_FS, 0, 0};
    v->sink = (IMPCell){DEV_ID_ENC, 0, 0};
    v->isp_open = v->sensor_added = v->sensor_enabled = v->system_initialized = true;
    v->tuning_enabled = v->fs_created = v->fs_enabled = v->group_created = true;
    v->encoder_created = v->encoder_registered = v->bound = v->receiving = true;
    test_v = v; events[0] = 0;
    releases = 0; poll_result = get_result = release_result = 0;
}
int main(void)
{
    struct video v;
    char error[1024];
    const char *steps[] = {"Stop", "DisableFS", "Unbind", "Unregister", "DestroyEnc",
        "DestroyGroup", "DestroyFS", "DisableTuning", "DisableSensor", "DelSensor",
        "CloseISP", "SystemExit"};
    configure(&v); fail_op = NULL; error[0] = 0;
    cleanup(&v, error, sizeof(error));
    assert(sdk_owners_empty(&v) && error[0] == 0);
    assert(strcmp(events, "Stop;DisableFS;Unbind;Unregister;DestroyEnc;DestroyGroup;DestroyFS;DisableTuning;DisableSensor;DelSensor;CloseISP;SystemExit;") == 0);
    for (size_t i = 0; i < sizeof(steps)/sizeof(steps[0]); ++i) {
        configure(&v); fail_op = steps[i]; error[0] = 0;
        cleanup(&v, error, sizeof(error));
        assert(!sdk_owners_empty(&v) && v.failed);
        assert(strstr(error, "code=-55") && strstr(error, "POISONED"));
    }
    configure(&v); fail_op = NULL; v.stream_held = true; error[0] = 0;
    cleanup(&v, error, sizeof(error));
    assert(events[0] == 0 && v.stream_held && strstr(error, "POISONED"));

    uint8_t *ring = calloc(1, 4096);
    assert(ring != NULL);
    memcpy(ring, "ABCDEFGH", 8);
    IMPEncoderPack packs[3];
    memset(packs, 0, sizeof(packs));
    packs[0].offset = 6; packs[0].length = 4; packs[0].timestamp = 123456;
    packs[1].offset = UINT32_MAX; /* zero length must not access this address */
    packs[2].offset = 2; packs[2].length = 3; packs[2].frameEnd = true;
    supplied = (IMPEncoderStream){0, (uintptr_t)ring, 8, packs, 3, 1};
    uint8_t dst[32]; uint32_t length; int64_t timestamp;
    error[0] = 0;
    assert(copy_frame(&supplied, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == 0);
    assert(length == 7 && timestamp == 123456 && memcmp(dst, "GHABCDE", 7) == 0);
    memset(dst, 0x99, sizeof(dst)); error[0] = 0;
    assert(copy_frame(&supplied, dst, 6, &length, &timestamp, error, sizeof(error)) == -1);
    for (size_t i = 0; i < sizeof(dst); ++i) assert(dst[i] == 0x99);
    packs[2].offset = 8; error[0] = 0;
    assert(copy_frame(&supplied, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == -1);
    for (size_t i = 0; i < sizeof(dst); ++i) assert(dst[i] == 0x99);
    packs[2].offset = 2;
    supplied.packCount = 257; error[0] = 0;
    assert(copy_frame(&supplied, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == -1);
    supplied.packCount = 3;
    supplied.streamSize = MAX_RING_BYTES + 1; error[0] = 0;
    assert(copy_frame(&supplied, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == -1);
    supplied.streamSize = 8;

    configure(&v); owner = &v; poll_result = -99;
    assert(t31_video_next(&v, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == 1);
    assert(length == 0 && timestamp == 0 && releases == 0 && !v.failed);
    assert(strstr(error, "code=-99"));
    poll_result = 0; get_result = -88;
    assert(t31_video_next(&v, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == -1);
    assert(releases == 0 && v.failed && strstr(error, "code=-88"));
    assert(v.acquisition_uncertain && strstr(error, "POISONED"));
    assert(t31_video_close(&v, error, sizeof(error)) == -1);
    assert(events[0] == 0 && releases == 0 && owner == &v && v.closed);
    configure(&v); supplied.packCount = 257;
    assert(t31_video_next(&v, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == -1);
    assert(releases == 1 && !v.stream_held && v.failed && length == 0);
    configure(&v); supplied.packCount = 3; release_result = -77;
    assert(t31_video_next(&v, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == -1);
    assert(releases == 1 && v.stream_held && v.failed && length == 0);
    assert(strstr(error, "code=-77") && strstr(error, "POISONED"));
    assert(t31_video_close(&v, error, sizeof(error)) == -1);
    assert(releases == 1 && events[0] == 0 && v.closed && owner == &v);
    assert(t31_video_close(&v, error, sizeof(error)) == -1 && releases == 1);
    configure(&v);
    assert(t31_video_next(&v, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == 0);
    assert(length == 7 && timestamp == 123456 && releases == 1 && !v.stream_held && !v.failed);
    /* Leading empty packs do not supply the frame's PTS. */
    packs[0].length = 0; packs[0].timestamp = -123;
    packs[2].timestamp = 567890;
    assert(t31_video_next(&v, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == 0);
    assert(length == 3 && timestamp == 567890 && memcmp(dst, "CDE", 3) == 0);
    owner = NULL;
    const char *open_steps[] = {"OpenISP", "AddSensor", "EnableSensor", "SystemInit",
        "EnableTuning", "TuningValue", "DayMode", "CreateFS", "SetFS", "CreateGroup",
        "DefaultParam", "SetMaxStreamCnt", "CreateEnc", "Register", "Bind", "EnableFS", "Start"};
    for (size_t i = 0; i < sizeof(open_steps)/sizeof(open_steps[0]); ++i) {
        void *handle = (void *)1;
        events[0] = 0; reset_loader(); fail_op = open_steps[i];
        assert(t31_video_open(&handle, error, sizeof(error)) == -1);
        bool acquisition = strcmp(open_steps[i], "TuningValue") != 0 &&
            strcmp(open_steps[i], "DayMode") != 0 && strcmp(open_steps[i], "SetFS") != 0 &&
            strcmp(open_steps[i], "DefaultParam") != 0 && strcmp(open_steps[i], "SetMaxStreamCnt") != 0;
        assert(handle == NULL && strstr(error, "code=-55"));
        if (acquisition) {
            assert(owner != NULL && owner->closed && owner->acquisition_uncertain && unloads == 0);
            assert(strstr(error, "POISONED") && !strstr(events, "Destroy"));
            /* Fake callbacks own no SDK state; free fixture, never retry cleanup. */
            free(owner); owner = NULL;
        } else {
            assert(owner == NULL && unloads == 3 && !strstr(error, "POISONED"));
        }
    }
    fail_op = NULL; missing_symbol = "IMP_Encoder_ReleaseStream"; reset_loader(); events[0] = 0;
    void *handle = NULL;
    assert(t31_video_open(&handle, error, sizeof(error)) == -1);
    assert(owner == NULL && handle == NULL && unloads == 3 && events[0] == 0);
    missing_symbol = NULL; events[0] = 0; reset_loader();
    assert(t31_video_open(&handle, error, sizeof(error)) == 0 && handle == owner);
    assert(t31_video_close(handle, error, sizeof(error)) == 0 && owner == NULL && unloads == 3);
    assert(t31_video_close(handle, error, sizeof(error)) == -1 && unloads == 3);
    assert(loaded == 0 && strcmp(load_order, "MLS") == 0 && strcmp(close_order, "SLM") == 0);

    /* An init error plus a dependent cleanup error retains poisoned ownership. */
    fail_op = "DefaultParam"; second_fail_op = "DestroyGroup"; reset_loader(); events[0] = 0;
    assert(t31_video_open(&handle, error, sizeof(error)) == -1);
    assert(handle == NULL && owner != NULL && owner->closed && unloads == 0);
    assert(strstr(error, "code=-55") && strstr(error, "code=-66") && strstr(error, "POISONED"));
    assert(t31_video_open(&handle, error, sizeof(error)) == -1 && owner != NULL && unloads == 0);
    assert(strstr(error, "POISONED"));
    /* Only fake callbacks ran; discard fixture storage without retrying cleanup. */
    free(owner); owner = NULL; fail_op = second_fail_op = NULL;
    /* Every strong provider and actual GOT slot is checked before ISP_Open. */
    for (size_t i = 0; i < sizeof(sdk_imports)/sizeof(sdk_imports[0]); ++i) {
        reset_loader(); events[0] = 0; missing_symbol = sdk_imports[i].name;
        assert(t31_video_open(&handle, error, sizeof(error)) == -1);
        assert(handle == NULL && owner == NULL && unloads == 3 && loaded == 0 && events[0] == 0);
        assert(strstr(error, sdk_imports[i].name) && !strstr(error, "POISONED"));
        missing_symbol = NULL; null_got_symbol = sdk_imports[i].name;
        reset_loader(); events[0] = 0;
        assert(t31_video_open(&handle, error, sizeof(error)) == -1);
        assert(handle == NULL && owner == NULL && unloads == 3 && loaded == 0 && events[0] == 0);
        assert(strstr(error, sdk_imports[i].name) && strstr(error, "GOT slot is null"));
        null_got_symbol = NULL;
    }
    const char *libraries[] = {MATH_LIBRARY, LOGGER_LIBRARY, SDK_LIBRARY};
    for (size_t i = 0; i < 3; ++i) {
        reset_loader(); events[0] = 0; failed_library = libraries[i];
        assert(t31_video_open(&handle, error, sizeof(error)) == -1);
        assert(handle == NULL && owner == NULL && loaded == 0 && events[0] == 0);
        assert(unloads == (int)i && strstr(error, "dlopen"));
    }
    failed_library = NULL;
    const char *reverse_libraries[] = {SDK_LIBRARY, LOGGER_LIBRARY, MATH_LIBRARY};
    for (size_t i = 0; i < 3; ++i) {
        reset_loader(); events[0] = 0; failed_unload = reverse_libraries[i];
        assert(t31_video_open(&handle, error, sizeof(error)) == 0);
        assert(t31_video_close(handle, error, sizeof(error)) == -1);
        assert(owner != NULL && owner->closed && strstr(error, "code=-77") && strstr(error, "POISONED"));
        assert(loaded == (7U >> i) && unloads == (int)i + 1);
        /* Fake loader has no real refs; discard fixture without retrying close. */
        free(owner); owner = NULL;
    }
    failed_unload = NULL;
    /* Dual stream uses one qualified loader/System, independent channel records,
     * one borrowed SDK handle, and child-before-parent teardown. */
    dual_fixture = true; expected_poll_ms = 10;
    reset_loader(); events[0] = 0; handle = NULL;
    poll_result = get_result = release_result = 0;
    released_channels[0] = released_channels[1] = 0;
    assert(t31_video_open_dual(&handle, error, sizeof(error)) == 0 && handle == owner);
    assert(owner->sub && owner->sub->channel_id == 1);
    assert(owner->sub->sdk.IMP_Encoder_GetStream == owner->sdk.IMP_Encoder_GetStream);
    assert(!owner->sub->library && !owner->sub->isp_open && !owner->sub->system_initialized);
    assert(loaded == 7 && !unloads && strcmp(load_order, "MLS") == 0);
    assert(t31_video_sdk(handle) == &fake_library && t31_video_sdk(NULL) == NULL);
    assert(t31_video_night(handle, 1, error, sizeof(error)) == 0 && strstr(events, "NightMode"));
    size_t prior_events = strlen(events);
    assert(t31_video_night(handle, 2, error, sizeof(error)) == -1 && strlen(events) == prior_events);
    packs[0].length = 0; packs[2].timestamp = 432100; packs[2].frameEnd = true;
    supplied.packCount = 3; supplied.streamSize = 8;
    assert(t31_video_next_channel(handle, 0, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == 0);
    assert(length == 3 && timestamp == 432100 && !memcmp(dst, "CDE", 3));
    packs[2].timestamp = 543200;
    assert(t31_video_next_channel(handle, 1, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == 0);
    assert(length == 3 && timestamp == 543200 && !memcmp(dst, "CDE", 3));
    assert(released_channels[0] == 1 && released_channels[1] == 1);
    assert(t31_video_next_channel(handle, 2, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == -1);
    assert(length == 0 && timestamp == 0 && !owner->failed && !owner->sub->failed);
    assert(t31_video_close(handle, error, sizeof(error)) == 0 && owner == NULL && unloads == 3);
    assert(t31_video_sdk(handle) == NULL);
    const char *sub_end = strstr(events, "SubDestroyFS;");
    const char *main_start = strstr(events, ";Stop;");
    assert(sub_end && main_start && sub_end < main_start);
    assert(strstr(main_start, "SystemExit;") && !strstr(strstr(events, "SystemExit;") + 1, "SystemExit;"));

    /* A child's failed acquisition must retain shared System/ISP/providers,
     * including the first CreateFS failure before any child stage flag exists. */
    const char *sub_acquisitions[] = {"SubCreateFS", "SubCreateGroup", "SubCreateEnc",
        "SubRegister", "SubBind", "SubEnableFS", "SubStart"};
    for (size_t i = 0; i < sizeof(sub_acquisitions)/sizeof(sub_acquisitions[0]); ++i) {
        reset_loader(); events[0] = 0; handle = NULL; fail_op = sub_acquisitions[i];
        assert(t31_video_open_dual(&handle, error, sizeof(error)) == -1);
        assert(!handle && owner && owner->sub && owner->sub->acquisition_uncertain);
        assert(owner->isp_open && owner->system_initialized && !unloads && loaded == 7);
        assert(!strstr(events, "DisableTuning;") && !strstr(events, "SystemExit;") && strstr(error, "POISONED"));
        free(owner->sub); free(owner); owner = NULL;
    }
    /* A known nonallocating child configuration error can unwind both channels. */
    reset_loader(); events[0] = 0; handle = NULL; fail_op = "SubDefaultParam";
    assert(t31_video_open_dual(&handle, error, sizeof(error)) == -1);
    assert(!handle && !owner && unloads == 3 && loaded == 0 && !strstr(error, "POISONED"));

    reset_loader(); events[0] = 0; handle = NULL; fail_op = "SubSetMaxStreamCnt";
    assert(t31_video_open_dual(&handle, error, sizeof(error)) == -1);
    assert(!handle && !owner && unloads == 3 && loaded == 0 && !strstr(error, "POISONED"));

    reset_loader(); events[0] = 0; handle = NULL; fail_op = NULL;
    assert(t31_video_open_dual(&handle, error, sizeof(error)) == 0);
    released_channels[1] = 0; failed_release_channel = 1;
    assert(t31_video_next_channel(handle, 1, dst, sizeof(dst), &length, &timestamp, error, sizeof(error)) == -1);
    assert(!length && !timestamp && released_channels[1] == 1 && owner->sub->stream_held);
    assert(t31_video_close(handle, error, sizeof(error)) == -1);
    assert(released_channels[1] == 1 && !unloads && owner->isp_open && owner->system_initialized);
    assert(!strstr(events, "SubStop;") && !strstr(events, "SystemExit;"));
    free(owner->sub); free(owner); owner = NULL; failed_release_channel = -1;

    reset_loader(); events[0] = 0; handle = NULL;
    assert(t31_video_open_dual(&handle, error, sizeof(error)) == 0);
    fail_op = "SubStop";
    assert(t31_video_close(handle, error, sizeof(error)) == -1 && !unloads);
    assert(owner && owner->sub->receiving && owner->isp_open && owner->system_initialized);
    assert(!strstr(events, "SubUnbind;") && !strstr(events, "SystemExit;"));
    free(owner->sub); free(owner); owner = NULL; fail_op = NULL;

    /* Reject the entire specification before any provider/SDK call. Each field
     * is checked independently, including values that would narrow to O32/u16. */
    const struct t31_video_spec defaults = T31_VIDEO_SPEC_DEFAULT;
    const size_t offsets[] = {
        offsetof(struct t31_video_spec, main_width), offsetof(struct t31_video_spec, main_height),
        offsetof(struct t31_video_spec, sub_width), offsetof(struct t31_video_spec, sub_height),
        offsetof(struct t31_video_spec, main_profile), offsetof(struct t31_video_spec, sub_profile),
        offsetof(struct t31_video_spec, main_fps), offsetof(struct t31_video_spec, sub_fps),
        offsetof(struct t31_video_spec, main_bitrate), offsetof(struct t31_video_spec, sub_bitrate)
    };
    for (size_t field = 0; field < sizeof(offsets)/sizeof(*offsets); ++field) {
        const uint32_t invalid[] = {0, UINT32_MAX};
        for (size_t i = 0; i < sizeof(invalid)/sizeof(*invalid); ++i) {
            struct t31_video_spec spec = defaults;
            memcpy((uint8_t *)&spec + offsets[field], &invalid[i], sizeof(invalid[i]));
            struct t31_video_spec before = spec;
            reset_loader(); events[0] = 0; handle = (void *)1;
            assert(t31_video_open_spec(&handle, &spec, error, sizeof(error)) == -1);
            assert(!handle && !owner && !loaded && !unloads && !import_checks && !got_checks);
            assert(!events[0] && !load_order[0] && strstr(error, "video spec"));
            assert(memcmp(&spec, &before, sizeof(spec)) == 0);
        }
    }
    struct t31_video_spec invalid_specs[] = {
        {1920,720,640,360,66,66,20,20,800,240},
        {1280,1080,640,360,66,66,20,20,800,240},
        {640,360,640,180,66,66,20,20,800,240},
        {1920,1080,320,360,66,66,20,20,800,240},
        {1920,1080,640,360,67,66,20,20,800,240},
        {1920,1080,640,360,66,16777217,20,20,800,240},
        {1920,1080,640,360,66,66,21,20,800,240},
        {1920,1080,640,360,66,66,20,21,800,240},
        {1920,1080,640,360,66,66,20,20,31,240},
        {1920,1080,640,360,66,66,20,20,2049,240},
        {1920,1080,640,360,66,66,20,20,800,31},
        {1920,1080,640,360,66,66,20,20,800,513}
    };
    for (size_t i = 0; i < sizeof(invalid_specs)/sizeof(*invalid_specs); ++i) {
        reset_loader(); events[0] = 0; handle = (void *)1;
        assert(t31_video_open_spec(&handle, &invalid_specs[i], error, sizeof(error)) == -1);
        assert(!handle && !owner && !loaded && !load_order[0] && !events[0]);
    }
    reset_loader(); events[0] = 0; handle = (void *)1;
    assert(t31_video_open_spec(&handle, NULL, error, sizeof(error)) == -1 && !handle);
    assert(t31_video_open_spec(NULL, &defaults, error, sizeof(error)) == -1);
    assert(!owner && !loaded && !events[0] && !load_order[0]);

    /* The complete geometry/profile matrix, with independent channel rates and
     * creation bitrate boundary values. Both channels may have width640: the
     * fake must identify channel from its creation stage, never from width. */
    const unsigned main_sizes[][2] = {{1920,1080},{1280,720},{640,360}};
    const unsigned sub_sizes[][2] = {{640,360},{320,180}};
    const unsigned profiles[] = {66,77,100};
    unsigned specification_cases = 0;
    for (size_t m = 0; m < 3; ++m) for (size_t s = 0; s < 2; ++s)
    for (size_t mp = 0; mp < 3; ++mp) for (size_t sp = 0; sp < 3; ++sp) {
        expected_spec = (struct t31_video_spec){
            main_sizes[m][0],main_sizes[m][1],sub_sizes[s][0],sub_sizes[s][1],
            profiles[mp],profiles[sp],mp ? 20:1,sp ? 1:20,mp ? 2048:32,sp ? 32:512
        };
        struct t31_video_spec before = expected_spec;
        reset_loader(); events[0] = 0; handle = NULL;
        assert(t31_video_open_spec(&handle, &expected_spec, error, sizeof(error)) == 0);
        assert(handle == owner && owner->sub && loaded == 7 && strcmp(load_order,"MLS") == 0);
        assert(memcmp(&expected_spec,&before,sizeof(before)) == 0);
        assert(t31_video_sdk(handle) == &fake_library);
        assert(t31_video_close(handle,error,sizeof(error)) == 0 && !owner && unloads == 3);
        ++specification_cases;
    }
    assert(specification_cases == 54);
    /* Coprime non-default FPS expose the old fixed20-frame initial GOP. Check
     * the actual record delivered to CreateChn, using the duration getter's
     * integer contract, independently for both channels and with their roles
     * reversed. These are host-only SDK fixtures, never camera operations. */
    const unsigned rates[][2] = {{7,13},{13,7},{20,20}};
    for (size_t index = 0; index < sizeof(rates)/sizeof(*rates); ++index) {
        expected_spec = defaults;
        expected_spec.main_fps = rates[index][0];
        expected_spec.sub_fps = rates[index][1];
        memset(created_gop,0,sizeof(created_gop));
        memset(created_fps,0,sizeof(created_fps));
        reset_loader(); events[0] = 0; handle = NULL;
        assert(t31_video_open_spec(&handle,&expected_spec,error,sizeof(error)) == 0);
        for (unsigned channel = 0; channel < 2; ++channel) {
            assert(created_fps[channel].frmRateNum == rates[index][channel]);
            assert(created_fps[channel].frmRateDen == 1);
            assert(created_gop[channel].uGopCtrlMode == IMP_ENC_GOP_CTRL_MODE_DEFAULT);
            assert(created_gop[channel].uGopLength % created_fps[channel].frmRateNum == 0);
            assert(created_gop[channel].uGopLength / created_fps[channel].frmRateNum == 1);
            assert(created_gop[channel].uMaxSameSenceCnt == 3);
            assert(!created_gop[channel].uNumB && !created_gop[channel].bEnableLT && !created_gop[channel].bLTRC);
        }
        assert(t31_video_close(handle,error,sizeof(error)) == 0 && !owner && unloads == 3);
    }
    expected_spec = defaults;
    /* A rejected second constructor cannot replace or mutate a live graph. */
    reset_loader(); events[0] = 0; handle = NULL;
    assert(t31_video_open_spec(&handle,&defaults,error,sizeof(error)) == 0);
    struct video *original = owner; void *other = (void *)1;
    size_t before_events = strlen(events);
    assert(t31_video_open_spec(&other,&defaults,error,sizeof(error)) == -1);
    assert(!other && owner == original && strlen(events) == before_events);
    assert(t31_video_close(handle,error,sizeof(error)) == 0 && !owner);
    free(ring);
    puts("PASS: ring wrap/PTS/bounds; poll/Get/Release; init/cleanup failures; 116 imports/GOT slots; dual channel configuration/copy/borrow/teardown; child poison retains parents; atomic spec refusal and54 geometry/profile configurations; independent7/13/20fps one-second initial GOPs");
}
