/* Maintained Mi360 IMP 1.1.2 ABI declarations for T31/SC2336.
 * No SDK implementation. Layout/prototype contracts are checked by the bridge
 * tests.
 */
#ifndef MI360_IMP_ABI_H
#define MI360_IMP_ABI_H
#include <stdbool.h>
#include <stdint.h>

typedef enum {
  DEV_ID_FS,
  DEV_ID_ENC,
  DEV_ID_DEC,
  DEV_ID_IVS,
  DEV_ID_OSD,
  DEV_ID_FG1DIRECT,
  DEV_ID_RESERVED_START,
  DEV_ID_RESERVED_END = 23,
  NR_MAX_DEVICES,
} IMPDeviceID;

typedef struct {
  IMPDeviceID deviceID;
  int groupID;
  int outputID;
} IMPCell;

typedef enum {
  PIX_FMT_YUV420P,
  PIX_FMT_YUYV422,
  PIX_FMT_UYVY422,
  PIX_FMT_YUV422P,
  PIX_FMT_YUV444P,
  PIX_FMT_YUV410P,
  PIX_FMT_YUV411P,
  PIX_FMT_GRAY8,
  PIX_FMT_MONOWHITE,
  PIX_FMT_MONOBLACK,
  PIX_FMT_NV12,
  PIX_FMT_NV21,
  PIX_FMT_RGB24,
  PIX_FMT_BGR24,
  PIX_FMT_ARGB,
  PIX_FMT_RGBA,
  PIX_FMT_ABGR,
  PIX_FMT_BGRA,
  PIX_FMT_RGB565BE,
  PIX_FMT_RGB565LE,
  PIX_FMT_RGB555BE,
  PIX_FMT_RGB555LE,
  PIX_FMT_BGR565BE,
  PIX_FMT_BGR565LE,
  PIX_FMT_BGR555BE,
  PIX_FMT_BGR555LE,
  PIX_FMT_0RGB,
  PIX_FMT_RGB0,
  PIX_FMT_0BGR,
  PIX_FMT_BGR0,
  PIX_FMT_BAYER_BGGR8,
  PIX_FMT_BAYER_RGGB8,
  PIX_FMT_BAYER_GBRG8,
  PIX_FMT_BAYER_GRBG8,
  PIX_FMT_RAW,
  PIX_FMT_HSV,
  PIX_FMT_NB,
} IMPPixelFormat;

int IMP_System_Init(void);

int IMP_System_Exit(void);

int IMP_System_Bind(IMPCell *srcCell, IMPCell *dstCell);

int IMP_System_UnBind(IMPCell *srcCell, IMPCell *dstCell);

typedef struct {
  int enable;
  int left;
  int top;
  int width;
  int height;
} IMPFSChnCrop;

typedef struct {
  int enable;
  int outwidth;
  int outheight;
} IMPFSChnScaler;

typedef enum {
  FS_PHY_CHANNEL,
  FS_EXT_CHANNEL,
} IMPFSChnType;

typedef struct {
  int picWidth;
  int picHeight;
  IMPPixelFormat pixFmt;
  IMPFSChnCrop crop;
  IMPFSChnScaler scaler;
  int outFrmRateNum;
  int outFrmRateDen;
  int nrVBs;
  IMPFSChnType type;
  IMPFSChnCrop fcrop;
} IMPFSChnAttr;

int IMP_FrameSource_CreateChn(int chnNum, IMPFSChnAttr *chn_attr);

int IMP_FrameSource_DestroyChn(int chnNum);

int IMP_FrameSource_EnableChn(int chnNum);

int IMP_FrameSource_DisableChn(int chnNum);

int IMP_FrameSource_SetChnAttr(int chnNum, const IMPFSChnAttr *chnAttr);

typedef enum {
  IMP_H264_NAL_UNKNOWN = 0,
  IMP_H264_NAL_SLICE = 1,
  IMP_H264_NAL_SLICE_DPA = 2,
  IMP_H264_NAL_SLICE_DPB = 3,
  IMP_H264_NAL_SLICE_DPC = 4,
  IMP_H264_NAL_SLICE_IDR = 5,
  IMP_H264_NAL_SEI = 6,
  IMP_H264_NAL_SPS = 7,
  IMP_H264_NAL_PPS = 8,
  IMP_H264_NAL_AUD = 9,
  IMP_H264_NAL_FILLER = 12,
} IMPEncoderH264NaluType;

