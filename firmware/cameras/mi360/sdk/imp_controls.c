/* Exact shipped IMP1.1.2 scalar ABI. See imp_controls.h for capability limits. */
#include "imp_controls.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <dlfcn.h>
#include <imp/imp_isp.h>
#include <imp/imp_audio.h>
#include <imp/imp_encoder.h>

#ifndef T31_CONTROLS_HOST_TEST
_Static_assert(sizeof(void *) == 4, "O32 pointers");
#endif
_Static_assert(sizeof(int) == 4 && sizeof(unsigned char) == 1, "scalar ABI");
_Static_assert(sizeof(&IMP_ISP_Tuning_GetHVFlip) == sizeof(void *) &&
               sizeof(&IMP_AI_SetVol) == sizeof(void *) &&
               sizeof(&IMP_Encoder_SetChnFrmRate) == sizeof(void *), "SDK function pointers");
_Static_assert(sizeof(IMPISPHVFLIP) == 4, "HV enum ABI");
_Static_assert(IMPISP_FLIP_NORMAL_MODE == 0 && IMPISP_FLIP_H_MODE == 1 &&
               IMPISP_FLIP_V_MODE == 2 && IMPISP_FLIP_HV_MODE == 3, "HV values");
_Static_assert(sizeof(IMPISPAntiflickerAttr) == 4 && IMPISP_ANTIFLICKER_DISABLE == 0 &&
               IMPISP_ANTIFLICKER_50HZ == 1 && IMPISP_ANTIFLICKER_60HZ == 2, "flicker ABI");
_Static_assert(sizeof(IMPEncoderFrmRate) == 8 && _Alignof(IMPEncoderFrmRate) == 4,
               "FPS record ABI");
_Static_assert(offsetof(IMPEncoderFrmRate, frmRateNum) == 0 &&
               offsetof(IMPEncoderFrmRate, frmRateDen) == 4, "FPS offsets");
_Static_assert(sizeof(IMPEncoderAttrRcMode) == 36 && _Alignof(IMPEncoderAttrRcMode) == 4,
               "RC record ABI");
_Static_assert(offsetof(IMPEncoderAttrRcMode, attrCbr.uTargetBitRate) == 4 &&
               offsetof(IMPEncoderAttrRcMode, attrVbr.uMaxBitRate) == 8 &&
               offsetof(IMPEncoderAttrRcMode, attrCappedVbr.uMaxBitRate) == 8,
               "RC bitrate offsets");
_Static_assert(sizeof(IMPEncoderGopAttr) == 24 && _Alignof(IMPEncoderGopAttr) == 4 &&
               offsetof(IMPEncoderGopAttr, uGopLength) == 4 &&
               offsetof(IMPEncoderGopAttr, uNumB) == 6 &&
               offsetof(IMPEncoderGopAttr, uMaxSameSenceCnt) == 8 &&
               offsetof(IMPEncoderGopAttr, bEnableLT) == 12, "GOP record ABI");
_Static_assert(IMP_ENC_RC_MODE_FIXQP == 0 && IMP_ENC_RC_MODE_CBR == 1 &&
               IMP_ENC_RC_MODE_VBR == 2 && IMP_ENC_RC_MODE_CAPPED_VBR == 4 &&
               IMP_ENC_RC_MODE_CAPPED_QUALITY == 8 &&
               IMP_ENC_GOP_CTRL_MODE_DEFAULT == 2, "encoder enum ABI");

