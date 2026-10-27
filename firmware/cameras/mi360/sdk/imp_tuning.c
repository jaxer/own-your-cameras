/* Exact shipped IMP1.1.2 tuning ABI; no lifecycle/device ownership here. */
#include "imp_tuning.h"
#include <dlfcn.h>
#include <imp_abi.h>
#include <stddef.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#ifndef T31_TUNING_HOST_TEST
_Static_assert(sizeof(void *) == 4, "O32 pointers");
#endif
_Static_assert(sizeof(int) == 4 && sizeof(unsigned int) == 4, "scalar ABI");
_Static_assert(sizeof(&IMP_ISP_Tuning_GetWB) == sizeof(void *) &&
               sizeof(&IMP_ISP_Tuning_SetModuleControl) == sizeof(void *) &&
               sizeof(&IMP_ISP_Tuning_GetEVAttr) == sizeof(void *), "function pointer ABI");
_Static_assert(sizeof(IMPISPWB) == 8 && _Alignof(IMPISPWB) == 4 &&
               offsetof(IMPISPWB, mode) == 0 && offsetof(IMPISPWB, rgain) == 4 &&
               offsetof(IMPISPWB, bgain) == 6, "WB O32 layout");
_Static_assert(ISP_CORE_WB_MODE_AUTO == 0 && ISP_CORE_WB_MODE_MANUAL == 1 &&
               ISP_CORE_WB_MODE_DAY_LIGHT == 2 && ISP_CORE_WB_MODE_CLOUDY == 3 &&
               ISP_CORE_WB_MODE_INCANDESCENT == 4 && ISP_CORE_WB_MODE_FLOURESCENT == 5 &&
               ISP_CORE_WB_MODE_TWILIGHT == 6 && ISP_CORE_WB_MODE_SHADE == 7 &&
               ISP_CORE_WB_MODE_WARM_FLOURESCENT == 8 && ISP_CORE_WB_MODE_CUSTOM == 9,
               "WB mode ABI");
_Static_assert(sizeof(IMPISPEVAttr) == 24 && _Alignof(IMPISPEVAttr) == 4 &&
               offsetof(IMPISPEVAttr, ev) == 0 && offsetof(IMPISPEVAttr, expr_us) == 4 &&
               offsetof(IMPISPEVAttr, ev_log2) == 8 && offsetof(IMPISPEVAttr, again) == 12 &&
               offsetof(IMPISPEVAttr, dgain) == 16 && offsetof(IMPISPEVAttr, gain_log2) == 20,
               "EV O32 layout");
_Static_assert(sizeof(IMPISPModuleCtl) == 4 && _Alignof(IMPISPModuleCtl) == 4 &&
               offsetof(IMPISPModuleCtl, key) == 0, "module mask O32 layout");

/* imp_isp.h plus shipped tx-isp-t31 tisp_{s,g}_module_control@62388/62418:
 * ADR bit7 and SDNS bit17 are register bypasses; MDNS bit31 is synthesized from
 * parameter0x180's inverse enable. Avoid relying on host C bitfield allocation. */
