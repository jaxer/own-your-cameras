// Frozen ARM layout literals from the previously qualified header translation.
// They guard drift; they do not establish new binary ABI evidence.

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_UPOINT"][::core::mem::size_of::<_HD_UPOINT>() - 8usize];
    ["Alignment of _HD_UPOINT"][::core::mem::align_of::<_HD_UPOINT>() - 4usize];
    ["Offset of field: _HD_UPOINT::x"][::core::mem::offset_of!(_HD_UPOINT, x) - 0usize];
    ["Offset of field: _HD_UPOINT::y"][::core::mem::offset_of!(_HD_UPOINT, y) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_URECT"][::core::mem::size_of::<_HD_URECT>() - 16usize];
    ["Alignment of _HD_URECT"][::core::mem::align_of::<_HD_URECT>() - 4usize];
    ["Offset of field: _HD_URECT::x"][::core::mem::offset_of!(_HD_URECT, x) - 0usize];
    ["Offset of field: _HD_URECT::y"][::core::mem::offset_of!(_HD_URECT, y) - 4usize];
    ["Offset of field: _HD_URECT::w"][::core::mem::offset_of!(_HD_URECT, w) - 8usize];
    ["Offset of field: _HD_URECT::h"][::core::mem::offset_of!(_HD_URECT, h) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_IPOINT"][::core::mem::size_of::<_HD_IPOINT>() - 8usize];
    ["Alignment of _HD_IPOINT"][::core::mem::align_of::<_HD_IPOINT>() - 4usize];
    ["Offset of field: _HD_IPOINT::x"][::core::mem::offset_of!(_HD_IPOINT, x) - 0usize];
    ["Offset of field: _HD_IPOINT::y"][::core::mem::offset_of!(_HD_IPOINT, y) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_IRECT"][::core::mem::size_of::<_HD_IRECT>() - 16usize];
    ["Alignment of _HD_IRECT"][::core::mem::align_of::<_HD_IRECT>() - 4usize];
    ["Offset of field: _HD_IRECT::x"][::core::mem::offset_of!(_HD_IRECT, x) - 0usize];
    ["Offset of field: _HD_IRECT::y"][::core::mem::offset_of!(_HD_IRECT, y) - 4usize];
    ["Offset of field: _HD_IRECT::w"][::core::mem::offset_of!(_HD_IRECT, w) - 8usize];
    ["Offset of field: _HD_IRECT::h"][::core::mem::offset_of!(_HD_IRECT, h) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_BUFINFO"][::core::mem::size_of::<_HD_BUFINFO>() - 8usize];
    ["Alignment of _HD_BUFINFO"][::core::mem::align_of::<_HD_BUFINFO>() - 4usize];
    ["Offset of field: _HD_BUFINFO::phy_addr"]
        [::core::mem::offset_of!(_HD_BUFINFO, phy_addr) - 0usize];
    ["Offset of field: _HD_BUFINFO::buf_size"]
        [::core::mem::offset_of!(_HD_BUFINFO, buf_size) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_DEVCOUNT"][::core::mem::size_of::<_HD_DEVCOUNT>() - 4usize];
    ["Alignment of _HD_DEVCOUNT"][::core::mem::align_of::<_HD_DEVCOUNT>() - 4usize];
    ["Offset of field: _HD_DEVCOUNT::max_dev_count"]
        [::core::mem::offset_of!(_HD_DEVCOUNT, max_dev_count) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_METADATA"][::core::mem::size_of::<_HD_METADATA>() - 8usize];
    ["Alignment of _HD_METADATA"][::core::mem::align_of::<_HD_METADATA>() - 4usize];
    ["Offset of field: _HD_METADATA::sign"][::core::mem::offset_of!(_HD_METADATA, sign) - 0usize];
    ["Offset of field: _HD_METADATA::p_next"]
        [::core::mem::offset_of!(_HD_METADATA, p_next) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_DIM"][::core::mem::size_of::<_HD_DIM>() - 8usize];
    ["Alignment of _HD_DIM"][::core::mem::align_of::<_HD_DIM>() - 4usize];
    ["Offset of field: _HD_DIM::w"][::core::mem::offset_of!(_HD_DIM, w) - 0usize];
    ["Offset of field: _HD_DIM::h"][::core::mem::offset_of!(_HD_DIM, h) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_CROP"][::core::mem::size_of::<_HD_VIDEO_CROP>() - 24usize];
    ["Alignment of _HD_VIDEO_CROP"][::core::mem::align_of::<_HD_VIDEO_CROP>() - 4usize];
    ["Offset of field: _HD_VIDEO_CROP::coord"]
        [::core::mem::offset_of!(_HD_VIDEO_CROP, coord) - 0usize];
    ["Offset of field: _HD_VIDEO_CROP::rect"]
        [::core::mem::offset_of!(_HD_VIDEO_CROP, rect) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_PALETTE_TBL"][::core::mem::size_of::<_HD_PALETTE_TBL>() - 24usize];
    ["Alignment of _HD_PALETTE_TBL"][::core::mem::align_of::<_HD_PALETTE_TBL>() - 4usize];
    ["Offset of field: _HD_PALETTE_TBL::sign"]
        [::core::mem::offset_of!(_HD_PALETTE_TBL, sign) - 0usize];
    ["Offset of field: _HD_PALETTE_TBL::p_next"]
        [::core::mem::offset_of!(_HD_PALETTE_TBL, p_next) - 4usize];
    ["Offset of field: _HD_PALETTE_TBL::reserved"]
        [::core::mem::offset_of!(_HD_PALETTE_TBL, reserved) - 8usize];
    ["Offset of field: _HD_PALETTE_TBL::pxlfmt"]
        [::core::mem::offset_of!(_HD_PALETTE_TBL, pxlfmt) - 12usize];
    ["Offset of field: _HD_PALETTE_TBL::table_size"]
        [::core::mem::offset_of!(_HD_PALETTE_TBL, table_size) - 16usize];
    ["Offset of field: _HD_PALETTE_TBL::p_table"]
        [::core::mem::offset_of!(_HD_PALETTE_TBL, p_table) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_FRAME"][::core::mem::size_of::<_HD_VIDEO_FRAME>() - 144usize];
    ["Alignment of _HD_VIDEO_FRAME"][::core::mem::align_of::<_HD_VIDEO_FRAME>() - 8usize];
    ["Offset of field: _HD_VIDEO_FRAME::sign"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, sign) - 0usize];
    ["Offset of field: _HD_VIDEO_FRAME::p_next"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, p_next) - 4usize];
    ["Offset of field: _HD_VIDEO_FRAME::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, ddr_id) - 8usize];
    ["Offset of field: _HD_VIDEO_FRAME::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, pxlfmt) - 12usize];
    ["Offset of field: _HD_VIDEO_FRAME::dim"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, dim) - 16usize];
    ["Offset of field: _HD_VIDEO_FRAME::count"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, count) - 24usize];
    ["Offset of field: _HD_VIDEO_FRAME::timestamp"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, timestamp) - 32usize];
    ["Offset of field: _HD_VIDEO_FRAME::pw"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, pw) - 40usize];
    ["Offset of field: _HD_VIDEO_FRAME::ph"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, ph) - 56usize];
    ["Offset of field: _HD_VIDEO_FRAME::loff"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, loff) - 72usize];
    ["Offset of field: _HD_VIDEO_FRAME::phy_addr"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, phy_addr) - 88usize];
    ["Offset of field: _HD_VIDEO_FRAME::poc_info"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, poc_info) - 104usize];
    ["Offset of field: _HD_VIDEO_FRAME::reserved"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, reserved) - 108usize];
    ["Offset of field: _HD_VIDEO_FRAME::blk"]
        [::core::mem::offset_of!(_HD_VIDEO_FRAME, blk) - 136usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_NALU_TYPE"][::core::mem::size_of::<_HD_NALU_TYPE>() - 4usize];
    ["Alignment of _HD_NALU_TYPE"][::core::mem::align_of::<_HD_NALU_TYPE>() - 4usize];
    ["Offset of field: _HD_NALU_TYPE::h264_type"]
        [::core::mem::offset_of!(_HD_NALU_TYPE, h264_type) - 0usize];
    ["Offset of field: _HD_NALU_TYPE::jpeg_type"]
        [::core::mem::offset_of!(_HD_NALU_TYPE, jpeg_type) - 0usize];
    ["Offset of field: _HD_NALU_TYPE::h265_type"]
        [::core::mem::offset_of!(_HD_NALU_TYPE, h265_type) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_BS_PSNR_INFO"][::core::mem::size_of::<_HD_VIDEO_BS_PSNR_INFO>() - 12usize];
    ["Alignment of _HD_VIDEO_BS_PSNR_INFO"]
        [::core::mem::align_of::<_HD_VIDEO_BS_PSNR_INFO>() - 4usize];
    ["Offset of field: _HD_VIDEO_BS_PSNR_INFO::y_mse"]
        [::core::mem::offset_of!(_HD_VIDEO_BS_PSNR_INFO, y_mse) - 0usize];
    ["Offset of field: _HD_VIDEO_BS_PSNR_INFO::u_mse"]
        [::core::mem::offset_of!(_HD_VIDEO_BS_PSNR_INFO, u_mse) - 4usize];
    ["Offset of field: _HD_VIDEO_BS_PSNR_INFO::v_mse"]
        [::core::mem::offset_of!(_HD_VIDEO_BS_PSNR_INFO, v_mse) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_H264_BLOCK_PARTITION_INFO"]
        [::core::mem::size_of::<_HD_VIDEO_H264_BLOCK_PARTITION_INFO>() - 20usize];
    ["Alignment of _HD_VIDEO_H264_BLOCK_PARTITION_INFO"]
        [::core::mem::align_of::<_HD_VIDEO_H264_BLOCK_PARTITION_INFO>() - 4usize];
    ["Offset of field: _HD_VIDEO_H264_BLOCK_PARTITION_INFO::intra_16x16_mb_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H264_BLOCK_PARTITION_INFO, intra_16x16_mb_num) - 0usize];
    ["Offset of field: _HD_VIDEO_H264_BLOCK_PARTITION_INFO::intra_8x8_mb_Num"]
        [::core::mem::offset_of!(_HD_VIDEO_H264_BLOCK_PARTITION_INFO, intra_8x8_mb_Num) - 4usize];
    ["Offset of field: _HD_VIDEO_H264_BLOCK_PARTITION_INFO::intra_4x4_mb_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H264_BLOCK_PARTITION_INFO, intra_4x4_mb_num) - 8usize];
    ["Offset of field: _HD_VIDEO_H264_BLOCK_PARTITION_INFO::inter_mb_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H264_BLOCK_PARTITION_INFO, inter_mb_num) - 12usize];
    ["Offset of field: _HD_VIDEO_H264_BLOCK_PARTITION_INFO::skip_mb_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H264_BLOCK_PARTITION_INFO, skip_mb_num) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_H265_BLOCK_PARTITION_INFO"]
        [::core::mem::size_of::<_HD_VIDEO_H265_BLOCK_PARTITION_INFO>() - 32usize];
    ["Alignment of _HD_VIDEO_H265_BLOCK_PARTITION_INFO"]
        [::core::mem::align_of::<_HD_VIDEO_H265_BLOCK_PARTITION_INFO>() - 4usize];
    ["Offset of field: _HD_VIDEO_H265_BLOCK_PARTITION_INFO::intra_32x32_cu_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H265_BLOCK_PARTITION_INFO, intra_32x32_cu_num) - 0usize];
    ["Offset of field: _HD_VIDEO_H265_BLOCK_PARTITION_INFO::intra_16x16_cu_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H265_BLOCK_PARTITION_INFO, intra_16x16_cu_num) - 4usize];
    ["Offset of field: _HD_VIDEO_H265_BLOCK_PARTITION_INFO::intra_8x8_cu_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H265_BLOCK_PARTITION_INFO, intra_8x8_cu_num) - 8usize];
    ["Offset of field: _HD_VIDEO_H265_BLOCK_PARTITION_INFO::inter_64x64_cu_num"][::core::mem::offset_of!(
        _HD_VIDEO_H265_BLOCK_PARTITION_INFO,
        inter_64x64_cu_num
    ) - 12usize];
    ["Offset of field: _HD_VIDEO_H265_BLOCK_PARTITION_INFO::inter_32x32_cu_num"][::core::mem::offset_of!(
        _HD_VIDEO_H265_BLOCK_PARTITION_INFO,
        inter_32x32_cu_num
    ) - 16usize];
    ["Offset of field: _HD_VIDEO_H265_BLOCK_PARTITION_INFO::inter_16x16_cu_num"][::core::mem::offset_of!(
        _HD_VIDEO_H265_BLOCK_PARTITION_INFO,
        inter_16x16_cu_num
    ) - 20usize];
    ["Offset of field: _HD_VIDEO_H265_BLOCK_PARTITION_INFO::skip_cu_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H265_BLOCK_PARTITION_INFO, skip_cu_num) - 24usize];
    ["Offset of field: _HD_VIDEO_H265_BLOCK_PARTITION_INFO::merge_cu_num"]
        [::core::mem::offset_of!(_HD_VIDEO_H265_BLOCK_PARTITION_INFO, merge_cu_num) - 28usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_BS_BLOCK_PARTITION_INFO"]
        [::core::mem::size_of::<_HD_VIDEO_BS_BLOCK_PARTITION_INFO>() - 32usize];
    ["Alignment of _HD_VIDEO_BS_BLOCK_PARTITION_INFO"]
        [::core::mem::align_of::<_HD_VIDEO_BS_BLOCK_PARTITION_INFO>() - 4usize];
    ["Offset of field: _HD_VIDEO_BS_BLOCK_PARTITION_INFO::h264_info"]
        [::core::mem::offset_of!(_HD_VIDEO_BS_BLOCK_PARTITION_INFO, h264_info) - 0usize];
    ["Offset of field: _HD_VIDEO_BS_BLOCK_PARTITION_INFO::h265_info"]
        [::core::mem::offset_of!(_HD_VIDEO_BS_BLOCK_PARTITION_INFO, h265_info) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_BS_DATA"][::core::mem::size_of::<_HD_VIDEO_BS_DATA>() - 12usize];
    ["Alignment of _HD_VIDEO_BS_DATA"][::core::mem::align_of::<_HD_VIDEO_BS_DATA>() - 4usize];
    ["Offset of field: _HD_VIDEO_BS_DATA::pack_type"]
        [::core::mem::offset_of!(_HD_VIDEO_BS_DATA, pack_type) - 0usize];
    ["Offset of field: _HD_VIDEO_BS_DATA::phy_addr"]
        [::core::mem::offset_of!(_HD_VIDEO_BS_DATA, phy_addr) - 4usize];
    ["Offset of field: _HD_VIDEO_BS_DATA::size"]
        [::core::mem::offset_of!(_HD_VIDEO_BS_DATA, size) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_BS"][::core::mem::size_of::<_HD_VIDEOENC_BS>() - 144usize];
    ["Alignment of _HD_VIDEOENC_BS"][::core::mem::align_of::<_HD_VIDEOENC_BS>() - 8usize];
    ["Offset of field: _HD_VIDEOENC_BS::sign"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, sign) - 0usize];
    ["Offset of field: _HD_VIDEOENC_BS::p_next"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, p_next) - 4usize];
    ["Offset of field: _HD_VIDEOENC_BS::vcodec_format"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, vcodec_format) - 8usize];
    ["Offset of field: _HD_VIDEOENC_BS::pack_num"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, pack_num) - 12usize];
    ["Offset of field: _HD_VIDEOENC_BS::timestamp"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, timestamp) - 16usize];
    ["Offset of field: _HD_VIDEOENC_BS::frame_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, frame_type) - 24usize];
    ["Offset of field: _HD_VIDEOENC_BS::svc_layer_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, svc_layer_type) - 28usize];
    ["Offset of field: _HD_VIDEOENC_BS::video_pack"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, video_pack) - 32usize];
    ["Offset of field: _HD_VIDEOENC_BS::psnr_info"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, psnr_info) - 80usize];
    ["Offset of field: _HD_VIDEOENC_BS::blk_info"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, blk_info) - 92usize];
    ["Offset of field: _HD_VIDEOENC_BS::evbr_state"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, evbr_state) - 124usize];
    ["Offset of field: _HD_VIDEOENC_BS::qp"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, qp) - 128usize];
    ["Offset of field: _HD_VIDEOENC_BS::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, ddr_id) - 132usize];
    ["Offset of field: _HD_VIDEOENC_BS::blk"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, blk) - 136usize];
    ["Offset of field: _HD_VIDEOENC_BS::motion_ratio"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS, motion_ratio) - 140usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_BS3"][::core::mem::size_of::<_HD_VIDEOENC_BS3>() - 104usize];
    ["Alignment of _HD_VIDEOENC_BS3"][::core::mem::align_of::<_HD_VIDEOENC_BS3>() - 8usize];
    ["Offset of field: _HD_VIDEOENC_BS3::sign"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, sign) - 0usize];
    ["Offset of field: _HD_VIDEOENC_BS3::p_next"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, p_next) - 4usize];
    ["Offset of field: _HD_VIDEOENC_BS3::vcodec_format"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, vcodec_format) - 8usize];
    ["Offset of field: _HD_VIDEOENC_BS3::pack_num"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, pack_num) - 12usize];
    ["Offset of field: _HD_VIDEOENC_BS3::timestamp"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, timestamp) - 16usize];
    ["Offset of field: _HD_VIDEOENC_BS3::frame_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, frame_type) - 24usize];
    ["Offset of field: _HD_VIDEOENC_BS3::svc_layer_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, svc_layer_type) - 28usize];
    ["Offset of field: _HD_VIDEOENC_BS3::video_pack"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, video_pack) - 32usize];
    ["Offset of field: _HD_VIDEOENC_BS3::video_pack_num"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, video_pack_num) - 36usize];
    ["Offset of field: _HD_VIDEOENC_BS3::psnr_info"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, psnr_info) - 40usize];
    ["Offset of field: _HD_VIDEOENC_BS3::blk_info"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, blk_info) - 52usize];
    ["Offset of field: _HD_VIDEOENC_BS3::evbr_state"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, evbr_state) - 84usize];
    ["Offset of field: _HD_VIDEOENC_BS3::qp"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, qp) - 88usize];
    ["Offset of field: _HD_VIDEOENC_BS3::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, ddr_id) - 92usize];
    ["Offset of field: _HD_VIDEOENC_BS3::blk"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, blk) - 96usize];
    ["Offset of field: _HD_VIDEOENC_BS3::motion_ratio"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BS3, motion_ratio) - 100usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEODEC_BS"][::core::mem::size_of::<_HD_VIDEODEC_BS>() - 48usize];
    ["Alignment of _HD_VIDEODEC_BS"][::core::mem::align_of::<_HD_VIDEODEC_BS>() - 8usize];
    ["Offset of field: _HD_VIDEODEC_BS::sign"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, sign) - 0usize];
    ["Offset of field: _HD_VIDEODEC_BS::p_next"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, p_next) - 4usize];
    ["Offset of field: _HD_VIDEODEC_BS::vcodec_format"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, vcodec_format) - 8usize];
    ["Offset of field: _HD_VIDEODEC_BS::count"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, count) - 16usize];
    ["Offset of field: _HD_VIDEODEC_BS::timestamp"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, timestamp) - 24usize];
    ["Offset of field: _HD_VIDEODEC_BS::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, ddr_id) - 32usize];
    ["Offset of field: _HD_VIDEODEC_BS::phy_addr"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, phy_addr) - 36usize];
    ["Offset of field: _HD_VIDEODEC_BS::size"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, size) - 40usize];
    ["Offset of field: _HD_VIDEODEC_BS::blk"]
        [::core::mem::offset_of!(_HD_VIDEODEC_BS, blk) - 44usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIO_FRAME"][::core::mem::size_of::<_HD_AUDIO_FRAME>() - 64usize];
    ["Alignment of _HD_AUDIO_FRAME"][::core::mem::align_of::<_HD_AUDIO_FRAME>() - 8usize];
    ["Offset of field: _HD_AUDIO_FRAME::sign"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, sign) - 0usize];
    ["Offset of field: _HD_AUDIO_FRAME::p_next"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, p_next) - 4usize];
    ["Offset of field: _HD_AUDIO_FRAME::ddr_id"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, ddr_id) - 8usize];
    ["Offset of field: _HD_AUDIO_FRAME::size"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, size) - 12usize];
    ["Offset of field: _HD_AUDIO_FRAME::phy_addr"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, phy_addr) - 16usize];
    ["Offset of field: _HD_AUDIO_FRAME::bit_width"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, bit_width) - 24usize];
    ["Offset of field: _HD_AUDIO_FRAME::sound_mode"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, sound_mode) - 28usize];
    ["Offset of field: _HD_AUDIO_FRAME::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, sample_rate) - 32usize];
    ["Offset of field: _HD_AUDIO_FRAME::reserved1"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, reserved1) - 36usize];
    ["Offset of field: _HD_AUDIO_FRAME::count"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, count) - 40usize];
    ["Offset of field: _HD_AUDIO_FRAME::timestamp"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, timestamp) - 48usize];
    ["Offset of field: _HD_AUDIO_FRAME::blk"]
        [::core::mem::offset_of!(_HD_AUDIO_FRAME, blk) - 56usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIO_CODEC_INFO"][::core::mem::size_of::<_HD_AUDIO_CODEC_INFO>() - 60usize];
    ["Alignment of _HD_AUDIO_CODEC_INFO"][::core::mem::align_of::<_HD_AUDIO_CODEC_INFO>() - 4usize];
    ["Offset of field: _HD_AUDIO_CODEC_INFO::is_support"]
        [::core::mem::offset_of!(_HD_AUDIO_CODEC_INFO, is_support) - 0usize];
    ["Offset of field: _HD_AUDIO_CODEC_INFO::support_sr"]
        [::core::mem::offset_of!(_HD_AUDIO_CODEC_INFO, support_sr) - 4usize];
    ["Offset of field: _HD_AUDIO_CODEC_INFO::support_bit"]
        [::core::mem::offset_of!(_HD_AUDIO_CODEC_INFO, support_bit) - 40usize];
    ["Offset of field: _HD_AUDIO_CODEC_INFO::support_mode"]
        [::core::mem::offset_of!(_HD_AUDIO_CODEC_INFO, support_mode) - 48usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIO_BS"][::core::mem::size_of::<_HD_AUDIO_BS>() - 40usize];
    ["Alignment of _HD_AUDIO_BS"][::core::mem::align_of::<_HD_AUDIO_BS>() - 8usize];
    ["Offset of field: _HD_AUDIO_BS::sign"][::core::mem::offset_of!(_HD_AUDIO_BS, sign) - 0usize];
    ["Offset of field: _HD_AUDIO_BS::p_next"]
        [::core::mem::offset_of!(_HD_AUDIO_BS, p_next) - 4usize];
    ["Offset of field: _HD_AUDIO_BS::acodec_format"]
        [::core::mem::offset_of!(_HD_AUDIO_BS, acodec_format) - 8usize];
    ["Offset of field: _HD_AUDIO_BS::timestamp"]
        [::core::mem::offset_of!(_HD_AUDIO_BS, timestamp) - 16usize];
    ["Offset of field: _HD_AUDIO_BS::size"][::core::mem::offset_of!(_HD_AUDIO_BS, size) - 24usize];
    ["Offset of field: _HD_AUDIO_BS::ddr_id"]
        [::core::mem::offset_of!(_HD_AUDIO_BS, ddr_id) - 28usize];
    ["Offset of field: _HD_AUDIO_BS::phy_addr"]
        [::core::mem::offset_of!(_HD_AUDIO_BS, phy_addr) - 32usize];
    ["Offset of field: _HD_AUDIO_BS::blk"][::core::mem::offset_of!(_HD_AUDIO_BS, blk) - 36usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_OSG_STAMP_BUF"][::core::mem::size_of::<_HD_OSG_STAMP_BUF>() - 16usize];
    ["Alignment of _HD_OSG_STAMP_BUF"][::core::mem::align_of::<_HD_OSG_STAMP_BUF>() - 4usize];
    ["Offset of field: _HD_OSG_STAMP_BUF::type_"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_BUF, type_) - 0usize];
    ["Offset of field: _HD_OSG_STAMP_BUF::size"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_BUF, size) - 4usize];
    ["Offset of field: _HD_OSG_STAMP_BUF::p_addr"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_BUF, p_addr) - 8usize];
    ["Offset of field: _HD_OSG_STAMP_BUF::ddr_id"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_BUF, ddr_id) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_OSG_STAMP_IMG"][::core::mem::size_of::<_HD_OSG_STAMP_IMG>() - 24usize];
    ["Alignment of _HD_OSG_STAMP_IMG"][::core::mem::align_of::<_HD_OSG_STAMP_IMG>() - 4usize];
    ["Offset of field: _HD_OSG_STAMP_IMG::fmt"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_IMG, fmt) - 0usize];
    ["Offset of field: _HD_OSG_STAMP_IMG::dim"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_IMG, dim) - 4usize];
    ["Offset of field: _HD_OSG_STAMP_IMG::p_addr"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_IMG, p_addr) - 12usize];
    ["Offset of field: _HD_OSG_STAMP_IMG::ddr_id"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_IMG, ddr_id) - 16usize];
    ["Offset of field: _HD_OSG_STAMP_IMG::copy"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_IMG, copy) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_OSG_STAMP_ATTR"][::core::mem::size_of::<_HD_OSG_STAMP_ATTR>() - 48usize];
    ["Alignment of _HD_OSG_STAMP_ATTR"][::core::mem::align_of::<_HD_OSG_STAMP_ATTR>() - 4usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::align_type"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, align_type) - 0usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::alpha"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, alpha) - 4usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::position"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, position) - 8usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::colorkey_en"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, colorkey_en) - 16usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::colorkey_val"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, colorkey_val) - 20usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::qp_en"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, qp_en) - 24usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::qp_fix"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, qp_fix) - 28usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::qp_val"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, qp_val) - 32usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::layer"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, layer) - 36usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::region"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, region) - 40usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::gcac_enable"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, gcac_enable) - 44usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::gcac_blk_width"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, gcac_blk_width) - 45usize];
    ["Offset of field: _HD_OSG_STAMP_ATTR::gcac_blk_height"]
        [::core::mem::offset_of!(_HD_OSG_STAMP_ATTR, gcac_blk_height) - 46usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_OSG_MASK_ATTR"][::core::mem::size_of::<_HD_OSG_MASK_ATTR>() - 48usize];
    ["Alignment of _HD_OSG_MASK_ATTR"][::core::mem::align_of::<_HD_OSG_MASK_ATTR>() - 4usize];
    ["Offset of field: _HD_OSG_MASK_ATTR::type_"]
        [::core::mem::offset_of!(_HD_OSG_MASK_ATTR, type_) - 0usize];
    ["Offset of field: _HD_OSG_MASK_ATTR::color"]
        [::core::mem::offset_of!(_HD_OSG_MASK_ATTR, color) - 4usize];
    ["Offset of field: _HD_OSG_MASK_ATTR::alpha"]
        [::core::mem::offset_of!(_HD_OSG_MASK_ATTR, alpha) - 8usize];
    ["Offset of field: _HD_OSG_MASK_ATTR::position"]
        [::core::mem::offset_of!(_HD_OSG_MASK_ATTR, position) - 12usize];
    ["Offset of field: _HD_OSG_MASK_ATTR::thickness"]
        [::core::mem::offset_of!(_HD_OSG_MASK_ATTR, thickness) - 44usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_OSG_MOSAIC_ATTR"][::core::mem::size_of::<_HD_OSG_MOSAIC_ATTR>() - 48usize];
    ["Alignment of _HD_OSG_MOSAIC_ATTR"][::core::mem::align_of::<_HD_OSG_MOSAIC_ATTR>() - 4usize];
    ["Offset of field: _HD_OSG_MOSAIC_ATTR::type_"]
        [::core::mem::offset_of!(_HD_OSG_MOSAIC_ATTR, type_) - 0usize];
    ["Offset of field: _HD_OSG_MOSAIC_ATTR::alpha"]
        [::core::mem::offset_of!(_HD_OSG_MOSAIC_ATTR, alpha) - 4usize];
    ["Offset of field: _HD_OSG_MOSAIC_ATTR::mosaic_blk_w"]
        [::core::mem::offset_of!(_HD_OSG_MOSAIC_ATTR, mosaic_blk_w) - 8usize];
    ["Offset of field: _HD_OSG_MOSAIC_ATTR::mosaic_blk_h"]
        [::core::mem::offset_of!(_HD_OSG_MOSAIC_ATTR, mosaic_blk_h) - 12usize];
    ["Offset of field: _HD_OSG_MOSAIC_ATTR::position"]
        [::core::mem::offset_of!(_HD_OSG_MOSAIC_ATTR, position) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_COMMON_MEM_POOL_INFO"]
        [::core::mem::size_of::<_HD_COMMON_MEM_POOL_INFO>() - 24usize];
    ["Alignment of _HD_COMMON_MEM_POOL_INFO"]
        [::core::mem::align_of::<_HD_COMMON_MEM_POOL_INFO>() - 4usize];
    ["Offset of field: _HD_COMMON_MEM_POOL_INFO::ddr_id"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_POOL_INFO, ddr_id) - 0usize];
    ["Offset of field: _HD_COMMON_MEM_POOL_INFO::type_"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_POOL_INFO, type_) - 4usize];
    ["Offset of field: _HD_COMMON_MEM_POOL_INFO::blk_size"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_POOL_INFO, blk_size) - 8usize];
    ["Offset of field: _HD_COMMON_MEM_POOL_INFO::blk_cnt"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_POOL_INFO, blk_cnt) - 12usize];
    ["Offset of field: _HD_COMMON_MEM_POOL_INFO::start_addr"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_POOL_INFO, start_addr) - 16usize];
    ["Offset of field: _HD_COMMON_MEM_POOL_INFO::shared_pool"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_POOL_INFO, shared_pool) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_COMMON_MEM_CONFIG"][::core::mem::size_of::<_HD_COMMON_MEM_CONFIG>() - 3072usize];
    ["Alignment of _HD_COMMON_MEM_CONFIG"]
        [::core::mem::align_of::<_HD_COMMON_MEM_CONFIG>() - 4usize];
    ["Offset of field: _HD_COMMON_MEM_CONFIG::pool_info"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_CONFIG, pool_info) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_COMMON_MEM_VIRT_INFO"]
        [::core::mem::size_of::<_HD_COMMON_MEM_VIRT_INFO>() - 12usize];
    ["Alignment of _HD_COMMON_MEM_VIRT_INFO"]
        [::core::mem::align_of::<_HD_COMMON_MEM_VIRT_INFO>() - 4usize];
    ["Offset of field: _HD_COMMON_MEM_VIRT_INFO::va"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_VIRT_INFO, va) - 0usize];
    ["Offset of field: _HD_COMMON_MEM_VIRT_INFO::pa"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_VIRT_INFO, pa) - 4usize];
    ["Offset of field: _HD_COMMON_MEM_VIRT_INFO::cached"]
        [::core::mem::offset_of!(_HD_COMMON_MEM_VIRT_INFO, cached) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_VI__bindgen_ty_1"]
        [::core::mem::size_of::<_HD_VIDEOCAP_VI__bindgen_ty_1>() - 36usize];
    ["Alignment of _HD_VIDEOCAP_VI__bindgen_ty_1"]
        [::core::mem::align_of::<_HD_VIDEOCAP_VI__bindgen_ty_1>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_1::src"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_1, src) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_1::format"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_1, format) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_1::tdm"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_1, tdm) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_1::id_extract"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_1, id_extract) - 12usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_1::latch_edge"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_1, latch_edge) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_1::reserved"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_1, reserved) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_VI__bindgen_ty_2"]
        [::core::mem::size_of::<_HD_VIDEOCAP_VI__bindgen_ty_2>() - 36usize];
    ["Alignment of _HD_VIDEOCAP_VI__bindgen_ty_2"]
        [::core::mem::align_of::<_HD_VIDEOCAP_VI__bindgen_ty_2>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_2::data_swap"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_2, data_swap) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_2::clk_pin"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_2, clk_pin) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_2::clk_inv"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_2, clk_inv) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_2::clk_dly"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_2, clk_dly) - 12usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_2::clk_pdly"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_2, clk_pdly) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_VI__bindgen_ty_2::reserved"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI__bindgen_ty_2, reserved) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_VI"][::core::mem::size_of::<_HD_VIDEOCAP_VI>() - 112usize];
    ["Alignment of _HD_VIDEOCAP_VI"][::core::mem::align_of::<_HD_VIDEOCAP_VI>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI::chip"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI, chip) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_VI::vcap"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI, vcap) - 1usize];
    ["Offset of field: _HD_VIDEOCAP_VI::vi"][::core::mem::offset_of!(_HD_VIDEOCAP_VI, vi) - 2usize];
    ["Offset of field: _HD_VIDEOCAP_VI::global"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI, global) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI::vport"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI, vport) - 40usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_VI_CH_NORM"]
        [::core::mem::size_of::<_HD_VIDEOCAP_VI_CH_NORM>() - 76usize];
    ["Alignment of _HD_VIDEOCAP_VI_CH_NORM"]
        [::core::mem::align_of::<_HD_VIDEOCAP_VI_CH_NORM>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::chip"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, chip) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::vcap"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, vcap) - 1usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::vi"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, vi) - 2usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::ch"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, ch) - 3usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::cap_width"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, cap_width) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::cap_height"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, cap_height) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::org_width"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, org_width) - 12usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::org_height"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, org_height) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::fps"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, fps) - 20usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::format"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, format) - 24usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::prog"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, prog) - 28usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::data_rate"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, data_rate) - 32usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::data_latch"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, data_latch) - 36usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::horiz_dup"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, horiz_dup) - 40usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM::reserved"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM, reserved) - 44usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_VI_CH_NORM3"]
        [::core::mem::size_of::<_HD_VIDEOCAP_VI_CH_NORM3>() - 80usize];
    ["Alignment of _HD_VIDEOCAP_VI_CH_NORM3"]
        [::core::mem::align_of::<_HD_VIDEOCAP_VI_CH_NORM3>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::chip"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, chip) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::vcap"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, vcap) - 1usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::vi"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, vi) - 2usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::ch"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, ch) - 3usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::cap_width"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, cap_width) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::cap_height"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, cap_height) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::org_width"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, org_width) - 12usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::org_height"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, org_height) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::fps"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, fps) - 20usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::format"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, format) - 24usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::prog"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, prog) - 28usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::data_rate"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, data_rate) - 32usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::data_latch"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, data_latch) - 36usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::horiz_dup"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, horiz_dup) - 40usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::data_src"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, data_src) - 44usize];
    ["Offset of field: _HD_VIDEOCAP_VI_CH_NORM3::reserved"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_VI_CH_NORM3, reserved) - 48usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_PINMUX_CFG"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_PINMUX_CFG>() - 12usize];
    ["Alignment of _HD_VIDEOCAP_SEN_PINMUX_CFG"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_PINMUX_CFG>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_PINMUX_CFG::sensor_pinmux"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_PINMUX_CFG, sensor_pinmux) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_PINMUX_CFG::serial_if_pinmux"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_PINMUX_CFG, serial_if_pinmux) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_PINMUX_CFG::cmd_if_pinmux"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_PINMUX_CFG, cmd_if_pinmux) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_INIT_PIN_CFG"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_INIT_PIN_CFG>() - 64usize];
    ["Alignment of _HD_VIDEOCAP_SEN_INIT_PIN_CFG"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_INIT_PIN_CFG>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_PIN_CFG::pinmux"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_PIN_CFG, pinmux) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_PIN_CFG::clk_lane_sel"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_PIN_CFG, clk_lane_sel) - 12usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_PIN_CFG::sen_2_serial_pin_map"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_PIN_CFG, sen_2_serial_pin_map) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_PIN_CFG::ccir_msblsb_switch"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_PIN_CFG, ccir_msblsb_switch) - 48usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_PIN_CFG::ccir_vd_hd_pin"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_PIN_CFG, ccir_vd_hd_pin) - 52usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_PIN_CFG::vx1_tx241_cko_pin"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_PIN_CFG, vx1_tx241_cko_pin) - 56usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_PIN_CFG::vx1_tx241_cfg_2lane_mode"][::core::mem::offset_of!(
        _HD_VIDEOCAP_SEN_INIT_PIN_CFG,
        vx1_tx241_cfg_2lane_mode
    ) - 60usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_IF_VX1"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_IF_VX1>() - 16usize];
    ["Alignment of _HD_VIDEOCAP_SEN_IF_VX1"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_IF_VX1>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_IF_VX1::en"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_IF_VX1, en) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_IF_VX1::if_sel"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_IF_VX1, if_sel) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_IF_VX1::ctl_sel"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_IF_VX1, ctl_sel) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_IF_VX1::tx_type"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_IF_VX1, tx_type) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_IF_TGE"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_IF_TGE>() - 16usize];
    ["Alignment of _HD_VIDEOCAP_SEN_IF_TGE"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_IF_TGE>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_IF_TGE::tge_en"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_IF_TGE, tge_en) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_IF_TGE::swap"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_IF_TGE, swap) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_IF_TGE::vcap_vd_src"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_IF_TGE, vcap_vd_src) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_IF_TGE::vcap_sync_set"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_IF_TGE, vcap_sync_set) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_INIT_IF_CFG"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_INIT_IF_CFG>() - 32usize];
    ["Alignment of _HD_VIDEOCAP_SEN_INIT_IF_CFG"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_INIT_IF_CFG>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_IF_CFG::vx1"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_IF_CFG, vx1) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_IF_CFG::tge"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_IF_CFG, tge) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_INIT_OPTION"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_INIT_OPTION>() - 12usize];
    ["Alignment of _HD_VIDEOCAP_SEN_INIT_OPTION"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_INIT_OPTION>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_OPTION::en_mask"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_OPTION, en_mask) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_OPTION::sen_map_if"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_OPTION, sen_map_if) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_INIT_OPTION::if_time_out"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_INIT_OPTION, if_time_out) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SENSOR_DEVICE"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SENSOR_DEVICE>() - 144usize];
    ["Alignment of _HD_VIDEOCAP_SENSOR_DEVICE"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SENSOR_DEVICE>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SENSOR_DEVICE::driver_name"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SENSOR_DEVICE, driver_name) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SENSOR_DEVICE::if_type"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SENSOR_DEVICE, if_type) - 32usize];
    ["Offset of field: _HD_VIDEOCAP_SENSOR_DEVICE::if_cfg"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SENSOR_DEVICE, if_cfg) - 36usize];
    ["Offset of field: _HD_VIDEOCAP_SENSOR_DEVICE::pin_cfg"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SENSOR_DEVICE, pin_cfg) - 68usize];
    ["Offset of field: _HD_VIDEOCAP_SENSOR_DEVICE::option"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SENSOR_DEVICE, option) - 132usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_CONFIG"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_CONFIG>() - 148usize];
    ["Alignment of _HD_VIDEOCAP_SEN_CONFIG"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_CONFIG::sen_dev"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_CONFIG, sen_dev) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_CONFIG::shdr_map"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_CONFIG, shdr_map) - 144usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_CONFIG"][::core::mem::size_of::<_HD_VIDEOCAP_CONFIG>() - 148usize];
    ["Alignment of _HD_VIDEOCAP_CONFIG"][::core::mem::align_of::<_HD_VIDEOCAP_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_CONFIG::sen_cfg"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CONFIG, sen_cfg) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_CFG_EXT"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_CFG_EXT>() - 8usize];
    ["Alignment of _HD_VIDEOCAP_SEN_CFG_EXT"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_CFG_EXT>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_CFG_EXT::func"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_CFG_EXT, func) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_CFG_EXT::value"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_CFG_EXT, value) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SEN_CONFIG_EXT"]
        [::core::mem::size_of::<_HD_VIDEOCAP_SEN_CONFIG_EXT>() - 8usize];
    ["Alignment of _HD_VIDEOCAP_SEN_CONFIG_EXT"]
        [::core::mem::align_of::<_HD_VIDEOCAP_SEN_CONFIG_EXT>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_CONFIG_EXT::cfg_num"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_CONFIG_EXT, cfg_num) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SEN_CONFIG_EXT::p_sen_cfg_ext"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SEN_CONFIG_EXT, p_sen_cfg_ext) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_CONFIG3"][::core::mem::size_of::<_HD_VIDEOCAP_CONFIG3>() - 40usize];
    ["Alignment of _HD_VIDEOCAP_CONFIG3"][::core::mem::align_of::<_HD_VIDEOCAP_CONFIG3>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_CONFIG3::driver_name"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CONFIG3, driver_name) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_CONFIG3::sen_cfg_ext"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CONFIG3, sen_cfg_ext) - 32usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_CTRL"][::core::mem::size_of::<_HD_VIDEOCAP_CTRL>() - 4usize];
    ["Alignment of _HD_VIDEOCAP_CTRL"][::core::mem::align_of::<_HD_VIDEOCAP_CTRL>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_CTRL::func"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CTRL, func) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_IN"][::core::mem::size_of::<_HD_VIDEOCAP_IN>() - 24usize];
    ["Alignment of _HD_VIDEOCAP_IN"][::core::mem::align_of::<_HD_VIDEOCAP_IN>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_IN::sen_mode"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_IN, sen_mode) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_IN::frc"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_IN, frc) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_IN::dim"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_IN, dim) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_IN::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_IN, pxlfmt) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_IN::out_frame_num"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_IN, out_frame_num) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_OUT"][::core::mem::size_of::<_HD_VIDEOCAP_OUT>() - 24usize];
    ["Alignment of _HD_VIDEOCAP_OUT"][::core::mem::align_of::<_HD_VIDEOCAP_OUT>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_OUT::dim"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_OUT, dim) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_OUT::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_OUT, pxlfmt) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_OUT::dir"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_OUT, dir) - 12usize];
    ["Offset of field: _HD_VIDEOCAP_OUT::frc"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_OUT, frc) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_OUT::depth"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_OUT, depth) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_CROP_AUTO_INFO"]
        [::core::mem::size_of::<_HD_VIDEOCAP_CROP_AUTO_INFO>() - 16usize];
    ["Alignment of _HD_VIDEOCAP_CROP_AUTO_INFO"]
        [::core::mem::align_of::<_HD_VIDEOCAP_CROP_AUTO_INFO>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_CROP_AUTO_INFO::ratio_w_h"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CROP_AUTO_INFO, ratio_w_h) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_CROP_AUTO_INFO::factor"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CROP_AUTO_INFO, factor) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_CROP_AUTO_INFO::sft"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CROP_AUTO_INFO, sft) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_CROP"][::core::mem::size_of::<_HD_VIDEOCAP_CROP>() - 52usize];
    ["Alignment of _HD_VIDEOCAP_CROP"][::core::mem::align_of::<_HD_VIDEOCAP_CROP>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_CROP::mode"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CROP, mode) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_CROP::win"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CROP, win) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_CROP::auto_info"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CROP, auto_info) - 28usize];
    ["Offset of field: _HD_VIDEOCAP_CROP::align"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_CROP, align) - 44usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SYSCAPS"][::core::mem::size_of::<_HD_VIDEOCAP_SYSCAPS>() - 76usize];
    ["Alignment of _HD_VIDEOCAP_SYSCAPS"][::core::mem::align_of::<_HD_VIDEOCAP_SYSCAPS>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::dev_id"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, dev_id) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::chip_id"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, chip_id) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::max_in_count"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, max_in_count) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::max_out_count"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, max_out_count) - 12usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::dev_caps"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, dev_caps) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::in_caps"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, in_caps) - 20usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::out_caps"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, out_caps) - 24usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::max_dim"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, max_dim) - 56usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::max_frame_rate"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, max_frame_rate) - 64usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::max_w_scaleup"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, max_w_scaleup) - 68usize];
    ["Offset of field: _HD_VIDEOCAP_SYSCAPS::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSCAPS, pxlfmt) - 72usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_SYSINFO"][::core::mem::size_of::<_HD_VIDEOCAP_SYSINFO>() - 64usize];
    ["Alignment of _HD_VIDEOCAP_SYSINFO"][::core::mem::align_of::<_HD_VIDEOCAP_SYSINFO>() - 8usize];
    ["Offset of field: _HD_VIDEOCAP_SYSINFO::dev_id"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSINFO, dev_id) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_SYSINFO::cur_fps"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSINFO, cur_fps) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_SYSINFO::vd_count"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSINFO, vd_count) - 40usize];
    ["Offset of field: _HD_VIDEOCAP_SYSINFO::output_started"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSINFO, output_started) - 48usize];
    ["Offset of field: _HD_VIDEOCAP_SYSINFO::cur_dim"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_SYSINFO, cur_dim) - 52usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_MD_STATUS"][::core::mem::size_of::<_HD_VIDEOCAP_MD_STATUS>() - 4usize];
    ["Alignment of _HD_VIDEOCAP_MD_STATUS"]
        [::core::mem::align_of::<_HD_VIDEOCAP_MD_STATUS>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_MD_STATUS::enabled"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_MD_STATUS, enabled) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_POOL"][::core::mem::size_of::<_HD_VIDEOCAP_POOL>() - 32usize];
    ["Alignment of _HD_VIDEOCAP_POOL"][::core::mem::align_of::<_HD_VIDEOCAP_POOL>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_POOL::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_POOL, ddr_id) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_POOL::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_POOL, pxlfmt) - 4usize];
    ["Offset of field: _HD_VIDEOCAP_POOL::dim"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_POOL, dim) - 8usize];
    ["Offset of field: _HD_VIDEOCAP_POOL::counts"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_POOL, counts) - 16usize];
    ["Offset of field: _HD_VIDEOCAP_POOL::max_counts"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_POOL, max_counts) - 20usize];
    ["Offset of field: _HD_VIDEOCAP_POOL::min_counts"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_POOL, min_counts) - 24usize];
    ["Offset of field: _HD_VIDEOCAP_POOL::mode"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_POOL, mode) - 28usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_PATH_CONFIG"]
        [::core::mem::size_of::<_HD_VIDEOCAP_PATH_CONFIG>() - 128usize];
    ["Alignment of _HD_VIDEOCAP_PATH_CONFIG"]
        [::core::mem::align_of::<_HD_VIDEOCAP_PATH_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_PATH_CONFIG::data_pool"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_PATH_CONFIG, data_pool) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOCAP_FUNC_CONFIG"]
        [::core::mem::size_of::<_HD_VIDEOCAP_FUNC_CONFIG>() - 8usize];
    ["Alignment of _HD_VIDEOCAP_FUNC_CONFIG"]
        [::core::mem::align_of::<_HD_VIDEOCAP_FUNC_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOCAP_FUNC_CONFIG::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_FUNC_CONFIG, ddr_id) - 0usize];
    ["Offset of field: _HD_VIDEOCAP_FUNC_CONFIG::out_func"]
        [::core::mem::offset_of!(_HD_VIDEOCAP_FUNC_CONFIG, out_func) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_SYSCAPS"][::core::mem::size_of::<_HD_VIDEOPROC_SYSCAPS>() - 120usize];
    ["Alignment of _HD_VIDEOPROC_SYSCAPS"]
        [::core::mem::align_of::<_HD_VIDEOPROC_SYSCAPS>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::dev_id"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, dev_id) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::chip_id"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, chip_id) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_in_count"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_in_count) - 8usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_out_count"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_out_count) - 12usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::dev_caps"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, dev_caps) - 16usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::in_caps"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, in_caps) - 20usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::out_caps"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, out_caps) - 24usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_w_scaleup_ratio"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_w_scaleup_ratio) - 88usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_w_scaledown_ratio"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_w_scaledown_ratio) - 92usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_h_scaleup_ratio"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_h_scaleup_ratio) - 96usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_h_scaledown_ratio"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_h_scaledown_ratio) - 100usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_in_stamp"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_in_stamp) - 104usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_in_stamp_ex"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_in_stamp_ex) - 108usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_in_mask"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_in_mask) - 112usize];
    ["Offset of field: _HD_VIDEOPROC_SYSCAPS::max_in_mask_ex"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSCAPS, max_in_mask_ex) - 116usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_SYSINFO"][::core::mem::size_of::<_HD_VIDEOPROC_SYSINFO>() - 72usize];
    ["Alignment of _HD_VIDEOPROC_SYSINFO"]
        [::core::mem::align_of::<_HD_VIDEOPROC_SYSINFO>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_SYSINFO::dev_id"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSINFO, dev_id) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_SYSINFO::cur_in_fps"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSINFO, cur_in_fps) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_SYSINFO::cur_out_fps"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SYSINFO, cur_out_fps) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_CROP"][::core::mem::size_of::<_HD_VIDEOPROC_CROP>() - 28usize];
    ["Alignment of _HD_VIDEOPROC_CROP"][::core::mem::align_of::<_HD_VIDEOPROC_CROP>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_CROP::mode"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_CROP, mode) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_CROP::win"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_CROP, win) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_FRC"][::core::mem::size_of::<_HD_VIDEOPROC_FRC>() - 4usize];
    ["Alignment of _HD_VIDEOPROC_FRC"][::core::mem::align_of::<_HD_VIDEOPROC_FRC>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_FRC::frc"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_FRC, frc) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_IN"][::core::mem::size_of::<_HD_VIDEOPROC_IN>() - 24usize];
    ["Alignment of _HD_VIDEOPROC_IN"][::core::mem::align_of::<_HD_VIDEOPROC_IN>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_IN::func"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN, func) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_IN::dim"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN, dim) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_IN::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN, pxlfmt) - 12usize];
    ["Offset of field: _HD_VIDEOPROC_IN::dir"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN, dir) - 16usize];
    ["Offset of field: _HD_VIDEOPROC_IN::frc"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN, frc) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_IN3"][::core::mem::size_of::<_HD_VIDEOPROC_IN3>() - 28usize];
    ["Alignment of _HD_VIDEOPROC_IN3"][::core::mem::align_of::<_HD_VIDEOPROC_IN3>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_IN3::func"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN3, func) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_IN3::dim"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN3, dim) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_IN3::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN3, pxlfmt) - 12usize];
    ["Offset of field: _HD_VIDEOPROC_IN3::dir"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN3, dir) - 16usize];
    ["Offset of field: _HD_VIDEOPROC_IN3::frc"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN3, frc) - 20usize];
    ["Offset of field: _HD_VIDEOPROC_IN3::depth"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_IN3, depth) - 24usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_OUT"][::core::mem::size_of::<_HD_VIDEOPROC_OUT>() - 52usize];
    ["Alignment of _HD_VIDEOPROC_OUT"][::core::mem::align_of::<_HD_VIDEOPROC_OUT>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_OUT::func"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT, func) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_OUT::dim"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT, dim) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_OUT::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT, pxlfmt) - 12usize];
    ["Offset of field: _HD_VIDEOPROC_OUT::dir"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT, dir) - 16usize];
    ["Offset of field: _HD_VIDEOPROC_OUT::frc"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT, frc) - 20usize];
    ["Offset of field: _HD_VIDEOPROC_OUT::depth"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT, depth) - 24usize];
    ["Offset of field: _HD_VIDEOPROC_OUT::rect"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT, rect) - 28usize];
    ["Offset of field: _HD_VIDEOPROC_OUT::bg"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT, bg) - 44usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_OUT_EX"][::core::mem::size_of::<_HD_VIDEOPROC_OUT_EX>() - 28usize];
    ["Alignment of _HD_VIDEOPROC_OUT_EX"][::core::mem::align_of::<_HD_VIDEOPROC_OUT_EX>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_OUT_EX::src_path"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT_EX, src_path) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_OUT_EX::dim"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT_EX, dim) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_OUT_EX::pxlfmt"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT_EX, pxlfmt) - 12usize];
    ["Offset of field: _HD_VIDEOPROC_OUT_EX::dir"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT_EX, dir) - 16usize];
    ["Offset of field: _HD_VIDEOPROC_OUT_EX::frc"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT_EX, frc) - 20usize];
    ["Offset of field: _HD_VIDEOPROC_OUT_EX::depth"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_OUT_EX, depth) - 24usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_PATTERN_IMG"]
        [::core::mem::size_of::<_HD_VIDEOPROC_PATTERN_IMG>() - 152usize];
    ["Alignment of _HD_VIDEOPROC_PATTERN_IMG"]
        [::core::mem::align_of::<_HD_VIDEOPROC_PATTERN_IMG>() - 8usize];
    ["Offset of field: _HD_VIDEOPROC_PATTERN_IMG::index"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_PATTERN_IMG, index) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_PATTERN_IMG::image"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_PATTERN_IMG, image) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_PATTERN_SELECT"]
        [::core::mem::size_of::<_HD_VIDEOPROC_PATTERN_SELECT>() - 44usize];
    ["Alignment of _HD_VIDEOPROC_PATTERN_SELECT"]
        [::core::mem::align_of::<_HD_VIDEOPROC_PATTERN_SELECT>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_PATTERN_SELECT::index"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_PATTERN_SELECT, index) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_PATTERN_SELECT::rect"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_PATTERN_SELECT, rect) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_PATTERN_SELECT::bg_color_sel"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_PATTERN_SELECT, bg_color_sel) - 20usize];
    ["Offset of field: _HD_VIDEOPROC_PATTERN_SELECT::img_crop_enable"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_PATTERN_SELECT, img_crop_enable) - 24usize];
    ["Offset of field: _HD_VIDEOPROC_PATTERN_SELECT::img_crop"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_PATTERN_SELECT, img_crop) - 28usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_CTRL"][::core::mem::size_of::<_HD_VIDEOPROC_CTRL>() - 8usize];
    ["Alignment of _HD_VIDEOPROC_CTRL"][::core::mem::align_of::<_HD_VIDEOPROC_CTRL>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_CTRL::func"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_CTRL, func) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_CTRL::ref_path_3dnr"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_CTRL, ref_path_3dnr) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_LL_CONFIG"][::core::mem::size_of::<_HD_VIDEOPROC_LL_CONFIG>() - 4usize];
    ["Alignment of _HD_VIDEOPROC_LL_CONFIG"]
        [::core::mem::align_of::<_HD_VIDEOPROC_LL_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_LL_CONFIG::delay_trig_lowlatency"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_LL_CONFIG, delay_trig_lowlatency) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_POOL"][::core::mem::size_of::<_HD_VIDEOPROC_POOL>() - 20usize];
    ["Alignment of _HD_VIDEOPROC_POOL"][::core::mem::align_of::<_HD_VIDEOPROC_POOL>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_POOL::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_POOL, ddr_id) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_POOL::counts"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_POOL, counts) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_POOL::max_counts"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_POOL, max_counts) - 8usize];
    ["Offset of field: _HD_VIDEOPROC_POOL::min_counts"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_POOL, min_counts) - 12usize];
    ["Offset of field: _HD_VIDEOPROC_POOL::mode"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_POOL, mode) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_DEV_CONFIG"]
        [::core::mem::size_of::<_HD_VIDEOPROC_DEV_CONFIG>() - 120usize];
    ["Alignment of _HD_VIDEOPROC_DEV_CONFIG"]
        [::core::mem::align_of::<_HD_VIDEOPROC_DEV_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_DEV_CONFIG::pipe"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_DEV_CONFIG, pipe) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_DEV_CONFIG::isp_id"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_DEV_CONFIG, isp_id) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_DEV_CONFIG::ctrl_max"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_DEV_CONFIG, ctrl_max) - 8usize];
    ["Offset of field: _HD_VIDEOPROC_DEV_CONFIG::in_max"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_DEV_CONFIG, in_max) - 16usize];
    ["Offset of field: _HD_VIDEOPROC_DEV_CONFIG::data_pool"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_DEV_CONFIG, data_pool) - 40usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_FUNC_CONFIG"]
        [::core::mem::size_of::<_HD_VIDEOPROC_FUNC_CONFIG>() - 16usize];
    ["Alignment of _HD_VIDEOPROC_FUNC_CONFIG"]
        [::core::mem::align_of::<_HD_VIDEOPROC_FUNC_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOPROC_FUNC_CONFIG::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_FUNC_CONFIG, ddr_id) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_FUNC_CONFIG::in_func"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_FUNC_CONFIG, in_func) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_FUNC_CONFIG::out_func"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_FUNC_CONFIG, out_func) - 8usize];
    ["Offset of field: _HD_VIDEOPROC_FUNC_CONFIG::out_order"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_FUNC_CONFIG, out_order) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_USER_BS"][::core::mem::size_of::<_HD_VIDEOPROC_USER_BS>() - 48usize];
    ["Alignment of _HD_VIDEOPROC_USER_BS"]
        [::core::mem::align_of::<_HD_VIDEOPROC_USER_BS>() - 8usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::sign"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, sign) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::p_next"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, p_next) - 4usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::p_bs_buf"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, p_bs_buf) - 8usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::bs_buf_size"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, bs_buf_size) - 12usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::retval"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, retval) - 16usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::time_align"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, time_align) - 20usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::time_diff"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, time_diff) - 24usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::timestamp"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, timestamp) - 32usize];
    ["Offset of field: _HD_VIDEOPROC_USER_BS::user_flag"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_USER_BS, user_flag) - 40usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOPROC_SEND_LIST"]
        [::core::mem::size_of::<_HD_VIDEOPROC_SEND_LIST>() - 64usize];
    ["Alignment of _HD_VIDEOPROC_SEND_LIST"]
        [::core::mem::align_of::<_HD_VIDEOPROC_SEND_LIST>() - 8usize];
    ["Offset of field: _HD_VIDEOPROC_SEND_LIST::path_id"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SEND_LIST, path_id) - 0usize];
    ["Offset of field: _HD_VIDEOPROC_SEND_LIST::user_bs"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SEND_LIST, user_bs) - 8usize];
    ["Offset of field: _HD_VIDEOPROC_SEND_LIST::retval"]
        [::core::mem::offset_of!(_HD_VIDEOPROC_SEND_LIST, retval) - 56usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_SYSCAPS"][::core::mem::size_of::<_HD_VIDEOENC_SYSCAPS>() - 804usize];
    ["Alignment of _HD_VIDEOENC_SYSCAPS"][::core::mem::align_of::<_HD_VIDEOENC_SYSCAPS>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::dev_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, dev_id) - 0usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::chip_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, chip_id) - 4usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::max_in_count"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, max_in_count) - 8usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::max_out_count"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, max_out_count) - 12usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::dev_caps"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, dev_caps) - 16usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::in_caps"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, in_caps) - 20usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::out_caps"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, out_caps) - 404usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::max_in_stamp"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, max_in_stamp) - 788usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::max_in_stamp_ex"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, max_in_stamp_ex) - 792usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::max_in_mask"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, max_in_mask) - 796usize];
    ["Offset of field: _HD_VIDEOENC_SYSCAPS::max_in_mask_ex"]
        [::core::mem::offset_of!(_HD_VIDEOENC_SYSCAPS, max_in_mask_ex) - 800usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_BUFINFO"][::core::mem::size_of::<_HD_VIDEOENC_BUFINFO>() - 8usize];
    ["Alignment of _HD_VIDEOENC_BUFINFO"][::core::mem::align_of::<_HD_VIDEOENC_BUFINFO>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_BUFINFO::buf_info"]
        [::core::mem::offset_of!(_HD_VIDEOENC_BUFINFO, buf_info) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_MAXMEM"][::core::mem::size_of::<_HD_VIDEOENC_MAXMEM>() - 36usize];
    ["Alignment of _HD_VIDEOENC_MAXMEM"][::core::mem::align_of::<_HD_VIDEOENC_MAXMEM>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_MAXMEM::codec_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_MAXMEM, codec_type) - 0usize];
    ["Offset of field: _HD_VIDEOENC_MAXMEM::max_dim"]
        [::core::mem::offset_of!(_HD_VIDEOENC_MAXMEM, max_dim) - 4usize];
    ["Offset of field: _HD_VIDEOENC_MAXMEM::bitrate"]
        [::core::mem::offset_of!(_HD_VIDEOENC_MAXMEM, bitrate) - 12usize];
    ["Offset of field: _HD_VIDEOENC_MAXMEM::enc_buf_ms"]
        [::core::mem::offset_of!(_HD_VIDEOENC_MAXMEM, enc_buf_ms) - 16usize];
    ["Offset of field: _HD_VIDEOENC_MAXMEM::svc_layer"]
        [::core::mem::offset_of!(_HD_VIDEOENC_MAXMEM, svc_layer) - 20usize];
    ["Offset of field: _HD_VIDEOENC_MAXMEM::ltr"]
        [::core::mem::offset_of!(_HD_VIDEOENC_MAXMEM, ltr) - 24usize];
    ["Offset of field: _HD_VIDEOENC_MAXMEM::rotate"]
        [::core::mem::offset_of!(_HD_VIDEOENC_MAXMEM, rotate) - 28usize];
    ["Offset of field: _HD_VIDEOENC_MAXMEM::source_output"]
        [::core::mem::offset_of!(_HD_VIDEOENC_MAXMEM, source_output) - 32usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_POOL"][::core::mem::size_of::<_HD_VIDEOENC_POOL>() - 20usize];
    ["Alignment of _HD_VIDEOENC_POOL"][::core::mem::align_of::<_HD_VIDEOENC_POOL>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_POOL::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_POOL, ddr_id) - 0usize];
    ["Offset of field: _HD_VIDEOENC_POOL::counts"]
        [::core::mem::offset_of!(_HD_VIDEOENC_POOL, counts) - 4usize];
    ["Offset of field: _HD_VIDEOENC_POOL::max_counts"]
        [::core::mem::offset_of!(_HD_VIDEOENC_POOL, max_counts) - 8usize];
    ["Offset of field: _HD_VIDEOENC_POOL::min_counts"]
        [::core::mem::offset_of!(_HD_VIDEOENC_POOL, min_counts) - 12usize];
    ["Offset of field: _HD_VIDEOENC_POOL::mode"]
        [::core::mem::offset_of!(_HD_VIDEOENC_POOL, mode) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_PATH_CONFIG"]
        [::core::mem::size_of::<_HD_VIDEOENC_PATH_CONFIG>() - 120usize];
    ["Alignment of _HD_VIDEOENC_PATH_CONFIG"]
        [::core::mem::align_of::<_HD_VIDEOENC_PATH_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_PATH_CONFIG::max_mem"]
        [::core::mem::offset_of!(_HD_VIDEOENC_PATH_CONFIG, max_mem) - 0usize];
    ["Offset of field: _HD_VIDEOENC_PATH_CONFIG::isp_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_PATH_CONFIG, isp_id) - 36usize];
    ["Offset of field: _HD_VIDEOENC_PATH_CONFIG::data_pool"]
        [::core::mem::offset_of!(_HD_VIDEOENC_PATH_CONFIG, data_pool) - 40usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_FUNC_CONFIG"]
        [::core::mem::size_of::<_HD_VIDEOENC_FUNC_CONFIG>() - 8usize];
    ["Alignment of _HD_VIDEOENC_FUNC_CONFIG"]
        [::core::mem::align_of::<_HD_VIDEOENC_FUNC_CONFIG>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_FUNC_CONFIG::ddr_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_FUNC_CONFIG, ddr_id) - 0usize];
    ["Offset of field: _HD_VIDEOENC_FUNC_CONFIG::in_func"]
        [::core::mem::offset_of!(_HD_VIDEOENC_FUNC_CONFIG, in_func) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_FRC"][::core::mem::size_of::<_HD_VIDEOENC_FRC>() - 4usize];
    ["Alignment of _HD_VIDEOENC_FRC"][::core::mem::align_of::<_HD_VIDEOENC_FRC>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_FRC::frc"]
        [::core::mem::offset_of!(_HD_VIDEOENC_FRC, frc) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_IN"][::core::mem::size_of::<_HD_VIDEOENC_IN>() - 20usize];
    ["Alignment of _HD_VIDEOENC_IN"][::core::mem::align_of::<_HD_VIDEOENC_IN>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_IN::dim"]
        [::core::mem::offset_of!(_HD_VIDEOENC_IN, dim) - 0usize];
    ["Offset of field: _HD_VIDEOENC_IN::pxl_fmt"]
        [::core::mem::offset_of!(_HD_VIDEOENC_IN, pxl_fmt) - 8usize];
    ["Offset of field: _HD_VIDEOENC_IN::dir"]
        [::core::mem::offset_of!(_HD_VIDEOENC_IN, dir) - 12usize];
    ["Offset of field: _HD_VIDEOENC_IN::frc"]
        [::core::mem::offset_of!(_HD_VIDEOENC_IN, frc) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26X_CONFIG"][::core::mem::size_of::<_HD_H26X_CONFIG>() - 40usize];
    ["Alignment of _HD_H26X_CONFIG"][::core::mem::align_of::<_HD_H26X_CONFIG>() - 4usize];
    ["Offset of field: _HD_H26X_CONFIG::gop_num"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, gop_num) - 0usize];
    ["Offset of field: _HD_H26X_CONFIG::chrm_qp_idx"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, chrm_qp_idx) - 4usize];
    ["Offset of field: _HD_H26X_CONFIG::sec_chrm_qp_idx"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, sec_chrm_qp_idx) - 5usize];
    ["Offset of field: _HD_H26X_CONFIG::ltr_interval"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, ltr_interval) - 8usize];
    ["Offset of field: _HD_H26X_CONFIG::ltr_pre_ref"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, ltr_pre_ref) - 12usize];
    ["Offset of field: _HD_H26X_CONFIG::gray_en"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, gray_en) - 16usize];
    ["Offset of field: _HD_H26X_CONFIG::source_output"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, source_output) - 20usize];
    ["Offset of field: _HD_H26X_CONFIG::profile"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, profile) - 24usize];
    ["Offset of field: _HD_H26X_CONFIG::level_idc"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, level_idc) - 28usize];
    ["Offset of field: _HD_H26X_CONFIG::svc_layer"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, svc_layer) - 32usize];
    ["Offset of field: _HD_H26X_CONFIG::entropy_mode"]
        [::core::mem::offset_of!(_HD_H26X_CONFIG, entropy_mode) - 36usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_JPEG_CONFIG"][::core::mem::size_of::<_HD_JPEG_CONFIG>() - 8usize];
    ["Alignment of _HD_JPEG_CONFIG"][::core::mem::align_of::<_HD_JPEG_CONFIG>() - 4usize];
    ["Offset of field: _HD_JPEG_CONFIG::retstart_interval"]
        [::core::mem::offset_of!(_HD_JPEG_CONFIG, retstart_interval) - 0usize];
    ["Offset of field: _HD_JPEG_CONFIG::image_quality"]
        [::core::mem::offset_of!(_HD_JPEG_CONFIG, image_quality) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_JPEG_CONFIG2"][::core::mem::size_of::<_HD_JPEG_CONFIG2>() - 20usize];
    ["Alignment of _HD_JPEG_CONFIG2"][::core::mem::align_of::<_HD_JPEG_CONFIG2>() - 4usize];
    ["Offset of field: _HD_JPEG_CONFIG2::retstart_interval"]
        [::core::mem::offset_of!(_HD_JPEG_CONFIG2, retstart_interval) - 0usize];
    ["Offset of field: _HD_JPEG_CONFIG2::image_quality"]
        [::core::mem::offset_of!(_HD_JPEG_CONFIG2, image_quality) - 4usize];
    ["Offset of field: _HD_JPEG_CONFIG2::bitrate"]
        [::core::mem::offset_of!(_HD_JPEG_CONFIG2, bitrate) - 8usize];
    ["Offset of field: _HD_JPEG_CONFIG2::frame_rate_base"]
        [::core::mem::offset_of!(_HD_JPEG_CONFIG2, frame_rate_base) - 12usize];
    ["Offset of field: _HD_JPEG_CONFIG2::frame_rate_incr"]
        [::core::mem::offset_of!(_HD_JPEG_CONFIG2, frame_rate_incr) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_OUT__bindgen_ty_1"]
        [::core::mem::size_of::<_HD_VIDEOENC_OUT__bindgen_ty_1>() - 40usize];
    ["Alignment of _HD_VIDEOENC_OUT__bindgen_ty_1"]
        [::core::mem::align_of::<_HD_VIDEOENC_OUT__bindgen_ty_1>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_OUT__bindgen_ty_1::h26x"]
        [::core::mem::offset_of!(_HD_VIDEOENC_OUT__bindgen_ty_1, h26x) - 0usize];
    ["Offset of field: _HD_VIDEOENC_OUT__bindgen_ty_1::jpeg"]
        [::core::mem::offset_of!(_HD_VIDEOENC_OUT__bindgen_ty_1, jpeg) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_OUT"][::core::mem::size_of::<_HD_VIDEOENC_OUT>() - 44usize];
    ["Alignment of _HD_VIDEOENC_OUT"][::core::mem::align_of::<_HD_VIDEOENC_OUT>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_OUT::codec_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_OUT, codec_type) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_OUT2__bindgen_ty_1"]
        [::core::mem::size_of::<_HD_VIDEOENC_OUT2__bindgen_ty_1>() - 40usize];
    ["Alignment of _HD_VIDEOENC_OUT2__bindgen_ty_1"]
        [::core::mem::align_of::<_HD_VIDEOENC_OUT2__bindgen_ty_1>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_OUT2__bindgen_ty_1::h26x"]
        [::core::mem::offset_of!(_HD_VIDEOENC_OUT2__bindgen_ty_1, h26x) - 0usize];
    ["Offset of field: _HD_VIDEOENC_OUT2__bindgen_ty_1::jpeg"]
        [::core::mem::offset_of!(_HD_VIDEOENC_OUT2__bindgen_ty_1, jpeg) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_OUT2"][::core::mem::size_of::<_HD_VIDEOENC_OUT2>() - 44usize];
    ["Alignment of _HD_VIDEOENC_OUT2"][::core::mem::align_of::<_HD_VIDEOENC_OUT2>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_OUT2::codec_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_OUT2, codec_type) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_VUI"][::core::mem::size_of::<_HD_H26XENC_VUI>() - 24usize];
    ["Alignment of _HD_H26XENC_VUI"][::core::mem::align_of::<_HD_H26XENC_VUI>() - 4usize];
    ["Offset of field: _HD_H26XENC_VUI::vui_en"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, vui_en) - 0usize];
    ["Offset of field: _HD_H26XENC_VUI::sar_width"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, sar_width) - 4usize];
    ["Offset of field: _HD_H26XENC_VUI::sar_height"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, sar_height) - 8usize];
    ["Offset of field: _HD_H26XENC_VUI::matrix_coef"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, matrix_coef) - 12usize];
    ["Offset of field: _HD_H26XENC_VUI::transfer_characteristics"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, transfer_characteristics) - 13usize];
    ["Offset of field: _HD_H26XENC_VUI::colour_primaries"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, colour_primaries) - 14usize];
    ["Offset of field: _HD_H26XENC_VUI::video_format"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, video_format) - 15usize];
    ["Offset of field: _HD_H26XENC_VUI::color_range"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, color_range) - 16usize];
    ["Offset of field: _HD_H26XENC_VUI::timing_present_flag"]
        [::core::mem::offset_of!(_HD_H26XENC_VUI, timing_present_flag) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_DEBLOCK"][::core::mem::size_of::<_HD_H26XENC_DEBLOCK>() - 3usize];
    ["Alignment of _HD_H26XENC_DEBLOCK"][::core::mem::align_of::<_HD_H26XENC_DEBLOCK>() - 1usize];
    ["Offset of field: _HD_H26XENC_DEBLOCK::dis_ilf_idc"]
        [::core::mem::offset_of!(_HD_H26XENC_DEBLOCK, dis_ilf_idc) - 0usize];
    ["Offset of field: _HD_H26XENC_DEBLOCK::db_alpha"]
        [::core::mem::offset_of!(_HD_H26XENC_DEBLOCK, db_alpha) - 1usize];
    ["Offset of field: _HD_H26XENC_DEBLOCK::db_beta"]
        [::core::mem::offset_of!(_HD_H26XENC_DEBLOCK, db_beta) - 2usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_CBR"][::core::mem::size_of::<_HD_H26XENC_CBR>() - 28usize];
    ["Alignment of _HD_H26XENC_CBR"][::core::mem::align_of::<_HD_H26XENC_CBR>() - 4usize];
    ["Offset of field: _HD_H26XENC_CBR::bitrate"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, bitrate) - 0usize];
    ["Offset of field: _HD_H26XENC_CBR::frame_rate_base"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, frame_rate_base) - 4usize];
    ["Offset of field: _HD_H26XENC_CBR::frame_rate_incr"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, frame_rate_incr) - 8usize];
    ["Offset of field: _HD_H26XENC_CBR::init_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, init_i_qp) - 12usize];
    ["Offset of field: _HD_H26XENC_CBR::max_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, max_i_qp) - 13usize];
    ["Offset of field: _HD_H26XENC_CBR::min_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, min_i_qp) - 14usize];
    ["Offset of field: _HD_H26XENC_CBR::init_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, init_p_qp) - 15usize];
    ["Offset of field: _HD_H26XENC_CBR::max_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, max_p_qp) - 16usize];
    ["Offset of field: _HD_H26XENC_CBR::min_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, min_p_qp) - 17usize];
    ["Offset of field: _HD_H26XENC_CBR::static_time"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, static_time) - 20usize];
    ["Offset of field: _HD_H26XENC_CBR::ip_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR, ip_weight) - 24usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_CBR2"][::core::mem::size_of::<_HD_H26XENC_CBR2>() - 56usize];
    ["Alignment of _HD_H26XENC_CBR2"][::core::mem::align_of::<_HD_H26XENC_CBR2>() - 4usize];
    ["Offset of field: _HD_H26XENC_CBR2::bitrate"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, bitrate) - 0usize];
    ["Offset of field: _HD_H26XENC_CBR2::frame_rate_base"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, frame_rate_base) - 4usize];
    ["Offset of field: _HD_H26XENC_CBR2::frame_rate_incr"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, frame_rate_incr) - 8usize];
    ["Offset of field: _HD_H26XENC_CBR2::init_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, init_i_qp) - 12usize];
    ["Offset of field: _HD_H26XENC_CBR2::max_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, max_i_qp) - 13usize];
    ["Offset of field: _HD_H26XENC_CBR2::min_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, min_i_qp) - 14usize];
    ["Offset of field: _HD_H26XENC_CBR2::init_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, init_p_qp) - 15usize];
    ["Offset of field: _HD_H26XENC_CBR2::max_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, max_p_qp) - 16usize];
    ["Offset of field: _HD_H26XENC_CBR2::min_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, min_p_qp) - 17usize];
    ["Offset of field: _HD_H26XENC_CBR2::static_time"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, static_time) - 20usize];
    ["Offset of field: _HD_H26XENC_CBR2::ip_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, ip_weight) - 24usize];
    ["Offset of field: _HD_H26XENC_CBR2::key_p_period"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, key_p_period) - 28usize];
    ["Offset of field: _HD_H26XENC_CBR2::kp_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, kp_weight) - 32usize];
    ["Offset of field: _HD_H26XENC_CBR2::p2_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, p2_weight) - 36usize];
    ["Offset of field: _HD_H26XENC_CBR2::p3_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, p3_weight) - 40usize];
    ["Offset of field: _HD_H26XENC_CBR2::lt_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, lt_weight) - 44usize];
    ["Offset of field: _HD_H26XENC_CBR2::motion_aq_str"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, motion_aq_str) - 48usize];
    ["Offset of field: _HD_H26XENC_CBR2::max_frame_size"]
        [::core::mem::offset_of!(_HD_H26XENC_CBR2, max_frame_size) - 52usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_VBR"][::core::mem::size_of::<_HD_H26XENC_VBR>() - 32usize];
    ["Alignment of _HD_H26XENC_VBR"][::core::mem::align_of::<_HD_H26XENC_VBR>() - 4usize];
    ["Offset of field: _HD_H26XENC_VBR::bitrate"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, bitrate) - 0usize];
    ["Offset of field: _HD_H26XENC_VBR::frame_rate_base"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, frame_rate_base) - 4usize];
    ["Offset of field: _HD_H26XENC_VBR::frame_rate_incr"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, frame_rate_incr) - 8usize];
    ["Offset of field: _HD_H26XENC_VBR::init_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, init_i_qp) - 12usize];
    ["Offset of field: _HD_H26XENC_VBR::max_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, max_i_qp) - 13usize];
    ["Offset of field: _HD_H26XENC_VBR::min_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, min_i_qp) - 14usize];
    ["Offset of field: _HD_H26XENC_VBR::init_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, init_p_qp) - 15usize];
    ["Offset of field: _HD_H26XENC_VBR::max_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, max_p_qp) - 16usize];
    ["Offset of field: _HD_H26XENC_VBR::min_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, min_p_qp) - 17usize];
    ["Offset of field: _HD_H26XENC_VBR::static_time"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, static_time) - 20usize];
    ["Offset of field: _HD_H26XENC_VBR::ip_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, ip_weight) - 24usize];
    ["Offset of field: _HD_H26XENC_VBR::change_pos"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR, change_pos) - 28usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_VBR2"][::core::mem::size_of::<_HD_H26XENC_VBR2>() - 60usize];
    ["Alignment of _HD_H26XENC_VBR2"][::core::mem::align_of::<_HD_H26XENC_VBR2>() - 4usize];
    ["Offset of field: _HD_H26XENC_VBR2::bitrate"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, bitrate) - 0usize];
    ["Offset of field: _HD_H26XENC_VBR2::frame_rate_base"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, frame_rate_base) - 4usize];
    ["Offset of field: _HD_H26XENC_VBR2::frame_rate_incr"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, frame_rate_incr) - 8usize];
    ["Offset of field: _HD_H26XENC_VBR2::init_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, init_i_qp) - 12usize];
    ["Offset of field: _HD_H26XENC_VBR2::max_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, max_i_qp) - 13usize];
    ["Offset of field: _HD_H26XENC_VBR2::min_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, min_i_qp) - 14usize];
    ["Offset of field: _HD_H26XENC_VBR2::init_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, init_p_qp) - 15usize];
    ["Offset of field: _HD_H26XENC_VBR2::max_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, max_p_qp) - 16usize];
    ["Offset of field: _HD_H26XENC_VBR2::min_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, min_p_qp) - 17usize];
    ["Offset of field: _HD_H26XENC_VBR2::static_time"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, static_time) - 20usize];
    ["Offset of field: _HD_H26XENC_VBR2::ip_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, ip_weight) - 24usize];
    ["Offset of field: _HD_H26XENC_VBR2::change_pos"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, change_pos) - 28usize];
    ["Offset of field: _HD_H26XENC_VBR2::key_p_period"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, key_p_period) - 32usize];
    ["Offset of field: _HD_H26XENC_VBR2::kp_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, kp_weight) - 36usize];
    ["Offset of field: _HD_H26XENC_VBR2::p2_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, p2_weight) - 40usize];
    ["Offset of field: _HD_H26XENC_VBR2::p3_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, p3_weight) - 44usize];
    ["Offset of field: _HD_H26XENC_VBR2::lt_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, lt_weight) - 48usize];
    ["Offset of field: _HD_H26XENC_VBR2::motion_aq_str"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, motion_aq_str) - 52usize];
    ["Offset of field: _HD_H26XENC_VBR2::max_frame_size"]
        [::core::mem::offset_of!(_HD_H26XENC_VBR2, max_frame_size) - 56usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_FIXQP"][::core::mem::size_of::<_HD_H26XENC_FIXQP>() - 12usize];
    ["Alignment of _HD_H26XENC_FIXQP"][::core::mem::align_of::<_HD_H26XENC_FIXQP>() - 4usize];
    ["Offset of field: _HD_H26XENC_FIXQP::frame_rate_base"]
        [::core::mem::offset_of!(_HD_H26XENC_FIXQP, frame_rate_base) - 0usize];
    ["Offset of field: _HD_H26XENC_FIXQP::frame_rate_incr"]
        [::core::mem::offset_of!(_HD_H26XENC_FIXQP, frame_rate_incr) - 4usize];
    ["Offset of field: _HD_H26XENC_FIXQP::fix_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_FIXQP, fix_i_qp) - 8usize];
    ["Offset of field: _HD_H26XENC_FIXQP::fix_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_FIXQP, fix_p_qp) - 9usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_EVBR"][::core::mem::size_of::<_HD_H26XENC_EVBR>() - 60usize];
    ["Alignment of _HD_H26XENC_EVBR"][::core::mem::align_of::<_HD_H26XENC_EVBR>() - 4usize];
    ["Offset of field: _HD_H26XENC_EVBR::bitrate"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, bitrate) - 0usize];
    ["Offset of field: _HD_H26XENC_EVBR::frame_rate_base"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, frame_rate_base) - 4usize];
    ["Offset of field: _HD_H26XENC_EVBR::frame_rate_incr"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, frame_rate_incr) - 8usize];
    ["Offset of field: _HD_H26XENC_EVBR::init_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, init_i_qp) - 12usize];
    ["Offset of field: _HD_H26XENC_EVBR::max_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, max_i_qp) - 13usize];
    ["Offset of field: _HD_H26XENC_EVBR::min_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, min_i_qp) - 14usize];
    ["Offset of field: _HD_H26XENC_EVBR::init_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, init_p_qp) - 15usize];
    ["Offset of field: _HD_H26XENC_EVBR::max_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, max_p_qp) - 16usize];
    ["Offset of field: _HD_H26XENC_EVBR::min_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, min_p_qp) - 17usize];
    ["Offset of field: _HD_H26XENC_EVBR::static_time"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, static_time) - 20usize];
    ["Offset of field: _HD_H26XENC_EVBR::ip_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, ip_weight) - 24usize];
    ["Offset of field: _HD_H26XENC_EVBR::key_p_period"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, key_p_period) - 28usize];
    ["Offset of field: _HD_H26XENC_EVBR::kp_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, kp_weight) - 32usize];
    ["Offset of field: _HD_H26XENC_EVBR::still_frame_cnd"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, still_frame_cnd) - 36usize];
    ["Offset of field: _HD_H26XENC_EVBR::motion_ratio_thd"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, motion_ratio_thd) - 40usize];
    ["Offset of field: _HD_H26XENC_EVBR::motion_aq_str"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, motion_aq_str) - 44usize];
    ["Offset of field: _HD_H26XENC_EVBR::still_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, still_i_qp) - 48usize];
    ["Offset of field: _HD_H26XENC_EVBR::still_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, still_p_qp) - 52usize];
    ["Offset of field: _HD_H26XENC_EVBR::still_kp_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR, still_kp_qp) - 56usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_EVBR2"][::core::mem::size_of::<_HD_H26XENC_EVBR2>() - 76usize];
    ["Alignment of _HD_H26XENC_EVBR2"][::core::mem::align_of::<_HD_H26XENC_EVBR2>() - 4usize];
    ["Offset of field: _HD_H26XENC_EVBR2::bitrate"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, bitrate) - 0usize];
    ["Offset of field: _HD_H26XENC_EVBR2::frame_rate_base"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, frame_rate_base) - 4usize];
    ["Offset of field: _HD_H26XENC_EVBR2::frame_rate_incr"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, frame_rate_incr) - 8usize];
    ["Offset of field: _HD_H26XENC_EVBR2::init_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, init_i_qp) - 12usize];
    ["Offset of field: _HD_H26XENC_EVBR2::max_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, max_i_qp) - 13usize];
    ["Offset of field: _HD_H26XENC_EVBR2::min_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, min_i_qp) - 14usize];
    ["Offset of field: _HD_H26XENC_EVBR2::init_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, init_p_qp) - 15usize];
    ["Offset of field: _HD_H26XENC_EVBR2::max_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, max_p_qp) - 16usize];
    ["Offset of field: _HD_H26XENC_EVBR2::min_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, min_p_qp) - 17usize];
    ["Offset of field: _HD_H26XENC_EVBR2::static_time"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, static_time) - 20usize];
    ["Offset of field: _HD_H26XENC_EVBR2::ip_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, ip_weight) - 24usize];
    ["Offset of field: _HD_H26XENC_EVBR2::key_p_period"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, key_p_period) - 28usize];
    ["Offset of field: _HD_H26XENC_EVBR2::kp_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, kp_weight) - 32usize];
    ["Offset of field: _HD_H26XENC_EVBR2::still_frame_cnd"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, still_frame_cnd) - 36usize];
    ["Offset of field: _HD_H26XENC_EVBR2::motion_ratio_thd"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, motion_ratio_thd) - 40usize];
    ["Offset of field: _HD_H26XENC_EVBR2::motion_aq_str"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, motion_aq_str) - 44usize];
    ["Offset of field: _HD_H26XENC_EVBR2::still_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, still_i_qp) - 48usize];
    ["Offset of field: _HD_H26XENC_EVBR2::still_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, still_p_qp) - 52usize];
    ["Offset of field: _HD_H26XENC_EVBR2::still_kp_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, still_kp_qp) - 56usize];
    ["Offset of field: _HD_H26XENC_EVBR2::p2_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, p2_weight) - 60usize];
    ["Offset of field: _HD_H26XENC_EVBR2::p3_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, p3_weight) - 64usize];
    ["Offset of field: _HD_H26XENC_EVBR2::lt_weight"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, lt_weight) - 68usize];
    ["Offset of field: _HD_H26XENC_EVBR2::max_frame_size"]
        [::core::mem::offset_of!(_HD_H26XENC_EVBR2, max_frame_size) - 72usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_RATE_CONTROL__bindgen_ty_1"]
        [::core::mem::size_of::<_HD_H26XENC_RATE_CONTROL__bindgen_ty_1>() - 60usize];
    ["Alignment of _HD_H26XENC_RATE_CONTROL__bindgen_ty_1"]
        [::core::mem::align_of::<_HD_H26XENC_RATE_CONTROL__bindgen_ty_1>() - 4usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL__bindgen_ty_1::cbr"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL__bindgen_ty_1, cbr) - 0usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL__bindgen_ty_1::vbr"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL__bindgen_ty_1, vbr) - 0usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL__bindgen_ty_1::fixqp"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL__bindgen_ty_1, fixqp) - 0usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL__bindgen_ty_1::evbr"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL__bindgen_ty_1, evbr) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_RATE_CONTROL"]
        [::core::mem::size_of::<_HD_H26XENC_RATE_CONTROL>() - 64usize];
    ["Alignment of _HD_H26XENC_RATE_CONTROL"]
        [::core::mem::align_of::<_HD_H26XENC_RATE_CONTROL>() - 4usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL::rc_mode"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL, rc_mode) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_RATE_CONTROL2__bindgen_ty_1"]
        [::core::mem::size_of::<_HD_H26XENC_RATE_CONTROL2__bindgen_ty_1>() - 76usize];
    ["Alignment of _HD_H26XENC_RATE_CONTROL2__bindgen_ty_1"]
        [::core::mem::align_of::<_HD_H26XENC_RATE_CONTROL2__bindgen_ty_1>() - 4usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL2__bindgen_ty_1::cbr"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL2__bindgen_ty_1, cbr) - 0usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL2__bindgen_ty_1::vbr"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL2__bindgen_ty_1, vbr) - 0usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL2__bindgen_ty_1::fixqp"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL2__bindgen_ty_1, fixqp) - 0usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL2__bindgen_ty_1::evbr"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL2__bindgen_ty_1, evbr) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_RATE_CONTROL2"]
        [::core::mem::size_of::<_HD_H26XENC_RATE_CONTROL2>() - 80usize];
    ["Alignment of _HD_H26XENC_RATE_CONTROL2"]
        [::core::mem::align_of::<_HD_H26XENC_RATE_CONTROL2>() - 4usize];
    ["Offset of field: _HD_H26XENC_RATE_CONTROL2::rc_mode"]
        [::core::mem::offset_of!(_HD_H26XENC_RATE_CONTROL2, rc_mode) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_USR_QP"][::core::mem::size_of::<_HD_H26XENC_USR_QP>() - 8usize];
    ["Alignment of _HD_H26XENC_USR_QP"][::core::mem::align_of::<_HD_H26XENC_USR_QP>() - 4usize];
    ["Offset of field: _HD_H26XENC_USR_QP::enable"]
        [::core::mem::offset_of!(_HD_H26XENC_USR_QP, enable) - 0usize];
    ["Offset of field: _HD_H26XENC_USR_QP::qp_map_addr"]
        [::core::mem::offset_of!(_HD_H26XENC_USR_QP, qp_map_addr) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_SLICE_SPLIT"][::core::mem::size_of::<_HD_H26XENC_SLICE_SPLIT>() - 8usize];
    ["Alignment of _HD_H26XENC_SLICE_SPLIT"]
        [::core::mem::align_of::<_HD_H26XENC_SLICE_SPLIT>() - 4usize];
    ["Offset of field: _HD_H26XENC_SLICE_SPLIT::enable"]
        [::core::mem::offset_of!(_HD_H26XENC_SLICE_SPLIT, enable) - 0usize];
    ["Offset of field: _HD_H26XENC_SLICE_SPLIT::slice_row_num"]
        [::core::mem::offset_of!(_HD_H26XENC_SLICE_SPLIT, slice_row_num) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_GDR"][::core::mem::size_of::<_HD_H26XENC_GDR>() - 12usize];
    ["Alignment of _HD_H26XENC_GDR"][::core::mem::align_of::<_HD_H26XENC_GDR>() - 4usize];
    ["Offset of field: _HD_H26XENC_GDR::enable"]
        [::core::mem::offset_of!(_HD_H26XENC_GDR, enable) - 0usize];
    ["Offset of field: _HD_H26XENC_GDR::period"]
        [::core::mem::offset_of!(_HD_H26XENC_GDR, period) - 4usize];
    ["Offset of field: _HD_H26XENC_GDR::number"]
        [::core::mem::offset_of!(_HD_H26XENC_GDR, number) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_ROI_WIN"][::core::mem::size_of::<_HD_H26XENC_ROI_WIN>() - 28usize];
    ["Alignment of _HD_H26XENC_ROI_WIN"][::core::mem::align_of::<_HD_H26XENC_ROI_WIN>() - 4usize];
    ["Offset of field: _HD_H26XENC_ROI_WIN::enable"]
        [::core::mem::offset_of!(_HD_H26XENC_ROI_WIN, enable) - 0usize];
    ["Offset of field: _HD_H26XENC_ROI_WIN::rect"]
        [::core::mem::offset_of!(_HD_H26XENC_ROI_WIN, rect) - 4usize];
    ["Offset of field: _HD_H26XENC_ROI_WIN::mode"]
        [::core::mem::offset_of!(_HD_H26XENC_ROI_WIN, mode) - 20usize];
    ["Offset of field: _HD_H26XENC_ROI_WIN::qp"]
        [::core::mem::offset_of!(_HD_H26XENC_ROI_WIN, qp) - 24usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_ROI"][::core::mem::size_of::<_HD_H26XENC_ROI>() - 284usize];
    ["Alignment of _HD_H26XENC_ROI"][::core::mem::align_of::<_HD_H26XENC_ROI>() - 4usize];
    ["Offset of field: _HD_H26XENC_ROI::roi_qp_mode"]
        [::core::mem::offset_of!(_HD_H26XENC_ROI, roi_qp_mode) - 0usize];
    ["Offset of field: _HD_H26XENC_ROI::st_roi"]
        [::core::mem::offset_of!(_HD_H26XENC_ROI, st_roi) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_ROW_RC"][::core::mem::size_of::<_HD_H26XENC_ROW_RC>() - 12usize];
    ["Alignment of _HD_H26XENC_ROW_RC"][::core::mem::align_of::<_HD_H26XENC_ROW_RC>() - 4usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::enable"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, enable) - 0usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::i_qp_range"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, i_qp_range) - 4usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::i_qp_step"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, i_qp_step) - 5usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::p_qp_range"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, p_qp_range) - 6usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::p_qp_step"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, p_qp_step) - 7usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::min_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, min_i_qp) - 8usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::max_i_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, max_i_qp) - 9usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::min_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, min_p_qp) - 10usize];
    ["Offset of field: _HD_H26XENC_ROW_RC::max_p_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_ROW_RC, max_p_qp) - 11usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_AQ"][::core::mem::size_of::<_HD_H26XENC_AQ>() - 72usize];
    ["Alignment of _HD_H26XENC_AQ"][::core::mem::align_of::<_HD_H26XENC_AQ>() - 4usize];
    ["Offset of field: _HD_H26XENC_AQ::enable"]
        [::core::mem::offset_of!(_HD_H26XENC_AQ, enable) - 0usize];
    ["Offset of field: _HD_H26XENC_AQ::i_str"]
        [::core::mem::offset_of!(_HD_H26XENC_AQ, i_str) - 4usize];
    ["Offset of field: _HD_H26XENC_AQ::p_str"]
        [::core::mem::offset_of!(_HD_H26XENC_AQ, p_str) - 5usize];
    ["Offset of field: _HD_H26XENC_AQ::max_delta_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_AQ, max_delta_qp) - 6usize];
    ["Offset of field: _HD_H26XENC_AQ::min_delta_qp"]
        [::core::mem::offset_of!(_HD_H26XENC_AQ, min_delta_qp) - 7usize];
    ["Offset of field: _HD_H26XENC_AQ::depth"]
        [::core::mem::offset_of!(_HD_H26XENC_AQ, depth) - 8usize];
    ["Offset of field: _HD_H26XENC_AQ::thd_table"]
        [::core::mem::offset_of!(_HD_H26XENC_AQ, thd_table) - 10usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_ENC_QP_RATIO"][::core::mem::size_of::<_HD_VIDEO_ENC_QP_RATIO>() - 2usize];
    ["Alignment of _HD_VIDEO_ENC_QP_RATIO"]
        [::core::mem::align_of::<_HD_VIDEO_ENC_QP_RATIO>() - 1usize];
    ["Offset of field: _HD_VIDEO_ENC_QP_RATIO::qp"]
        [::core::mem::offset_of!(_HD_VIDEO_ENC_QP_RATIO, qp) - 0usize];
    ["Offset of field: _HD_VIDEO_ENC_QP_RATIO::ratio"]
        [::core::mem::offset_of!(_HD_VIDEO_ENC_QP_RATIO, ratio) - 1usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEO_ENC_MIN_COMPRESS_RATIO"]
        [::core::mem::size_of::<_HD_VIDEO_ENC_MIN_COMPRESS_RATIO>() - 8usize];
    ["Alignment of _HD_VIDEO_ENC_MIN_COMPRESS_RATIO"]
        [::core::mem::align_of::<_HD_VIDEO_ENC_MIN_COMPRESS_RATIO>() - 4usize];
    ["Offset of field: _HD_VIDEO_ENC_MIN_COMPRESS_RATIO::codec_type"]
        [::core::mem::offset_of!(_HD_VIDEO_ENC_MIN_COMPRESS_RATIO, codec_type) - 0usize];
    ["Offset of field: _HD_VIDEO_ENC_MIN_COMPRESS_RATIO::qp_ratio"]
        [::core::mem::offset_of!(_HD_VIDEO_ENC_MIN_COMPRESS_RATIO, qp_ratio) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_REQUEST_IFRAME"]
        [::core::mem::size_of::<_HD_H26XENC_REQUEST_IFRAME>() - 4usize];
    ["Alignment of _HD_H26XENC_REQUEST_IFRAME"]
        [::core::mem::align_of::<_HD_H26XENC_REQUEST_IFRAME>() - 4usize];
    ["Offset of field: _HD_H26XENC_REQUEST_IFRAME::enable"]
        [::core::mem::offset_of!(_HD_H26XENC_REQUEST_IFRAME, enable) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_H26XENC_TRIG_SNAPSHOT"]
        [::core::mem::size_of::<_HD_H26XENC_TRIG_SNAPSHOT>() - 12usize];
    ["Alignment of _HD_H26XENC_TRIG_SNAPSHOT"]
        [::core::mem::align_of::<_HD_H26XENC_TRIG_SNAPSHOT>() - 4usize];
    ["Offset of field: _HD_H26XENC_TRIG_SNAPSHOT::phy_addr"]
        [::core::mem::offset_of!(_HD_H26XENC_TRIG_SNAPSHOT, phy_addr) - 0usize];
    ["Offset of field: _HD_H26XENC_TRIG_SNAPSHOT::size"]
        [::core::mem::offset_of!(_HD_H26XENC_TRIG_SNAPSHOT, size) - 4usize];
    ["Offset of field: _HD_H26XENC_TRIG_SNAPSHOT::image_quality"]
        [::core::mem::offset_of!(_HD_H26XENC_TRIG_SNAPSHOT, image_quality) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of HD_ENC_RET_EVENT"][::core::mem::size_of::<HD_ENC_RET_EVENT>() - 16usize];
    ["Alignment of HD_ENC_RET_EVENT"][::core::mem::align_of::<HD_ENC_RET_EVENT>() - 4usize];
    ["Offset of field: HD_ENC_RET_EVENT::event"]
        [::core::mem::offset_of!(HD_ENC_RET_EVENT, event) - 0usize];
    ["Offset of field: HD_ENC_RET_EVENT::bs_size"]
        [::core::mem::offset_of!(HD_ENC_RET_EVENT, bs_size) - 4usize];
    ["Offset of field: HD_ENC_RET_EVENT::frame_type"]
        [::core::mem::offset_of!(HD_ENC_RET_EVENT, frame_type) - 8usize];
    ["Offset of field: HD_ENC_RET_EVENT::svc_layer_type"]
        [::core::mem::offset_of!(HD_ENC_RET_EVENT, svc_layer_type) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_POLL_LIST"][::core::mem::size_of::<_HD_VIDEOENC_POLL_LIST>() - 20usize];
    ["Alignment of _HD_VIDEOENC_POLL_LIST"]
        [::core::mem::align_of::<_HD_VIDEOENC_POLL_LIST>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_POLL_LIST::path_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_POLL_LIST, path_id) - 0usize];
    ["Offset of field: _HD_VIDEOENC_POLL_LIST::revent"]
        [::core::mem::offset_of!(_HD_VIDEOENC_POLL_LIST, revent) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_USER_BS_DATA"]
        [::core::mem::size_of::<_HD_VIDEOENC_USER_BS_DATA>() - 12usize];
    ["Alignment of _HD_VIDEOENC_USER_BS_DATA"]
        [::core::mem::align_of::<_HD_VIDEOENC_USER_BS_DATA>() - 4usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS_DATA::pack_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS_DATA, pack_type) - 0usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS_DATA::user_buf_addr"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS_DATA, user_buf_addr) - 4usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS_DATA::size"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS_DATA, size) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_USER_BS"][::core::mem::size_of::<_HD_VIDEOENC_USER_BS>() - 184usize];
    ["Alignment of _HD_VIDEOENC_USER_BS"][::core::mem::align_of::<_HD_VIDEOENC_USER_BS>() - 8usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::sign"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, sign) - 0usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::p_next"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, p_next) - 4usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::vcodec_format"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, vcodec_format) - 8usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::pack_num"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, pack_num) - 12usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::timestamp"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, timestamp) - 16usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::frame_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, frame_type) - 24usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::svc_layer_type"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, svc_layer_type) - 28usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::video_pack"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, video_pack) - 32usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::psnr_info"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, psnr_info) - 80usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::blk_info"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, blk_info) - 92usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::newbs_flag"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, newbs_flag) - 124usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::qp"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, qp) - 128usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::evbr_state"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, evbr_state) - 132usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::slice_offset"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, slice_offset) - 136usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::motion_ratio"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, motion_ratio) - 168usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::p_user_buf"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, p_user_buf) - 172usize];
    ["Offset of field: _HD_VIDEOENC_USER_BS::user_buf_size"]
        [::core::mem::offset_of!(_HD_VIDEOENC_USER_BS, user_buf_size) - 176usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_VIDEOENC_RECV_LIST"][::core::mem::size_of::<_HD_VIDEOENC_RECV_LIST>() - 200usize];
    ["Alignment of _HD_VIDEOENC_RECV_LIST"]
        [::core::mem::align_of::<_HD_VIDEOENC_RECV_LIST>() - 8usize];
    ["Offset of field: _HD_VIDEOENC_RECV_LIST::path_id"]
        [::core::mem::offset_of!(_HD_VIDEOENC_RECV_LIST, path_id) - 0usize];
    ["Offset of field: _HD_VIDEOENC_RECV_LIST::user_bs"]
        [::core::mem::offset_of!(_HD_VIDEOENC_RECV_LIST, user_bs) - 8usize];
    ["Offset of field: _HD_VIDEOENC_RECV_LIST::retval"]
        [::core::mem::offset_of!(_HD_VIDEOENC_RECV_LIST, retval) - 192usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_SSP_CONFIG"]
        [::core::mem::size_of::<_HD_AUDIOCAP_SSP_CONFIG>() - 144usize];
    ["Alignment of _HD_AUDIOCAP_SSP_CONFIG"]
        [::core::mem::align_of::<_HD_AUDIOCAP_SSP_CONFIG>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::ssp_num"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, ssp_num) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::enable"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, enable) - 16usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::ssp_chan"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, ssp_chan) - 32usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::sample_size"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, sample_size) - 48usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, sample_rate) - 64usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::ssp_clock"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, ssp_clock) - 80usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::bit_clock"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, bit_clock) - 96usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::ssp_master"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, ssp_master) - 112usize];
    ["Offset of field: _HD_AUDIOCAP_SSP_CONFIG::live_sound_ch"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SSP_CONFIG, live_sound_ch) - 128usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_DRV_CONFIG"]
        [::core::mem::size_of::<_HD_AUDIOCAP_DRV_CONFIG>() - 148usize];
    ["Alignment of _HD_AUDIOCAP_DRV_CONFIG"]
        [::core::mem::align_of::<_HD_AUDIOCAP_DRV_CONFIG>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_DRV_CONFIG::mono"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_DRV_CONFIG, mono) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_DRV_CONFIG::ssp_config"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_DRV_CONFIG, ssp_config) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_SYSINFO"][::core::mem::size_of::<_HD_AUDIOCAP_SYSINFO>() - 80usize];
    ["Alignment of _HD_AUDIOCAP_SYSINFO"][::core::mem::align_of::<_HD_AUDIOCAP_SYSINFO>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_SYSINFO::dev_id"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSINFO, dev_id) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_SYSINFO::cur_in_sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSINFO, cur_in_sample_rate) - 4usize];
    ["Offset of field: _HD_AUDIOCAP_SYSINFO::cur_sample_bit"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSINFO, cur_sample_bit) - 8usize];
    ["Offset of field: _HD_AUDIOCAP_SYSINFO::cur_mode"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSINFO, cur_mode) - 12usize];
    ["Offset of field: _HD_AUDIOCAP_SYSINFO::cur_out_sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSINFO, cur_out_sample_rate) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_IN"][::core::mem::size_of::<_HD_AUDIOCAP_IN>() - 16usize];
    ["Alignment of _HD_AUDIOCAP_IN"][::core::mem::align_of::<_HD_AUDIOCAP_IN>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_IN::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_IN, sample_rate) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_IN::sample_bit"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_IN, sample_bit) - 4usize];
    ["Offset of field: _HD_AUDIOCAP_IN::mode"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_IN, mode) - 8usize];
    ["Offset of field: _HD_AUDIOCAP_IN::frame_sample"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_IN, frame_sample) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_OUT"][::core::mem::size_of::<_HD_AUDIOCAP_OUT>() - 4usize];
    ["Alignment of _HD_AUDIOCAP_OUT"][::core::mem::align_of::<_HD_AUDIOCAP_OUT>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_OUT::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_OUT, sample_rate) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_AEC"][::core::mem::size_of::<_HD_AUDIOCAP_AEC>() - 36usize];
    ["Alignment of _HD_AUDIOCAP_AEC"][::core::mem::align_of::<_HD_AUDIOCAP_AEC>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::enabled"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, enabled) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::leak_estimate_enabled"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, leak_estimate_enabled) - 4usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::leak_estimate_value"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, leak_estimate_value) - 8usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::noise_cancel_level"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, noise_cancel_level) - 12usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::echo_cancel_level"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, echo_cancel_level) - 16usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::filter_length"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, filter_length) - 20usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::frame_size"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, frame_size) - 24usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::notch_radius"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, notch_radius) - 28usize];
    ["Offset of field: _HD_AUDIOCAP_AEC::lb_channel"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_AEC, lb_channel) - 32usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_ANR"][::core::mem::size_of::<_HD_AUDIOCAP_ANR>() - 16usize];
    ["Alignment of _HD_AUDIOCAP_ANR"][::core::mem::align_of::<_HD_AUDIOCAP_ANR>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_ANR::enabled"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_ANR, enabled) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_ANR::suppress_level"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_ANR, suppress_level) - 4usize];
    ["Offset of field: _HD_AUDIOCAP_ANR::hpf_cut_off_freq"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_ANR, hpf_cut_off_freq) - 8usize];
    ["Offset of field: _HD_AUDIOCAP_ANR::bias_sensitive"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_ANR, bias_sensitive) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_POOL"][::core::mem::size_of::<_HD_AUDIOCAP_POOL>() - 24usize];
    ["Alignment of _HD_AUDIOCAP_POOL"][::core::mem::align_of::<_HD_AUDIOCAP_POOL>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_POOL::ddr_id"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_POOL, ddr_id) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_POOL::frame_sample_size"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_POOL, frame_sample_size) - 4usize];
    ["Offset of field: _HD_AUDIOCAP_POOL::counts"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_POOL, counts) - 8usize];
    ["Offset of field: _HD_AUDIOCAP_POOL::max_counts"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_POOL, max_counts) - 12usize];
    ["Offset of field: _HD_AUDIOCAP_POOL::min_counts"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_POOL, min_counts) - 16usize];
    ["Offset of field: _HD_AUDIOCAP_POOL::mode"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_POOL, mode) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_DEV_CONFIG"]
        [::core::mem::size_of::<_HD_AUDIOCAP_DEV_CONFIG>() - 172usize];
    ["Alignment of _HD_AUDIOCAP_DEV_CONFIG"]
        [::core::mem::align_of::<_HD_AUDIOCAP_DEV_CONFIG>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_DEV_CONFIG::in_max"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_DEV_CONFIG, in_max) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_DEV_CONFIG::frame_num_max"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_DEV_CONFIG, frame_num_max) - 16usize];
    ["Offset of field: _HD_AUDIOCAP_DEV_CONFIG::out_max"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_DEV_CONFIG, out_max) - 20usize];
    ["Offset of field: _HD_AUDIOCAP_DEV_CONFIG::aec_max"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_DEV_CONFIG, aec_max) - 24usize];
    ["Offset of field: _HD_AUDIOCAP_DEV_CONFIG::anr_max"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_DEV_CONFIG, anr_max) - 60usize];
    ["Offset of field: _HD_AUDIOCAP_DEV_CONFIG::data_pool"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_DEV_CONFIG, data_pool) - 76usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_SYSCAPS"][::core::mem::size_of::<_HD_AUDIOCAP_SYSCAPS>() - 156usize];
    ["Alignment of _HD_AUDIOCAP_SYSCAPS"][::core::mem::align_of::<_HD_AUDIOCAP_SYSCAPS>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::dev_id"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, dev_id) - 0usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::chip_id"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, chip_id) - 4usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::max_in_count"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, max_in_count) - 8usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::max_out_count"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, max_out_count) - 12usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::dev_caps"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, dev_caps) - 16usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::in_caps"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, in_caps) - 20usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::out_caps"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, out_caps) - 24usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::support_in_sr"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, support_in_sr) - 88usize];
    ["Offset of field: _HD_AUDIOCAP_SYSCAPS::support_out_sr"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_SYSCAPS, support_out_sr) - 92usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_VOLUME"][::core::mem::size_of::<_HD_AUDIOCAP_VOLUME>() - 4usize];
    ["Alignment of _HD_AUDIOCAP_VOLUME"][::core::mem::align_of::<_HD_AUDIOCAP_VOLUME>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_VOLUME::volume"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_VOLUME, volume) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOCAP_BUFINFO"][::core::mem::size_of::<_HD_AUDIOCAP_BUFINFO>() - 8usize];
    ["Alignment of _HD_AUDIOCAP_BUFINFO"][::core::mem::align_of::<_HD_AUDIOCAP_BUFINFO>() - 4usize];
    ["Offset of field: _HD_AUDIOCAP_BUFINFO::buf_info"]
        [::core::mem::offset_of!(_HD_AUDIOCAP_BUFINFO, buf_info) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_SYSCAPS"][::core::mem::size_of::<_HD_AUDIOENC_SYSCAPS>() - 532usize];
    ["Alignment of _HD_AUDIOENC_SYSCAPS"][::core::mem::align_of::<_HD_AUDIOENC_SYSCAPS>() - 4usize];
    ["Offset of field: _HD_AUDIOENC_SYSCAPS::dev_id"]
        [::core::mem::offset_of!(_HD_AUDIOENC_SYSCAPS, dev_id) - 0usize];
    ["Offset of field: _HD_AUDIOENC_SYSCAPS::chip_id"]
        [::core::mem::offset_of!(_HD_AUDIOENC_SYSCAPS, chip_id) - 4usize];
    ["Offset of field: _HD_AUDIOENC_SYSCAPS::max_in_count"]
        [::core::mem::offset_of!(_HD_AUDIOENC_SYSCAPS, max_in_count) - 8usize];
    ["Offset of field: _HD_AUDIOENC_SYSCAPS::max_out_count"]
        [::core::mem::offset_of!(_HD_AUDIOENC_SYSCAPS, max_out_count) - 12usize];
    ["Offset of field: _HD_AUDIOENC_SYSCAPS::dev_caps"]
        [::core::mem::offset_of!(_HD_AUDIOENC_SYSCAPS, dev_caps) - 16usize];
    ["Offset of field: _HD_AUDIOENC_SYSCAPS::in_caps"]
        [::core::mem::offset_of!(_HD_AUDIOENC_SYSCAPS, in_caps) - 20usize];
    ["Offset of field: _HD_AUDIOENC_SYSCAPS::out_caps"]
        [::core::mem::offset_of!(_HD_AUDIOENC_SYSCAPS, out_caps) - 276usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_BUFINFO"][::core::mem::size_of::<_HD_AUDIOENC_BUFINFO>() - 8usize];
    ["Alignment of _HD_AUDIOENC_BUFINFO"][::core::mem::align_of::<_HD_AUDIOENC_BUFINFO>() - 4usize];
    ["Offset of field: _HD_AUDIOENC_BUFINFO::buf_info"]
        [::core::mem::offset_of!(_HD_AUDIOENC_BUFINFO, buf_info) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_MAXMEM"][::core::mem::size_of::<_HD_AUDIOENC_MAXMEM>() - 16usize];
    ["Alignment of _HD_AUDIOENC_MAXMEM"][::core::mem::align_of::<_HD_AUDIOENC_MAXMEM>() - 4usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM::codec_type"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM, codec_type) - 0usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM, sample_rate) - 4usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM::sample_bit"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM, sample_bit) - 8usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM::mode"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM, mode) - 12usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_PATH_CONFIG"]
        [::core::mem::size_of::<_HD_AUDIOENC_PATH_CONFIG>() - 16usize];
    ["Alignment of _HD_AUDIOENC_PATH_CONFIG"]
        [::core::mem::align_of::<_HD_AUDIOENC_PATH_CONFIG>() - 4usize];
    ["Offset of field: _HD_AUDIOENC_PATH_CONFIG::max_mem"]
        [::core::mem::offset_of!(_HD_AUDIOENC_PATH_CONFIG, max_mem) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_MAXMEM2"][::core::mem::size_of::<_HD_AUDIOENC_MAXMEM2>() - 20usize];
    ["Alignment of _HD_AUDIOENC_MAXMEM2"][::core::mem::align_of::<_HD_AUDIOENC_MAXMEM2>() - 4usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM2::codec_type"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM2, codec_type) - 0usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM2::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM2, sample_rate) - 4usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM2::sample_bit"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM2, sample_bit) - 8usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM2::mode"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM2, mode) - 12usize];
    ["Offset of field: _HD_AUDIOENC_MAXMEM2::buf_ms"]
        [::core::mem::offset_of!(_HD_AUDIOENC_MAXMEM2, buf_ms) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_PATH_CONFIG2"]
        [::core::mem::size_of::<_HD_AUDIOENC_PATH_CONFIG2>() - 20usize];
    ["Alignment of _HD_AUDIOENC_PATH_CONFIG2"]
        [::core::mem::align_of::<_HD_AUDIOENC_PATH_CONFIG2>() - 4usize];
    ["Offset of field: _HD_AUDIOENC_PATH_CONFIG2::max_mem"]
        [::core::mem::offset_of!(_HD_AUDIOENC_PATH_CONFIG2, max_mem) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_IN"][::core::mem::size_of::<_HD_AUDIOENC_IN>() - 12usize];
    ["Alignment of _HD_AUDIOENC_IN"][::core::mem::align_of::<_HD_AUDIOENC_IN>() - 4usize];
    ["Offset of field: _HD_AUDIOENC_IN::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOENC_IN, sample_rate) - 0usize];
    ["Offset of field: _HD_AUDIOENC_IN::sample_bit"]
        [::core::mem::offset_of!(_HD_AUDIOENC_IN, sample_bit) - 4usize];
    ["Offset of field: _HD_AUDIOENC_IN::mode"]
        [::core::mem::offset_of!(_HD_AUDIOENC_IN, mode) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_OUT"][::core::mem::size_of::<_HD_AUDIOENC_OUT>() - 8usize];
    ["Alignment of _HD_AUDIOENC_OUT"][::core::mem::align_of::<_HD_AUDIOENC_OUT>() - 4usize];
    ["Offset of field: _HD_AUDIOENC_OUT::codec_type"]
        [::core::mem::offset_of!(_HD_AUDIOENC_OUT, codec_type) - 0usize];
    ["Offset of field: _HD_AUDIOENC_OUT::aac_adts"]
        [::core::mem::offset_of!(_HD_AUDIOENC_OUT, aac_adts) - 4usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_USER_BS"][::core::mem::size_of::<_HD_AUDIOENC_USER_BS>() - 40usize];
    ["Alignment of _HD_AUDIOENC_USER_BS"][::core::mem::align_of::<_HD_AUDIOENC_USER_BS>() - 8usize];
    ["Offset of field: _HD_AUDIOENC_USER_BS::sign"]
        [::core::mem::offset_of!(_HD_AUDIOENC_USER_BS, sign) - 0usize];
    ["Offset of field: _HD_AUDIOENC_USER_BS::p_next"]
        [::core::mem::offset_of!(_HD_AUDIOENC_USER_BS, p_next) - 4usize];
    ["Offset of field: _HD_AUDIOENC_USER_BS::acodec_format"]
        [::core::mem::offset_of!(_HD_AUDIOENC_USER_BS, acodec_format) - 8usize];
    ["Offset of field: _HD_AUDIOENC_USER_BS::timestamp"]
        [::core::mem::offset_of!(_HD_AUDIOENC_USER_BS, timestamp) - 16usize];
    ["Offset of field: _HD_AUDIOENC_USER_BS::size"]
        [::core::mem::offset_of!(_HD_AUDIOENC_USER_BS, size) - 24usize];
    ["Offset of field: _HD_AUDIOENC_USER_BS::newbs_flag"]
        [::core::mem::offset_of!(_HD_AUDIOENC_USER_BS, newbs_flag) - 28usize];
    ["Offset of field: _HD_AUDIOENC_USER_BS::p_user_buf"]
        [::core::mem::offset_of!(_HD_AUDIOENC_USER_BS, p_user_buf) - 32usize];
    ["Offset of field: _HD_AUDIOENC_USER_BS::user_buf_size"]
        [::core::mem::offset_of!(_HD_AUDIOENC_USER_BS, user_buf_size) - 36usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOENC_RECV_LIST"][::core::mem::size_of::<_HD_AUDIOENC_RECV_LIST>() - 56usize];
    ["Alignment of _HD_AUDIOENC_RECV_LIST"]
        [::core::mem::align_of::<_HD_AUDIOENC_RECV_LIST>() - 8usize];
    ["Offset of field: _HD_AUDIOENC_RECV_LIST::path_id"]
        [::core::mem::offset_of!(_HD_AUDIOENC_RECV_LIST, path_id) - 0usize];
    ["Offset of field: _HD_AUDIOENC_RECV_LIST::user_bs"]
        [::core::mem::offset_of!(_HD_AUDIOENC_RECV_LIST, user_bs) - 8usize];
    ["Offset of field: _HD_AUDIOENC_RECV_LIST::retval"]
        [::core::mem::offset_of!(_HD_AUDIOENC_RECV_LIST, retval) - 48usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOOUT_SSP_CONFIG"]
        [::core::mem::size_of::<_HD_AUDIOOUT_SSP_CONFIG>() - 48usize];
    ["Alignment of _HD_AUDIOOUT_SSP_CONFIG"]
        [::core::mem::align_of::<_HD_AUDIOOUT_SSP_CONFIG>() - 4usize];
    ["Offset of field: _HD_AUDIOOUT_SSP_CONFIG::enable"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SSP_CONFIG, enable) - 0usize];
    ["Offset of field: _HD_AUDIOOUT_SSP_CONFIG::resample_ratio"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SSP_CONFIG, resample_ratio) - 16usize];
    ["Offset of field: _HD_AUDIOOUT_SSP_CONFIG::playback_chmap"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SSP_CONFIG, playback_chmap) - 32usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOOUT_DRV_CONFIG"]
        [::core::mem::size_of::<_HD_AUDIOOUT_DRV_CONFIG>() - 56usize];
    ["Alignment of _HD_AUDIOOUT_DRV_CONFIG"]
        [::core::mem::align_of::<_HD_AUDIOOUT_DRV_CONFIG>() - 4usize];
    ["Offset of field: _HD_AUDIOOUT_DRV_CONFIG::mono"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_DRV_CONFIG, mono) - 0usize];
    ["Offset of field: _HD_AUDIOOUT_DRV_CONFIG::output"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_DRV_CONFIG, output) - 4usize];
    ["Offset of field: _HD_AUDIOOUT_DRV_CONFIG::ssp_config"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_DRV_CONFIG, ssp_config) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOOUT_SYSINFO"][::core::mem::size_of::<_HD_AUDIOOUT_SYSINFO>() - 24usize];
    ["Alignment of _HD_AUDIOOUT_SYSINFO"][::core::mem::align_of::<_HD_AUDIOOUT_SYSINFO>() - 4usize];
    ["Offset of field: _HD_AUDIOOUT_SYSINFO::dev_id"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSINFO, dev_id) - 0usize];
    ["Offset of field: _HD_AUDIOOUT_SYSINFO::cur_out_sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSINFO, cur_out_sample_rate) - 4usize];
    ["Offset of field: _HD_AUDIOOUT_SYSINFO::cur_sample_bit"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSINFO, cur_sample_bit) - 8usize];
    ["Offset of field: _HD_AUDIOOUT_SYSINFO::cur_mode"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSINFO, cur_mode) - 12usize];
    ["Offset of field: _HD_AUDIOOUT_SYSINFO::cur_in_sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSINFO, cur_in_sample_rate) - 16usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOOUT_OUT"][::core::mem::size_of::<_HD_AUDIOOUT_OUT>() - 12usize];
    ["Alignment of _HD_AUDIOOUT_OUT"][::core::mem::align_of::<_HD_AUDIOOUT_OUT>() - 4usize];
    ["Offset of field: _HD_AUDIOOUT_OUT::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_OUT, sample_rate) - 0usize];
    ["Offset of field: _HD_AUDIOOUT_OUT::sample_bit"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_OUT, sample_bit) - 4usize];
    ["Offset of field: _HD_AUDIOOUT_OUT::mode"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_OUT, mode) - 8usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOOUT_IN"][::core::mem::size_of::<_HD_AUDIOOUT_IN>() - 4usize];
    ["Alignment of _HD_AUDIOOUT_IN"][::core::mem::align_of::<_HD_AUDIOOUT_IN>() - 4usize];
    ["Offset of field: _HD_AUDIOOUT_IN::sample_rate"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_IN, sample_rate) - 0usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOOUT_DEV_CONFIG"]
        [::core::mem::size_of::<_HD_AUDIOOUT_DEV_CONFIG>() - 24usize];
    ["Alignment of _HD_AUDIOOUT_DEV_CONFIG"]
        [::core::mem::align_of::<_HD_AUDIOOUT_DEV_CONFIG>() - 4usize];
    ["Offset of field: _HD_AUDIOOUT_DEV_CONFIG::out_max"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_DEV_CONFIG, out_max) - 0usize];
    ["Offset of field: _HD_AUDIOOUT_DEV_CONFIG::frame_sample_max"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_DEV_CONFIG, frame_sample_max) - 12usize];
    ["Offset of field: _HD_AUDIOOUT_DEV_CONFIG::frame_num_max"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_DEV_CONFIG, frame_num_max) - 16usize];
    ["Offset of field: _HD_AUDIOOUT_DEV_CONFIG::in_max"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_DEV_CONFIG, in_max) - 20usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOOUT_SYSCAPS"][::core::mem::size_of::<_HD_AUDIOOUT_SYSCAPS>() - 44usize];
    ["Alignment of _HD_AUDIOOUT_SYSCAPS"][::core::mem::align_of::<_HD_AUDIOOUT_SYSCAPS>() - 4usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::dev_id"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, dev_id) - 0usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::chip_id"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, chip_id) - 4usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::max_in_count"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, max_in_count) - 8usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::max_out_count"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, max_out_count) - 12usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::dev_caps"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, dev_caps) - 16usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::in_caps"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, in_caps) - 20usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::out_caps"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, out_caps) - 28usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::support_in_sr"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, support_in_sr) - 32usize];
    ["Offset of field: _HD_AUDIOOUT_SYSCAPS::support_out_sr"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_SYSCAPS, support_out_sr) - 40usize];
};

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _HD_AUDIOOUT_VOLUME"][::core::mem::size_of::<_HD_AUDIOOUT_VOLUME>() - 4usize];
    ["Alignment of _HD_AUDIOOUT_VOLUME"][::core::mem::align_of::<_HD_AUDIOOUT_VOLUME>() - 4usize];
    ["Offset of field: _HD_AUDIOOUT_VOLUME::volume"]
        [::core::mem::offset_of!(_HD_AUDIOOUT_VOLUME, volume) - 0usize];
};