typedef enum {
  IMP_H265_NAL_SLICE_TRAIL_N = 0,
  IMP_H265_NAL_SLICE_TRAIL_R = 1,
  IMP_H265_NAL_SLICE_TSA_N = 2,
  IMP_H265_NAL_SLICE_TSA_R = 3,
  IMP_H265_NAL_SLICE_STSA_N = 4,
  IMP_H265_NAL_SLICE_STSA_R = 5,
  IMP_H265_NAL_SLICE_RADL_N = 6,
  IMP_H265_NAL_SLICE_RADL_R = 7,
  IMP_H265_NAL_SLICE_RASL_N = 8,
  IMP_H265_NAL_SLICE_RASL_R = 9,
  IMP_H265_NAL_SLICE_BLA_W_LP = 16,
  IMP_H265_NAL_SLICE_BLA_W_RADL = 17,
  IMP_H265_NAL_SLICE_BLA_N_LP = 18,
  IMP_H265_NAL_SLICE_IDR_W_RADL = 19,
  IMP_H265_NAL_SLICE_IDR_N_LP = 20,
  IMP_H265_NAL_SLICE_CRA = 21,
  IMP_H265_NAL_VPS = 32,
  IMP_H265_NAL_SPS = 33,
  IMP_H265_NAL_PPS = 34,
  IMP_H265_NAL_AUD = 35,
  IMP_H265_NAL_EOS = 36,
  IMP_H265_NAL_EOB = 37,
  IMP_H265_NAL_FILLER_DATA = 38,
  IMP_H265_NAL_PREFIX_SEI = 39,
  IMP_H265_NAL_SUFFIX_SEI = 40,
  IMP_H265_NAL_INVALID = 64,
} IMPEncoderH265NaluType;

typedef union {
  IMPEncoderH264NaluType h264NalType;
  IMPEncoderH265NaluType h265NalType;
} IMPEncoderNalType;

typedef enum {
  IMP_ENC_SLICE_SI = 4,
  IMP_ENC_SLICE_SP = 3,
  IMP_ENC_SLICE_GOLDEN = 3,
  IMP_ENC_SLICE_I = 2,
  IMP_ENC_SLICE_P = 1,
  IMP_ENC_SLICE_B = 0,
  IMP_ENC_SLICE_CONCEAL = 6,
  IMP_ENC_SLICE_SKIP = 7,
  IMP_ENC_SLICE_REPEAT = 8,
  IMP_ENC_SLICE_MAX_ENUM,
} IMPEncoderSliceType;

typedef struct {
  uint32_t offset;
  uint32_t length;
  int64_t timestamp;
  _Bool frameEnd;
  IMPEncoderNalType nalType;
  IMPEncoderSliceType sliceType;
} IMPEncoderPack;

typedef struct {
  uint32_t phyAddr;
  uint32_t virAddr;
  uint32_t streamSize;
  IMPEncoderPack *pack;
  uint32_t packCount;
  uint32_t seq;
} IMPEncoderStream;

typedef enum {
  IMP_ENC_TYPE_AVC = 0,
  IMP_ENC_TYPE_HEVC = 1,
  IMP_ENC_TYPE_JPEG = 4,
} IMPEncoderEncType;

typedef enum {
  IMP_ENC_PROFILE_AVC_BASELINE = ((IMP_ENC_TYPE_AVC << 24) | (66)),
  IMP_ENC_PROFILE_AVC_MAIN = ((IMP_ENC_TYPE_AVC << 24) | (77)),
  IMP_ENC_PROFILE_AVC_HIGH = ((IMP_ENC_TYPE_AVC << 24) | (100)),
  IMP_ENC_PROFILE_HEVC_MAIN = ((IMP_ENC_TYPE_HEVC << 24) | (1)),
  IMP_ENC_PROFILE_JPEG = (IMP_ENC_TYPE_JPEG << 24),
} IMPEncoderProfile;

typedef enum {
  IMP_ENC_PIC_FORMAT_400_8BITS = 0x0088,
  IMP_ENC_PIC_FORMAT_420_8BITS = 0x0188,
  IMP_ENC_PIC_FORMAT_422_8BITS = 0x0288,
} IMPEncoderPicFormat;

typedef struct {
  _Bool enable;
  uint32_t x;
  uint32_t y;
  uint32_t w;
  uint32_t h;
} IMPEncoderCropCfg;

