# RPCore image and stream decoder scope

Existing filters: Flate (PNG/TIFF predictors), ASCIIHex, ASCII85, RunLength; DCT/JPEG has a separate terminal image path. Other filters (including JPX, JBIG2, CCITT and LZW) are not implemented by generic filters::decode. This inventory is based on current source, not a complete standard conformance matrix.

B11.2a candidate: public filters::jpx::inspect, bounded raw JPEG 2000 SOC/SIZ and baseline single-codestream JP2 box inspection (signature, ftyp, jp2h/ihdr/bpcc, jp2c; 32/64-bit and to-end box lengths). Reference grid, tile origins/count, component precision/sign/subsampling, decoded-byte estimate, cross-header consistency. Limits: 100 MiB input and packed sample estimate, 16 components, 65536 tiles, 4096 boxes. Metadata only: unknown bounded boxes skipped, color/channel/coding style/tile payload not validated, no JPX pixel decode, no JPEG 2000 conformance claim. Generic stream decode still rejects JPX. Two full independent Pillow/OpenJPEG 2.5.4 fixtures plus synthetic boundary tests. No runtime dependency added.

After green CI: B11.2b main-header COD/COC/QCD/QCC coding-style and quantization parser, tile-part boundaries; then packet/entropy/wavelet/pixel stages based on actual source and independent reference fixtures. JBIG2 and CCITT/LZW planned separately. Never advertise rendered support from metadata parsing alone.
