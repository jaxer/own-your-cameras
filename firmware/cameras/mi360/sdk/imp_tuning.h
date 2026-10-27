#ifndef MI360_IMP_TUNING_H
#define MI360_IMP_TUNING_H
#include "imp_controls.h"

#ifdef __cplusplus
extern "C" {
#endif

enum t31_tuning_control {
    T31_TUNING_HUE = 30,
    T31_TUNING_AE_COMPENSATION = 31,
    T31_TUNING_HIGHLIGHT_SUPPRESSION = 32,
    T31_TUNING_DPC_STRENGTH = 33,
    T31_TUNING_DRC_STRENGTH = 34,
    T31_TUNING_WB_PRESET = 35,
    T31_TUNING_ADR_ENABLED = 36,
    T31_TUNING_2D_DENOISE_ENABLED = 37,
    T31_TUNING_3D_DENOISE_ENABLED = 38,
    T31_TUNING_AE_LUMA = 50,
    T31_TUNING_EXPOSURE_US = 51
};

/* Borrow the exact pinned SDK from the sole initialized media owner. Caller
 * serializes these calls with all controls, frames and media teardown. This
 * adapter opens nothing, initializes nothing and starts no worker. A local
 * nonwaiting gate rejects reentry; it does not replace media-wide serialization.
 * sdk/error/output allocations must remain valid and disjoint through the call;
 * error may be NULL only when cap=0. get leaves *out unchanged on every error.
 * Return codes are the T31_CONTROL_* values from imp_controls.h: this adapter
 * returns only OK, ERROR or UNSUPPORTED, never optimistic/pending success.
 *
 * Hue/AE compensation/DPC/DRC strengths:0..255. AE compensation is an AE target,
 * not EV stops. HLC:0..10; enabling it displaces the vendor backlight setting.
 * These SDK getters read driver tuning state, not physical pixels. DRC strength
 * does not imply ADR enabled. The defective IMP_ISP_Tuning_EnableDRC is not used.
 *
 * WB presets are raw modes0(auto),2(daylight),3(cloudy),4(incandescent),
 * 5(fluorescent),6(twilight),7(shade),8(warm fluorescent). Requests for manual1
 * or custom9 are Unsupported. Unknown/current unsupported modes refuse mutation.
 * Set reads the complete8-byte WB record and preserves both gains in its input;
 * post-readback must match mode. Preset/auto algorithms may legitimately change
 * returned gains, whose safe manual scale/range is not qualified here.
 *
 * IDs36..38 are logical0/1 module enables: inverse bypass bits7(ADR),17(SDNS),
 * 31(MDNS). Public GetModuleControl reads actual module register0xc low19 bits
 * plus the MDNS enable parameter; this is stronger than an SDK requested cache,
 * but not optical proof. Set preserves every other reported bit and requires
 * full-mask readback equality. Kernel preserves reserved register bits19..31
 * independently; reported bits19..30 must be zero. The ADR control changes bypass
 * only: it is not sensor HDR/WDR and does not initialize a new algorithm/profile.
 * MDNS readback is a driver parameter, not a directly read register bit. Exact
 * parameter0x180 is mdns_y_filter_en_array: this switch governs that temporal
 * luma filter, leaving separate UV filter settings untouched. It does not prove
 * that every form of temporal denoising is disabled.
 *
 * IDs50/51 are read-only AE statistic/raw signed luma and exposure microseconds
 * from the exact24-byte EV record. Luma is not calibrated lux; no invented range
 * is imposed. Exposure exceeding INT32_MAX is an error, never truncated.
 *
 * A set resolves both symbols and validates an actual current readback before
 * mutation. A no-op returns that observed state; otherwise it sets once and
 * independently re-reads. Any mismatch or SDK error is failure even if hardware
 * already changed. Never automatically retry or claim rollback on that result.
 * No waits/retries are added; SDK/kernel calls themselves are not time-bounded.
 */
int t31_tuning_get(void *sdk, unsigned int control, int32_t *out,
                   char *error, uint32_t cap);
int t31_tuning_set(void *sdk, unsigned int control, int32_t value,
                   char *error, uint32_t cap);

#ifdef __cplusplus
}
#endif
#endif