typedef struct {
  IMPEncoderProfile eProfile;
  uint8_t uLevel;
  uint8_t uTier;
  uint16_t uWidth;
  uint16_t uHeight;
  IMPEncoderPicFormat ePicFormat;
  uint32_t eEncOptions;
  uint32_t eEncTools;
  IMPEncoderCropCfg crop;
} IMPEncoderEncAttr;

typedef enum {
  IMP_ENC_GOP_CTRL_MODE_DEFAULT = 0x02,
  IMP_ENC_GOP_CTRL_MODE_PYRAMIDAL = 0x04,
  IMP_ENC_GOP_CTRL_MAX_ENUM = 0xff,
} IMPEncoderGopCtrlMode;

typedef struct {
  IMPEncoderGopCtrlMode uGopCtrlMode;
  uint16_t uGopLength;
  uint8_t uNumB;
  uint32_t uMaxSameSenceCnt;
  _Bool bEnableLT;
  uint32_t uFreqLT;
  _Bool bLTRC;
} IMPEncoderGopAttr;

typedef enum {
  IMP_ENC_RC_MODE_FIXQP = 0x0,
  IMP_ENC_RC_MODE_CBR = 0x1,
  IMP_ENC_RC_MODE_VBR = 0x2,
  IMP_ENC_RC_MODE_CAPPED_VBR = 0x4,
  IMP_ENC_RC_MODE_CAPPED_QUALITY = 0x8,
  IMP_ENC_RC_MODE_INVALID = 0xff,
} IMPEncoderRcMode;

typedef struct {
  int16_t iInitialQP;
} IMPEncoderAttrFixQP;

typedef struct {
  uint32_t uTargetBitRate;
  int16_t iInitialQP;
  int16_t iMinQP;
  int16_t iMaxQP;
  int16_t iIPDelta;
  int16_t iPBDelta;
  uint32_t eRcOptions;
  uint32_t uMaxPictureSize;
} IMPEncoderAttrCbr;

typedef struct {
  uint32_t uTargetBitRate;
  uint32_t uMaxBitRate;
  int16_t iInitialQP;
  int16_t iMinQP;
  int16_t iMaxQP;
  int16_t iIPDelta;
  int16_t iPBDelta;
  uint32_t eRcOptions;
  uint32_t uMaxPictureSize;
} IMPEncoderAttrVbr;

typedef struct {
  uint32_t uTargetBitRate;
  uint32_t uMaxBitRate;
  int16_t iInitialQP;
  int16_t iMinQP;
  int16_t iMaxQP;
  int16_t iIPDelta;
  int16_t iPBDelta;
  uint32_t eRcOptions;
  uint32_t uMaxPictureSize;
  uint16_t uMaxPSNR;
} IMPEncoderAttrCappedVbr;

typedef IMPEncoderAttrCappedVbr IMPEncoderAttrCappedQuality;

typedef struct {
  IMPEncoderRcMode rcMode;
  union {
    IMPEncoderAttrFixQP attrFixQp;
    IMPEncoderAttrCbr attrCbr;
    IMPEncoderAttrVbr attrVbr;
    IMPEncoderAttrCappedVbr attrCappedVbr;
    IMPEncoderAttrCappedQuality attrCappedQuality;
  };
} IMPEncoderAttrRcMode;

typedef struct {
  uint32_t frmRateNum;
  uint32_t frmRateDen;
} IMPEncoderFrmRate;

typedef struct {
  IMPEncoderAttrRcMode attrRcMode;
  IMPEncoderFrmRate outFrmRate;
} IMPEncoderRcAttr;

typedef struct {
  IMPEncoderEncAttr encAttr;
  IMPEncoderRcAttr rcAttr;
  IMPEncoderGopAttr gopAttr;
} IMPEncoderChnAttr;

typedef struct {
  _Bool registered;
  uint32_t leftPics;
  uint32_t leftStreamBytes;
  uint32_t leftStreamFrames;
  uint32_t curPacks;
  uint32_t work_done;
} IMPEncoderChnStat;

int IMP_Encoder_CreateGroup(int encGroup);

int IMP_Encoder_DestroyGroup(int encGroup);

