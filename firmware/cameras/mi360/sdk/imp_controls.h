#ifndef MI360_IMP_CONTROLS_H
#define MI360_IMP_CONTROLS_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

enum t31_control {
    T31_CONTROL_RAW_HV = 0,
    T31_CONTROL_BRIGHTNESS = 1,
    T31_CONTROL_CONTRAST = 2,
    T31_CONTROL_SATURATION = 3,
    T31_CONTROL_SHARPNESS = 4,
    T31_CONTROL_ANTIFLICKER = 5,
    T31_CONTROL_MIC_VOLUME = 10,
    T31_CONTROL_SPEAKER_VOLUME = 11,
    T31_CONTROL_MIC_GAIN = 12,
    T31_CONTROL_SPEAKER_GAIN = 13,
    T31_CONTROL_MIC_MUTE = 14,
    T31_CONTROL_SPEAKER_MUTE = 15,
    T31_CONTROL_MAIN_FPS = 20,
    T31_CONTROL_SUB_FPS = 21,
    T31_CONTROL_MAIN_BITRATE_KBPS = 22,
    T31_CONTROL_SUB_BITRATE_KBPS = 23,
    T31_CONTROL_MAIN_GOP = 24,
    T31_CONTROL_SUB_GOP = 25,
    T31_CONTROL_MAIN_IDR_MULTIPLIER = 26,
    T31_CONTROL_SUB_IDR_MULTIPLIER = 27
};

#define T31_CONTROL_OK 0
#define T31_CONTROL_PENDING 1
#define T31_CONTROL_ACCEPTED_UNVERIFIED 2
#define T31_CONTROL_ERROR (-1)
#define T31_CONTROL_UNSUPPORTED (-3)

/* Borrow the already pinned/import-qualified libimp handle from the sole live
 * media owner. Caller serializes against audio/video/frame users and teardown.
 * No dlopen, System/ISP init, new ISP fd, effect, endpoint or worker creation.
 * Valid, nonoverlapping buffers must remain alive through the call. error may
 * be NULL only when cap=0. get leaves *out unchanged on every nonzero result.
 *
 * IDs0..5: raw HV0..3; brightness/contrast/saturation/sharpness0..255;
 * antiflicker0=off,1=50Hz,2=60Hz. ISP readbacks are driver-reported values,
 * not optical acceptance. The physical orientation mapping is caller-owned.
 * Volume10/11: -30..120,60 unity,0.5dB/step,-30 special silence. GetVol is
 * cached requested level, independent of mute. SetVol can undo SDK PCM mute;
 * application must own/reapply mute. Gain12/13 are unsupported: SDK masks
 * hardware errors and GetGain only returns its cache. Mute14/15 set accepts
 * 0/1 and returns ACCEPTED_UNVERIFIED; get is unsupported (no SDK getter).
 * Mute changes PCM amplitude; it does not disable physical microphone capture.
 *
 * FPS20/21: integer1..20 in the qualified current20fps dual graph. Fractional
 * active readback is Unsupported. GOP24/25: integer duration1..15 seconds,
 * converted to duration*active integer FPS in the SDK. Get requires a positive
 * raw GOP exactly divisible by active FPS. Default GOP mode/no B/LT and the
 * same-scene multiplier1..3 only. IDs26/27 expose that multiplier1..3;
 * setter copies the current GOP record and changes only uMaxSameSenceCnt.
 * The configured maximum IDR cadence is GOP duration*multiplier (actual stream
 * still needs bitstream acceptance). Serialize FPS/GOP/multiplier changes per
 * channel. GOP duration setters preserve the multiplier. Queued setters return
 * PENDING until an independent active getter matches; caller bounds polling.
 *
 * Bitrate22/23: kbps scalar; setter converts to raw bits/s for this exact API.
 * VBR/capped modes only,1..existing maxBitRate, preserving that ceiling;
 * maxBitRate must also fit signed32 bits/s. CBR get works but CBR set is
 * Unsupported because this stateless adapter cannot prove its allocation
 * ceiling. FIXQP has no bitrate getter/setter capability. RC getter is cached
 * config, not actual stream bitrate or independent hardware verification.
 *
 * Every readable set preflights both symbols and a valid current readback,
 * invokes setter once, then reads independently. Return0 only if that readback
 * matches; no requested-value substitution. Other mismatches/errors can follow
 * a state-changing call: do not claim rollback or retry automatically. No sleep
 * or retry loops; SDK calls themselves can block. A nonwaiting local gate also
 * rejects reentry; media-wide serialization remains the caller's responsibility.
 */
int t31_controls_get(void *sdk, unsigned int control, int32_t *out,
                     char *error, uint32_t cap);
int t31_controls_set(void *sdk, unsigned int control, int32_t value,
                     char *error, uint32_t cap);

#ifdef __cplusplus
}
#endif
#endif
