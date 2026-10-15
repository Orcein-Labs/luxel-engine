//! Reopen actual CLI artifacts rather than relying on their producer's success.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use luxel_native_graphics_contract::{
    GraphicsFrameReceipt, GraphicsScenePacket, VisualQualityEvidence,
    depth_capture::{
        DepthCapturePayload, GroundingEvidence, VerifiedDepthCapture, measure_grounding,
    },
    visual_quality::{Campaign2VisualEvidence, validate_campaign2_visual_evidence_with_depth},
};
use serde::de::DeserializeOwned;
use std::{fs, path::Path};

fn read_json<T: DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&fs::read(path).expect("read artifact")).expect("parse artifact")
}

#[test]
#[ignore = "requires LUXEL_CAPTURE_ARTIFACTS pointing to completed contact or combined CLI output"]
fn revalidate_saved_depth_and_grounding() {
    let root = std::env::var_os("LUXEL_CAPTURE_ARTIFACTS").expect("artifact directory");
    for view in ["close", "medium", "wide"] {
        let directory = Path::new(&root).join(view);
        let packet: GraphicsScenePacket = read_json(&directory.join("graphics_scene_packet.json"));
        let certification: GraphicsFrameReceipt =
            read_json(&directory.join("graphics_frame_receipt.json"));
        let runtime: GraphicsFrameReceipt =
            read_json(&directory.join("graphics_runtime_frame_receipt.json"));
        let quality: VisualQualityEvidence =
            read_json(&directory.join("visual_quality_evidence.json"));
        let evidence: Campaign2VisualEvidence =
            read_json(&directory.join("campaign2_visual_evidence.json"));
        let grounding: GroundingEvidence = read_json(&directory.join("grounding_evidence.json"));
        let color = fs::read(directory.join("native_capture.rgba")).expect("color bytes");
        let bytes = fs::read(directory.join("view_depth.f32le")).expect("depth bytes");
        let metadata: serde_json::Value = read_json(&directory.join("view_depth.json"));
        assert_eq!(
            metadata["diagnostic_readback_bytes"].as_u64(),
            Some(bytes.len() as u64)
        );
        assert_eq!(
            metadata["color_capture_sha256"].as_str(),
            certification.body.capture_sha256.as_deref()
        );
        let mut payload = DepthCapturePayload {
            schema: metadata["schema"].as_str().unwrap().into(),
            packet_sha256: metadata["packet_sha256"].as_str().unwrap().into(),
            capture_id: metadata["capture_id"].as_str().unwrap().into(),
            width_px: metadata["width_px"].as_u64().unwrap().try_into().unwrap(),
            height_px: metadata["height_px"].as_u64().unwrap().try_into().unwrap(),
            depth_sha256: metadata["depth_sha256"].as_str().unwrap().into(),
            depth_base64: STANDARD.encode(&bytes),
        };
        let depth = VerifiedDepthCapture::verify(&packet, payload.clone()).expect("verify depth");
        assert_eq!(
            measure_grounding(&packet, &color, &depth).unwrap(),
            grounding
        );
        validate_campaign2_visual_evidence_with_depth(
            &evidence,
            &packet,
            &certification,
            &runtime,
            &quality,
            &color,
            &depth,
        )
        .expect("revalidate saved vector with actual color/depth");
        let mut corrupt = bytes;
        corrupt[0] ^= 1;
        payload.depth_base64 = STANDARD.encode(corrupt);
        assert!(VerifiedDepthCapture::verify(&packet, payload).is_err());
        eprintln!(
            "{view}: {} observed / {} candidates, contact {:?}",
            grounding.body.samples.len(),
            grounding.body.candidate_count,
            grounding.body.contact_fraction_bp
        );
    }
}