int IMP_Encoder_SetDefaultParam(IMPEncoderChnAttr *chnAttr,
                                IMPEncoderProfile profile,
                                IMPEncoderRcMode rcMode, uint16_t uWidth,
                                uint16_t uHeight, uint32_t frmRateNum,
                                uint32_t frmRateDen, uint32_t uGopLength,
                                int uMaxSameSenceCnt, int iInitialQP,
                                uint32_t uTargetBitRate);

int IMP_Encoder_CreateChn(int encChn, const IMPEncoderChnAttr *attr);

int IMP_Encoder_DestroyChn(int encChn);

int IMP_Encoder_RegisterChn(int encGroup, int encChn);

int IMP_Encoder_UnRegisterChn(int encChn);

int IMP_Encoder_StartRecvPic(int encChn);

int IMP_Encoder_StopRecvPic(int encChn);

int IMP_Encoder_GetStream(int encChn, IMPEncoderStream *stream,
                          _Bool blockFlag);

int IMP_Encoder_ReleaseStream(int encChn, IMPEncoderStream *stream);

int IMP_Encoder_PollingStream(int encChn, uint32_t timeoutMsec);

int IMP_Encoder_SetMaxStreamCnt(int encChn, int nrMaxStream);

int IMP_Encoder_GetChnFrmRate(int encChn, IMPEncoderFrmRate *pstFps);

int IMP_Encoder_SetChnFrmRate(int encChn, const IMPEncoderFrmRate *pstFps);

int IMP_Encoder_SetChnBitRate(int encChn, int iTargetBitRate, int iMaxBitRate);

int IMP_Encoder_SetChnGopLength(int encChn, int iGopLength);

int IMP_Encoder_GetChnAttrRcMode(int encChn,
                                 IMPEncoderAttrRcMode *pstRcModeCfg);

int IMP_Encoder_GetChnGopAttr(int encChn, IMPEncoderGopAttr *pGopAttr);

int IMP_Encoder_SetChnGopAttr(int encChn, const IMPEncoderGopAttr *pGopAttr);

typedef enum {
  TX_SENSOR_CONTROL_INTERFACE_I2C = 1,
  TX_SENSOR_CONTROL_INTERFACE_SPI,
} IMPSensorControlBusType;

typedef struct {
  char type[20];
  int addr;
  int i2c_adapter_id;
} IMPI2CInfo;

typedef struct {
  char modalias[32];
  int bus_num;
} IMPSPIInfo;

typedef struct {
  char name[32];
  IMPSensorControlBusType cbus_type;
  union {
    IMPI2CInfo i2c;
    IMPSPIInfo spi;
  };
  unsigned short rst_gpio;
  unsigned short pwdn_gpio;
  unsigned short power_gpio;
} IMPSensorInfo;

int IMP_ISP_Open(void);

int IMP_ISP_Close(void);

int IMP_ISP_AddSensor(IMPSensorInfo *pinfo);

int IMP_ISP_DelSensor(IMPSensorInfo *pinfo);

int IMP_ISP_EnableSensor(void);

int IMP_ISP_DisableSensor(void);

typedef enum {
  IMPISP_TUNING_OPS_MODE_DISABLE,
  IMPISP_TUNING_OPS_MODE_ENABLE,
  IMPISP_TUNING_OPS_MODE_BUTT,
} IMPISPTuningOpsMode;

int IMP_ISP_EnableTuning(void);

int IMP_ISP_DisableTuning(void);

typedef enum {
  IMPISP_ANTIFLICKER_DISABLE,
  IMPISP_ANTIFLICKER_50HZ,
  IMPISP_ANTIFLICKER_60HZ,
  IMPISP_ANTIFLICKER_BUTT,
} IMPISPAntiflickerAttr;

int IMP_ISP_Tuning_SetAntiFlickerAttr(IMPISPAntiflickerAttr attr);

int IMP_ISP_Tuning_GetAntiFlickerAttr(IMPISPAntiflickerAttr *pattr);

int IMP_ISP_Tuning_SetBrightness(unsigned char bright);

int IMP_ISP_Tuning_GetBrightness(unsigned char *pbright);

int IMP_ISP_Tuning_SetContrast(unsigned char contrast);

int IMP_ISP_Tuning_GetContrast(unsigned char *pcontrast);

int IMP_ISP_Tuning_SetSharpness(unsigned char sharpness);

int IMP_ISP_Tuning_GetSharpness(unsigned char *psharpness);

int IMP_ISP_Tuning_SetBcshHue(unsigned char hue);