static unsigned int busy;
static int diagnostic(char *error, uint32_t cap, int result, const char *format, ...) {
    if (cap) {
        va_list args;
        va_start(args, format);
        (void)vsnprintf(error, cap, format, args);
        va_end(args);
    }
    return result;
}
static int resolve(void *sdk, void *function, size_t size, const char *name,
                   char *error, uint32_t cap) {
    (void)dlerror();
    void *symbol = dlsym(sdk, name);
    const char *failure = dlerror();
    if (!symbol || failure || size != sizeof(symbol))
        return diagnostic(error, cap, -1, "missing qualified SDK symbol %s", name);
    memcpy(function, &symbol, size);
    return 0;
}
#define RESOLVE(name) \
    __typeof__(&name) name##_fn; \
    if (resolve(sdk, &name##_fn, sizeof(name##_fn), #name, error, cap)) return -1
#define SDK_CALL(expression) do { \
    int sdk_result = (expression); \
    if (sdk_result) return diagnostic(error, cap, -1, "%s failed (%d)", #expression, sdk_result); \
} while (0)

static int valid_scalar(unsigned int control, int32_t value) {
    if (control == 0) return value >= 0 && value <= 3;
    if (control >= 1 && control <= 4) return value >= 0 && value <= 255;
    if (control == 5) return value >= 0 && value <= 2;
    if (control == 10 || control == 11) return value >= -30 && value <= 120;
    if (control == 14 || control == 15) return value == 0 || value == 1;
    if (control == 20 || control == 21) return value >= 1 && value <= 20;
    if (control == 22 || control == 23) return value >= 1 && value <= INT32_MAX / 1000;
    if (control == 24 || control == 25) return value >= 1 && value <= 15;
    if (control == 26 || control == 27) return value >= 1 && value <= 3;
    return 0;
}
static int unsupported(unsigned int control, char *error, uint32_t cap) {
    return diagnostic(error, cap, T31_CONTROL_UNSUPPORTED,
                      "control %u has no qualified readable capability", control);
}

static int rc_bitrate(const IMPEncoderAttrRcMode *attr, int32_t *value,
                      uint32_t *maximum, char *error, uint32_t cap) {
    uint32_t target, max;
    switch (attr->rcMode) {
    case IMP_ENC_RC_MODE_CBR:
        target = max = attr->attrCbr.uTargetBitRate; break;
    case IMP_ENC_RC_MODE_VBR:
        target = attr->attrVbr.uTargetBitRate; max = attr->attrVbr.uMaxBitRate; break;
    case IMP_ENC_RC_MODE_CAPPED_VBR:
    case IMP_ENC_RC_MODE_CAPPED_QUALITY:
        target = attr->attrCappedVbr.uTargetBitRate;
        max = attr->attrCappedVbr.uMaxBitRate; break;
    default:
        return diagnostic(error, cap, -3, "rate-control mode has no qualified bitrate capability");
    }
    if (!target || target > INT32_MAX / 1000 || max < target || max > INT32_MAX / 1000)
        return diagnostic(error, cap, -1, "invalid encoder bitrate readback");
    *value = (int32_t)target;
    if (maximum) *maximum = max;
    return 0;
}

static int get(void *sdk, unsigned int control, int32_t *value, char *error, uint32_t cap) {
    int32_t result = INT32_MIN;
    switch (control) {
    case 0: {
        RESOLVE(IMP_ISP_Tuning_GetHVFlip);
        IMPISPHVFLIP attr = (IMPISPHVFLIP)-1;
        SDK_CALL(IMP_ISP_Tuning_GetHVFlip_fn(&attr)); result = (int32_t)attr; break;
    }
    case 1: case 2: case 3: case 4: {
        static const char *names[] = {"IMP_ISP_Tuning_GetBrightness", "IMP_ISP_Tuning_GetContrast",
                                     "IMP_ISP_Tuning_GetSaturation", "IMP_ISP_Tuning_GetSharpness"};
        __typeof__(&IMP_ISP_Tuning_GetBrightness) function;
        if (resolve(sdk, &function, sizeof(function), names[control-1], error, cap)) return -1;
        unsigned char byte = 0;
        SDK_CALL(function(&byte)); result = byte; break;
    }
    case 5: {
        RESOLVE(IMP_ISP_Tuning_GetAntiFlickerAttr);
        IMPISPAntiflickerAttr attr = (IMPISPAntiflickerAttr)-1;
        SDK_CALL(IMP_ISP_Tuning_GetAntiFlickerAttr_fn(&attr)); result = (int32_t)attr; break;
    }
    case 10: case 11: {
        const char *name = control == 10 ? "IMP_AI_GetVol" : "IMP_AO_GetVol";
        __typeof__(&IMP_AI_GetVol) function;
        if (resolve(sdk, &function, sizeof(function), name, error, cap)) return -1;
        int level = INT32_MIN;
        SDK_CALL(function(control == 10 ? 1 : 0, 0, &level)); result = level; break;
    }
    case 20: case 21: {
        RESOLVE(IMP_Encoder_GetChnFrmRate);
        IMPEncoderFrmRate fps = {0};
        SDK_CALL(IMP_Encoder_GetChnFrmRate_fn((int)(control-20), &fps));
        if (!fps.frmRateNum || !fps.frmRateDen)
            return diagnostic(error, cap, -1, "invalid active FPS readback");
        if (fps.frmRateNum % fps.frmRateDen)
            return diagnostic(error, cap, -3, "fractional active FPS is not a scalar capability");
        uint32_t quotient = fps.frmRateNum / fps.frmRateDen;
        if (quotient > INT32_MAX) return diagnostic(error, cap, -1, "active FPS overflow");
        result = (int32_t)quotient; break;
    }
    case 22: case 23: {
        RESOLVE(IMP_Encoder_GetChnAttrRcMode);
        IMPEncoderAttrRcMode attr = {0};
        SDK_CALL(IMP_Encoder_GetChnAttrRcMode_fn((int)(control-22), &attr));
        int rc = rc_bitrate(&attr, &result, NULL, error, cap);
        if (rc) return rc;
        break;
    }
    case 24: case 25: case 26: case 27: {
        RESOLVE(IMP_Encoder_GetChnGopAttr);
        IMPEncoderGopAttr attr = {0};
        SDK_CALL(IMP_Encoder_GetChnGopAttr_fn((int)((control-24)&1), &attr));
        if (!attr.uGopLength)
            return diagnostic(error, cap, -1, "invalid zero GOP readback");
        if (attr.uGopCtrlMode != IMP_ENC_GOP_CTRL_MODE_DEFAULT || attr.uNumB ||
            attr.bEnableLT || attr.bLTRC || !attr.uMaxSameSenceCnt || attr.uMaxSameSenceCnt > 3)
            return diagnostic(error, cap, -3, "GOP graph has no qualified duration capability");
        if (control >= 26) { result = (int32_t)attr.uMaxSameSenceCnt; break; }
        int32_t fps;
        int rc = get(sdk, control-4, &fps, error, cap);
        if (rc) return rc;
        if (attr.uGopLength % (uint32_t)fps)
            return diagnostic(error, cap, -3, "active GOP is not an integer duration");
        result = attr.uGopLength / (uint32_t)fps; break;
    }
    default: return unsupported(control, error, cap);
    }
    if (!valid_scalar(control, result))
        return diagnostic(error, cap, -1, "control %u returned an invalid scalar", control);
    *value = result;
    return 0;
}

static int set(void *sdk, unsigned int control, int32_t value, char *error, uint32_t cap) {
    if (control == 12 || control == 13 || control > 27 ||
        !(control <= 5 || (control >= 10 && control <= 15) || control >= 20))
        return unsupported(control, error, cap);
    if (!valid_scalar(control, value))
        return diagnostic(error, cap, -1, "control %u scalar out of range", control);
    /* Preflight the setter before any vendor getter. */
    static const char *isp_set[] = {"IMP_ISP_Tuning_SetHVFLIP", "IMP_ISP_Tuning_SetBrightness",
        "IMP_ISP_Tuning_SetContrast", "IMP_ISP_Tuning_SetSaturation", "IMP_ISP_Tuning_SetSharpness",
        "IMP_ISP_Tuning_SetAntiFlickerAttr"};
    const char *name;
    if (control <= 5) name = isp_set[control];
    else if (control == 10) name = "IMP_AI_SetVol";
    else if (control == 11) name = "IMP_AO_SetVol";
    else if (control == 14) name = "IMP_AI_SetVolMute";
    else if (control == 15) name = "IMP_AO_SetVolMute";
    else if (control <= 21) name = "IMP_Encoder_SetChnFrmRate";
    else if (control <= 23) name = "IMP_Encoder_SetChnBitRate";
    else if (control <= 25) name = "IMP_Encoder_SetChnGopLength";
    else name = "IMP_Encoder_SetChnGopAttr";
    void *setter;
    if (resolve(sdk, &setter, sizeof(setter), name, error, cap)) return -1;
    int32_t before;
    if (control != 14 && control != 15) {
        int rc = get(sdk, control, &before, error, cap);
        if (rc) return rc;
        if (before == value) return 0;
    }
    switch (control) {
    case 0: {
        __typeof__(&IMP_ISP_Tuning_SetHVFLIP) function;
        memcpy(&function, &setter, sizeof(function));
        SDK_CALL(function((IMPISPHVFLIP)value)); break;
    }
    case 1: case 2: case 3: case 4: {
        __typeof__(&IMP_ISP_Tuning_SetBrightness) function;
        memcpy(&function, &setter, sizeof(function));
        SDK_CALL(function((unsigned char)value)); break;
    }
    case 5: {
        __typeof__(&IMP_ISP_Tuning_SetAntiFlickerAttr) function;
        memcpy(&function, &setter, sizeof(function));
        SDK_CALL(function((IMPISPAntiflickerAttr)value)); break;
    }
    case 10: case 11: case 14: case 15: {
        __typeof__(&IMP_AI_SetVol) function;
        memcpy(&function, &setter, sizeof(function));
        SDK_CALL(function(control == 10 || control == 14 ? 1 : 0, 0, value));
        if (control >= 14)
            return diagnostic(error, cap, 2, "mute accepted; SDK provides no mute readback");
        break;
    }
    case 20: case 21: {
        __typeof__(&IMP_Encoder_SetChnFrmRate) function;
        memcpy(&function, &setter, sizeof(function));
        IMPEncoderFrmRate fps = {(uint32_t)value, 1};
        SDK_CALL(function((int)(control-20), &fps)); break;
    }
    case 22: case 23: {
        RESOLVE(IMP_Encoder_GetChnAttrRcMode);
        IMPEncoderAttrRcMode attr = {0}; uint32_t maximum; int32_t current;
        SDK_CALL(IMP_Encoder_GetChnAttrRcMode_fn((int)(control-22), &attr));
        int rc = rc_bitrate(&attr, &current, &maximum, error, cap);
        if (rc) return rc;
        if (attr.rcMode == IMP_ENC_RC_MODE_CBR)
            return diagnostic(error, cap, -3, "CBR allocation ceiling is not qualified for live mutation");
        if ((uint32_t)value > maximum)
            return diagnostic(error, cap, -1, "bitrate exceeds existing channel ceiling");
        __typeof__(&IMP_Encoder_SetChnBitRate) function;
        memcpy(&function, &setter, sizeof(function));
        SDK_CALL(function((int)(control-22), value*1000, (int)(maximum*1000))); break;
    }
    case 24: case 25: case 26: case 27: {
        RESOLVE(IMP_Encoder_GetChnGopAttr);
        IMPEncoderGopAttr attr = {0};
        int channel = (int)((control-24)&1);
        SDK_CALL(IMP_Encoder_GetChnGopAttr_fn(channel, &attr));
        if (attr.uGopCtrlMode != IMP_ENC_GOP_CTRL_MODE_DEFAULT || attr.uNumB ||
            attr.bEnableLT || attr.bLTRC || !attr.uMaxSameSenceCnt || attr.uMaxSameSenceCnt > 3)
            return diagnostic(error, cap, -3, "GOP mode/B/LT graph is not qualified for live mutation");
        if (control >= 26) {
            __typeof__(&IMP_Encoder_SetChnGopAttr) function;
            memcpy(&function, &setter, sizeof(function));
            attr.uMaxSameSenceCnt = (uint32_t)value;
            SDK_CALL(function(channel, &attr));
            break;
        }
        int32_t fps;
        int rc = get(sdk, control-4, &fps, error, cap);
        if (rc) return rc;
        __typeof__(&IMP_Encoder_SetChnGopLength) function;
        memcpy(&function, &setter, sizeof(function));
        SDK_CALL(function(channel, value*fps)); break;
    }
    default: return unsupported(control, error, cap);
    }
    int32_t active;
    int rc = get(sdk, control, &active, error, cap);
    if (rc) return rc;
    if (active == value) return 0;
    return diagnostic(error, cap, control >= 20 && (control <= 21 || control >= 24) ? 1 : -1,
                      "control %u readback %d differs from request %d", control, active, value);
}

int t31_controls_get(void *sdk, unsigned int control, int32_t *out, char *error, uint32_t cap) {
    if (!sdk || !out || (cap && !error)) return -1;
    if (cap) error[0] = '\0';
    if (__atomic_exchange_n(&busy, 1U, __ATOMIC_ACQUIRE))
        return diagnostic(error, cap, -1, "controls owner is busy");
    int32_t value;
    int result = get(sdk, control, &value, error, cap);
    if (!result) *out = value;
    __atomic_store_n(&busy, 0U, __ATOMIC_RELEASE);
    return result;
}
int t31_controls_set(void *sdk, unsigned int control, int32_t value, char *error, uint32_t cap) {
    if (!sdk || (cap && !error)) return -1;
    if (cap) error[0] = '\0';
    if (__atomic_exchange_n(&busy, 1U, __ATOMIC_ACQUIRE))
        return diagnostic(error, cap, -1, "controls owner is busy");
    int result = set(sdk, control, value, error, cap);
    __atomic_store_n(&busy, 0U, __ATOMIC_RELEASE);
    return result;
}
