# Using the camera's AI hardware

The G3 camera already runs human/vehicle recognition. The easiest custom integration
is to consume those results; replacing the detector is a much larger job.

| Hardware / interface | What it does | Custom-software status |
|---|---|---|
| G3 ISAPI region/line events | Reports vendor human/vehicle classifications | Existing interface; configure event linkage on the exact firmware |
| G3 IVE, `/dev/ive` | Background subtraction, morphology, connected components | Vendor usage identified; independent integration untested |
| G3 NNIE, `/dev/nnie` | Neural inference inside `hikdsp` | Custom models not demonstrated |
| G3 `libPicDetect.so` | Exports create / inspect / destroy functions | API names found; standalone initialization/ABI unverified |
| Door-station MPP devices | Hardware video/audio pipeline | Vendor interfaces mapped; a full replacement stack was not built |
| Indoor Android panel | Hardware AVC decode for viewing | Different platform and media API from the cameras |

Video encoding, video decoding, image processing and neural inference are different
hardware functions. An open device node does not prove you can use it independently
while vendor software owns the pipeline.

NNIE needs a compatible mapper/runtime, supported model operators and quantization.
Copying a recent ONNX model onto the camera is not enough. No custom modern
human/vehicle model or benchmark is included. Start with the [G3 interfaces](../devices/ds-2cd2t87g2-l.md#recognition-hardware)
if you want to investigate that path.