int IMP_ISP_Tuning_GetBcshHue(unsigned char *hue);

int IMP_ISP_Tuning_SetSaturation(unsigned char sat);

int IMP_ISP_Tuning_GetSaturation(unsigned char *psat);

typedef enum {
  IMPISP_RUNNING_MODE_DAY = 0,
  IMPISP_RUNNING_MODE_NIGHT = 1,
  IMPISP_RUNNING_MODE_BUTT,
} IMPISPRunningMode;

int IMP_ISP_Tuning_SetISPRunningMode(IMPISPRunningMode mode);

int IMP_ISP_Tuning_SetAeComp(int comp);

int IMP_ISP_Tuning_GetAeComp(int *comp);

int IMP_ISP_Tuning_GetAeLuma(int *luma);

enum isp_core_wb_mode {
  ISP_CORE_WB_MODE_AUTO = 0,
  ISP_CORE_WB_MODE_MANUAL,
  ISP_CORE_WB_MODE_DAY_LIGHT,
  ISP_CORE_WB_MODE_CLOUDY,
  ISP_CORE_WB_MODE_INCANDESCENT,
  ISP_CORE_WB_MODE_FLOURESCENT,
  ISP_CORE_WB_MODE_TWILIGHT,
  ISP_CORE_WB_MODE_SHADE,
  ISP_CORE_WB_MODE_WARM_FLOURESCENT,
  ISP_CORE_WB_MODE_CUSTOM,
};

typedef struct isp_core_wb_attr {
  enum isp_core_wb_mode mode;
  uint16_t rgain;
  uint16_t bgain;
} IMPISPWB;

int IMP_ISP_Tuning_SetWB(IMPISPWB *wb);

int IMP_ISP_Tuning_GetWB(IMPISPWB *wb);

int IMP_ISP_Tuning_SetHiLightDepress(uint32_t strength);

int IMP_ISP_Tuning_GetHiLightDepress(uint32_t *strength);

typedef struct {
  uint32_t ev;
  uint32_t expr_us;
  uint32_t ev_log2;
  uint32_t again;
  uint32_t dgain;
  uint32_t gain_log2;
} IMPISPEVAttr;

int IMP_ISP_Tuning_GetEVAttr(IMPISPEVAttr *attr);

typedef union {
  unsigned int key;
  struct {
    unsigned int bitBypassBLC : 1;
    unsigned int bitBypassGIB : 1;
    unsigned int bitBypassAG : 1;
    unsigned int bitBypassWDR : 1;
    unsigned int bitBypassDPC : 1;
    unsigned int bitBypassRDNS : 1;
    unsigned int bitBypassLSC : 1;
    unsigned int bitBypassADR : 1;
    unsigned int bitBypassDMSC : 1;
    unsigned int bitBypassCCM : 1;
    unsigned int bitBypassGAMMA : 1;
    unsigned int bitBypassDEFOG : 1;
    unsigned int bitBypassCSC : 1;
    unsigned int bitBypassCLM : 1;
    unsigned int bitBypassSP : 1;
    unsigned int bitBypassYDNS : 1;
    unsigned int bitBypassBCSH : 1;
    unsigned int bitBypassSDNS : 1;
    unsigned int bitBypassHLDC : 1;
    unsigned int bitRsv : 12;
    unsigned int bitBypassMDNS : 1;
  };
} IMPISPModuleCtl;

int IMP_ISP_Tuning_SetModuleControl(IMPISPModuleCtl *ispmodule);

int IMP_ISP_Tuning_GetModuleControl(IMPISPModuleCtl *ispmodule);

int IMP_ISP_Tuning_SetDPC_Strength(unsigned int ratio);

int IMP_ISP_Tuning_GetDPC_Strength(unsigned int *ratio);

int IMP_ISP_Tuning_SetDRC_Strength(unsigned int ratio);

int IMP_ISP_Tuning_GetDRC_Strength(unsigned int *ratio);

typedef enum {
  IMPISP_FLIP_NORMAL_MODE = 0,
  IMPISP_FLIP_H_MODE = 1,
  IMPISP_FLIP_V_MODE = 2,
  IMPISP_FLIP_HV_MODE = 3,
  IMPISP_FLIP_MODE_BUTT,
} IMPISPHVFLIP;

int IMP_ISP_Tuning_SetHVFLIP(IMPISPHVFLIP hvflip);