#define MODULE_REPORTED_BITS UINT32_C(0x8007ffff)
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
static int unsupported(unsigned int control, char *error, uint32_t cap) {
    return diagnostic(error, cap, T31_CONTROL_UNSUPPORTED,
                      "tuning control %u has no qualified capability", control);
}
static int resolve(void *sdk, void *function, size_t size, const char *name,
                   char *error, uint32_t cap) {
    (void)dlerror();
    void *symbol = dlsym(sdk, name);
    const char *failure = dlerror();
    if (!symbol || failure || size != sizeof(symbol))
        return diagnostic(error, cap, T31_CONTROL_ERROR, "missing qualified SDK symbol %s", name);
    memcpy(function, &symbol, size);
    return 0;
}
#define RESOLVE(name) \
    __typeof__(&name) name##_fn; \
    if (resolve(sdk, &name##_fn, sizeof(name##_fn), #name, error, cap)) return T31_CONTROL_ERROR
#define SDK_CALL(expression) do { \
    int sdk_result = (expression); \
    if (sdk_result) return diagnostic(error, cap, T31_CONTROL_ERROR, \
                                    "%s failed (%d)", #expression, sdk_result); \
} while (0)

struct scalar {
    const char *get, *set;
    int32_t maximum;
};
static const struct scalar scalars[] = {
    {"IMP_ISP_Tuning_GetBcshHue", "IMP_ISP_Tuning_SetBcshHue", 255},
    {"IMP_ISP_Tuning_GetAeComp", "IMP_ISP_Tuning_SetAeComp", 255},
    {"IMP_ISP_Tuning_GetHiLightDepress", "IMP_ISP_Tuning_SetHiLightDepress", 10},
    {"IMP_ISP_Tuning_GetDPC_Strength", "IMP_ISP_Tuning_SetDPC_Strength", 255},
    {"IMP_ISP_Tuning_GetDRC_Strength", "IMP_ISP_Tuning_SetDRC_Strength", 255}
};
static int preset(int32_t mode) { return mode == 0 || (mode >= 2 && mode <= 8); }
static uint32_t module_bit(unsigned int control) {
    switch (control) {
    case T31_TUNING_ADR_ENABLED: return UINT32_C(1) << 7;
    case T31_TUNING_2D_DENOISE_ENABLED: return UINT32_C(1) << 17;
    case T31_TUNING_3D_DENOISE_ENABLED: return UINT32_C(1) << 31;
    default: return 0;
    }
}
static int scalar_get(void *function, unsigned int control, int32_t *out,
                      char *error, uint32_t cap) {
    int32_t value;
    const struct scalar *spec = &scalars[control - T31_TUNING_HUE];
    if (control == T31_TUNING_HUE) {
        __typeof__(&IMP_ISP_Tuning_GetBcshHue) call;
        memcpy(&call, &function, sizeof(call));
        unsigned char byte = 0;
        int rc = call(&byte);
        if (rc) return diagnostic(error, cap, -1, "%s failed (%d)", spec->get, rc);
        value = byte;
    } else if (control == T31_TUNING_AE_COMPENSATION) {
        __typeof__(&IMP_ISP_Tuning_GetAeComp) call;
        memcpy(&call, &function, sizeof(call));
        int number = INT32_MIN;
        int rc = call(&number);
        if (rc) return diagnostic(error, cap, -1, "%s failed (%d)", spec->get, rc);
        value = number;
    } else {
        __typeof__(&IMP_ISP_Tuning_GetHiLightDepress) call;
        memcpy(&call, &function, sizeof(call));
        uint32_t number = UINT32_MAX;
        int rc = call(&number);
        if (rc) return diagnostic(error, cap, -1, "%s failed (%d)", spec->get, rc);
        if (number > (uint32_t)spec->maximum)
            return diagnostic(error, cap, -1, "%s returned invalid scalar %u", spec->get, number);
        value = (int32_t)number;
    }
    if (value < 0 || value > spec->maximum)
        return diagnostic(error, cap, -1, "%s returned invalid scalar %d", spec->get, value);
    *out = value;
    return 0;
}
static int read_wb(void *function, IMPISPWB *out, char *error, uint32_t cap) {
    __typeof__(&IMP_ISP_Tuning_GetWB) call;
    memcpy(&call, &function, sizeof(call));
    IMPISPWB wb = {.mode = (enum isp_core_wb_mode)-1, .rgain = 0, .bgain = 0};
    int sdk_result = call(&wb);
    if (sdk_result)
        return diagnostic(error, cap, -1, "IMP_ISP_Tuning_GetWB failed (%d)", sdk_result);
    if (!preset((int32_t)wb.mode)) {
        if (wb.mode == ISP_CORE_WB_MODE_MANUAL || wb.mode == ISP_CORE_WB_MODE_CUSTOM)
            return diagnostic(error, cap, -3, "WB mode %d requires unqualified manual gains", (int)wb.mode);
        return diagnostic(error, cap, -1, "IMP_ISP_Tuning_GetWB returned invalid mode %d", (int)wb.mode);
    }
    *out = wb;
    return 0;
}
static int read_modules(void *function, uint32_t *out, char *error, uint32_t cap) {
    __typeof__(&IMP_ISP_Tuning_GetModuleControl) call;
    memcpy(&call, &function, sizeof(call));
    IMPISPModuleCtl attr = {.key = UINT32_MAX};
    int sdk_result = call(&attr);
    if (sdk_result)
        return diagnostic(error, cap, -1, "IMP_ISP_Tuning_GetModuleControl failed (%d)", sdk_result);
    if (attr.key & ~MODULE_REPORTED_BITS)
        return diagnostic(error, cap, -1, "IMP_ISP_Tuning_GetModuleControl returned reserved bits 0x%08x", attr.key);
    *out = attr.key;
    return 0;
}
static int get(void *sdk, unsigned int control, int32_t *out, char *error, uint32_t cap) {
    if (control >= T31_TUNING_HUE && control <= T31_TUNING_DRC_STRENGTH) {
        void *function;
        const struct scalar *spec = &scalars[control - T31_TUNING_HUE];
        if (resolve(sdk, &function, sizeof(function), spec->get, error, cap)) return -1;
        return scalar_get(function, control, out, error, cap);
    }
    if (control == T31_TUNING_WB_PRESET) {
        RESOLVE(IMP_ISP_Tuning_GetWB);
        void *function;
        memcpy(&function, &IMP_ISP_Tuning_GetWB_fn, sizeof(function));
        IMPISPWB wb;
        int rc = read_wb(function, &wb, error, cap);
        if (rc) return rc;
        *out = (int32_t)wb.mode;
        return 0;
    }
    if (module_bit(control)) {
        RESOLVE(IMP_ISP_Tuning_GetModuleControl);
        void *function;
        memcpy(&function, &IMP_ISP_Tuning_GetModuleControl_fn, sizeof(function));
        uint32_t bits;
        int rc = read_modules(function, &bits, error, cap);
        if (rc) return rc;
        *out = !(bits & module_bit(control));
        return 0;
    }
    if (control == T31_TUNING_AE_LUMA) {
        RESOLVE(IMP_ISP_Tuning_GetAeLuma);
        int luma = 0;
        SDK_CALL(IMP_ISP_Tuning_GetAeLuma_fn(&luma));
        *out = luma;
        return 0;
    }
    if (control == T31_TUNING_EXPOSURE_US) {
        RESOLVE(IMP_ISP_Tuning_GetEVAttr);
        IMPISPEVAttr attr = {0};
        SDK_CALL(IMP_ISP_Tuning_GetEVAttr_fn(&attr));
        if (attr.expr_us > INT32_MAX)
            return diagnostic(error, cap, -1, "IMP_ISP_Tuning_GetEVAttr exposure exceeds signed scalar range");
        *out = (int32_t)attr.expr_us;
        return 0;
    }
    return unsupported(control, error, cap);
}
static int set(void *sdk, unsigned int control, int32_t value, char *error, uint32_t cap) {
    if (control >= T31_TUNING_HUE && control <= T31_TUNING_DRC_STRENGTH) {
        const struct scalar *spec = &scalars[control - T31_TUNING_HUE];
        if (value < 0 || value > spec->maximum)
            return diagnostic(error, cap, -1, "tuning control %u scalar out of range", control);
        void *getter, *setter;
        if (resolve(sdk, &getter, sizeof(getter), spec->get, error, cap) ||
            resolve(sdk, &setter, sizeof(setter), spec->set, error, cap)) return -1;
        int32_t before, after;
        int rc = scalar_get(getter, control, &before, error, cap);
        if (rc) return rc;
        if (before == value) return 0;
        if (control == T31_TUNING_HUE) {
            __typeof__(&IMP_ISP_Tuning_SetBcshHue) call;
            memcpy(&call, &setter, sizeof(call));
            rc = call((unsigned char)value);
        } else if (control == T31_TUNING_AE_COMPENSATION) {
            __typeof__(&IMP_ISP_Tuning_SetAeComp) call;
            memcpy(&call, &setter, sizeof(call));
            rc = call(value);
        } else {
            __typeof__(&IMP_ISP_Tuning_SetHiLightDepress) call;
            memcpy(&call, &setter, sizeof(call));
            rc = call((uint32_t)value);
        }
        if (rc) return diagnostic(error, cap, -1, "%s failed (%d)", spec->set, rc);
        rc = scalar_get(getter, control, &after, error, cap);
        if (rc) return rc;
        if (after != value)
            return diagnostic(error, cap, -1, "%s readback %d differs from request %d", spec->set, after, value);
        return 0;
    }
    if (control == T31_TUNING_WB_PRESET) {
        if (value == 1 || value == 9)
            return diagnostic(error, cap, -3, "manual/custom WB gains are not qualified");
        if (!preset(value)) return diagnostic(error, cap, -1, "WB preset out of range");
        RESOLVE(IMP_ISP_Tuning_GetWB);
        RESOLVE(IMP_ISP_Tuning_SetWB);
        void *getter;
        memcpy(&getter, &IMP_ISP_Tuning_GetWB_fn, sizeof(getter));
        IMPISPWB before, after;
        int rc = read_wb(getter, &before, error, cap);
        if (rc) return rc;
        if ((int32_t)before.mode == value) return 0;
        before.mode = (enum isp_core_wb_mode)value;
        SDK_CALL(IMP_ISP_Tuning_SetWB_fn(&before));
        rc = read_wb(getter, &after, error, cap);
        if (rc) return rc;
        if ((int32_t)after.mode != value)
            return diagnostic(error, cap, -1, "IMP_ISP_Tuning_SetWB readback %d differs from request %d", (int)after.mode, value);
        return 0;
    }
    uint32_t bit = module_bit(control);
    if (bit) {
        if (value != 0 && value != 1)
            return diagnostic(error, cap, -1, "module enable must be0 or1");
        RESOLVE(IMP_ISP_Tuning_GetModuleControl);
        RESOLVE(IMP_ISP_Tuning_SetModuleControl);
        void *getter;
        memcpy(&getter, &IMP_ISP_Tuning_GetModuleControl_fn, sizeof(getter));
        uint32_t before, after;
        int rc = read_modules(getter, &before, error, cap);
        if (rc) return rc;
        uint32_t expected = value ? before & ~bit : before | bit;
        if (expected == before) return 0;
        IMPISPModuleCtl attr = {.key = expected};
        SDK_CALL(IMP_ISP_Tuning_SetModuleControl_fn(&attr));
        rc = read_modules(getter, &after, error, cap);
        if (rc) return rc;
        if (after != expected)
            return diagnostic(error, cap, -1, "module readback 0x%08x differs from expected0x%08x", after, expected);
        return 0;
    }
    return unsupported(control, error, cap);
}
int t31_tuning_get(void *sdk, unsigned int control, int32_t *out, char *error, uint32_t cap) {
    if (!sdk || !out || (cap && !error)) return T31_CONTROL_ERROR;
    if (cap) error[0] = '\0';
    if (__atomic_exchange_n(&busy, 1U, __ATOMIC_ACQUIRE))
        return diagnostic(error, cap, -1, "tuning owner is busy");
    int32_t value;
    int rc = get(sdk, control, &value, error, cap);
    if (!rc) *out = value;
    __atomic_store_n(&busy, 0U, __ATOMIC_RELEASE);
    return rc;
}
int t31_tuning_set(void *sdk, unsigned int control, int32_t value, char *error, uint32_t cap) {
    if (!sdk || (cap && !error)) return T31_CONTROL_ERROR;
    if (cap) error[0] = '\0';
    if (__atomic_exchange_n(&busy, 1U, __ATOMIC_ACQUIRE))
        return diagnostic(error, cap, -1, "tuning owner is busy");
    int rc = set(sdk, control, value, error, cap);
    __atomic_store_n(&busy, 0U, __ATOMIC_RELEASE);
    return rc;
}