int IMP_ISP_Tuning_GetHVFlip(IMPISPHVFLIP *hvflip);

int IMP_ISP_Tuning_EnableDRC(IMPISPTuningOpsMode mode);

typedef enum {
  BLOCK = 0,
  NOBLOCK = 1,
} IMPBlock;

typedef enum {
  AUDIO_SAMPLE_RATE_8000 = 8000,
  AUDIO_SAMPLE_RATE_16000 = 16000,
  AUDIO_SAMPLE_RATE_24000 = 24000,
  AUDIO_SAMPLE_RATE_32000 = 32000,
  AUDIO_SAMPLE_RATE_44100 = 44100,
  AUDIO_SAMPLE_RATE_48000 = 48000,
  AUDIO_SAMPLE_RATE_96000 = 96000,
} IMPAudioSampleRate;

typedef enum {
  AUDIO_BIT_WIDTH_16 = 16,
} IMPAudioBitWidth;

typedef enum {
  AUDIO_SOUND_MODE_MONO = 1,
  AUDIO_SOUND_MODE_STEREO = 2,
} IMPAudioSoundMode;

typedef struct {
  IMPAudioSampleRate samplerate;
  IMPAudioBitWidth bitwidth;
  IMPAudioSoundMode soundmode;
  int frmNum;
  int numPerFrm;
  int chnCnt;
} IMPAudioIOAttr;

typedef struct {
  IMPAudioBitWidth bitwidth;
  IMPAudioSoundMode soundmode;
  uint32_t *virAddr;
  uint32_t phyAddr;
  int64_t timeStamp;
  int seq;
  int len;
} IMPAudioFrame;

typedef struct {
  int usrFrmDepth;
  int Rev;
} IMPAudioIChnParam;

typedef struct {
  int chnTotalNum;
  int chnFreeNum;
  int chnBusyNum;
} IMPAudioOChnState;

typedef struct {
  int TargetLevelDbfs;
  int CompressionGaindB;
} IMPAudioAgcConfig;

int IMP_AI_SetPubAttr(int audioDevId, IMPAudioIOAttr *attr);

int IMP_AI_Enable(int audioDevId);

int IMP_AI_Disable(int audioDevId);

int IMP_AI_EnableChn(int audioDevId, int aiChn);

int IMP_AI_DisableChn(int audioDevId, int aiChn);

int IMP_AI_GetFrame(int audioDevId, int aiChn, IMPAudioFrame *frm,
                    IMPBlock block);

int IMP_AI_ReleaseFrame(int audioDevId, int aiChn, IMPAudioFrame *frm);

int IMP_AI_SetChnParam(int audioDevId, int aiChn, IMPAudioIChnParam *chnParam);

int IMP_AI_EnableAec(int aiDevId, int aiChn, int aoDevId, int aoChn);

int IMP_AI_DisableAec(int aiDevId, int aiChn);

int IMP_AI_EnableNs(IMPAudioIOAttr *attr, int mode);

int IMP_AI_DisableNs(void);

int IMP_AO_EnableAgc(IMPAudioIOAttr *attr, IMPAudioAgcConfig agcConfig);

int IMP_AO_DisableAgc(void);

int IMP_AO_SetPubAttr(int audioDevId, IMPAudioIOAttr *attr);

int IMP_AO_Enable(int audioDevId);

int IMP_AO_Disable(int audioDevId);

int IMP_AO_EnableChn(int audioDevId, int aoChn);

int IMP_AO_DisableChn(int audioDevId, int aoChn);

int IMP_AO_SendFrame(int audioDevId, int aoChn, IMPAudioFrame *data,
                     IMPBlock block);

int IMP_AO_QueryChnStat(int audioDevId, int aoChn, IMPAudioOChnState *status);

int IMP_AI_SetVol(int audioDevId, int aiChn, int aiVol);

int IMP_AI_GetVol(int audioDevId, int aiChn, int *vol);

int IMP_AI_SetVolMute(int audioDevId, int aiChn, int mute);

int IMP_AO_SetVol(int audioDevId, int aoChn, int aoVol);

int IMP_AO_GetVol(int audioDevId, int aoChn, int *vol);

int IMP_AO_SetVolMute(int audioDevId, int aoChn, int mute);

int IMP_AI_SetGain(int audioDevId, int aiChn, int aiGain);

int IMP_AO_SetGain(int audioDevId, int aoChn, int aoGain);
#endif
