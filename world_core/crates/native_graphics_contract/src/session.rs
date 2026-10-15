//! Persistent presented-session contracts and the Rust-owned session runtime.
//!
//! Authority model: the scene packet, camera motion, simulation, and evidence
//! all remain Rust-owned. The Julia worker receives the validated base packet
//! plus a typed camera override; it never re-seals, mutates, or re-derives
//! scene identity. Presented frames carry no certification evidence — Tier-A
//! captures are independently promoted through the offscreen path over the
//! same packet, so the presented loop stays cheap and evidence stays
//! fail-closed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use luxel_reference_runtime::{WorldArtifact, validate_world_artifact};

use crate::{
    CameraProjection, GraphicsCamera, GraphicsContractError, GraphicsScenePacket,
    GraphicsWorkerError, GraphicsWorkerSupervisor, PromotedFrame, Transform3d,
    input_session::accept_presented_samples, seal_scene_packet, validate_camera,
    validate_transform,
};

pub const PRESENTED_SESSION_REQUEST_SCHEMA: &str = "luxel.presented-session-request/v1";

/// Typed request to open a persistent presented session. The window extent is
/// the session's fixed camera extent: every frame, presented or captured, uses
/// exactly these dimensions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresentedSessionRequest {
    pub schema_version: String,
    pub session_id: String,
    pub width_px: u32,
    pub height_px: u32,
    pub vsync: bool,
}

impl PresentedSessionRequest {
    pub fn new(
        session_id: impl Into<String>,
        width_px: u32,
        height_px: u32,
        vsync: bool,
    ) -> Result<Self, GraphicsContractError> {
        let request = Self {
            schema_version: PRESENTED_SESSION_REQUEST_SCHEMA.to_owned(),
            session_id: session_id.into(),
            width_px,
            height_px,
            vsync,
        };
        request.validate()?;
        Ok(request)
    }

    fn validate(&self) -> Result<(), GraphicsContractError> {
        if self.schema_version != PRESENTED_SESSION_REQUEST_SCHEMA {
            return Err(GraphicsContractError::unsupported(format!(
                "unsupported presented-session request schema {}",
                self.schema_version
            )));
        }
        if self.session_id.trim().is_empty() || self.session_id.len() > 128 {
            return Err(GraphicsContractError::malformed(
                "session_id must contain 1 to 128 non-whitespace bytes".to_owned(),
            ));
        }
        if self.width_px == 0 || self.height_px == 0 {
            return Err(GraphicsContractError::malformed(
                "presented-session dimensions must be positive".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresentedSessionOpenReceipt {
    pub window_width_px: u32,
    pub window_height_px: u32,
    pub vsync: bool,
}

/// One batch of presented frames, measured by the adapter. Frame times are
/// wall-clock per presented frame in microseconds; percentile policy is
/// computed by Rust so it stays authoritative.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WindowFrameReport {
    pub frames_presented: u32,
    pub window_presented_frames: u64,
    pub frame_times_us: Vec<u64>,
}

impl WindowFrameReport {
    pub fn frame_time_us_at_percentile(&self, percentile: f64) -> Option<u64> {
        if self.frame_times_us.is_empty() || !(0.0..=100.0).contains(&percentile) {
            return None;
        }
        let mut times = self.frame_times_us.clone();
        times.sort_unstable();
        let index = ((percentile / 100.0) * (times.len() as f64 - 1.0)).round() as usize;
        times.get(index).copied()
    }
}

/// A request-side instance transform. It travels beside the sealed packet the
/// same way `camera_override` does; it never reseals presented identity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InstanceOverride {
    pub instance_id: String,
    pub transform: Transform3d,
}

/// A persistent presented Luxel session. Rust owns the packet, the camera, the
/// session tick, and every evidence claim; the worker only ever receives the
/// validated packet plus typed camera / instance overrides and reports
/// presented frames.
#[derive(Debug)]
pub struct PresentedGraphicsSession {
    base_packet: GraphicsScenePacket,
    world_artifact_id: String,
    world_artifact_sha256: String,
    spatial_fields_sha256: String,
    session_id: String,
    width_px: u32,
    height_px: u32,
    vsync: bool,
    camera_position: [f32; 3],
    camera_forward: [f32; 3],
    camera_up: Option<[f32; 3]>,
    camera_projection: Option<CameraProjection>,
    instance_overrides: BTreeMap<String, Transform3d>,
    tick: u64,
    frames_presented: u64,
    window_presented_frames: u64,
    window_open: bool,
}

impl PresentedGraphicsSession {
    /// Bind the session to a validated packet and world artifact. The base
    /// camera's projection, planes, and extent are frozen for the session; only
    /// position and forward move.
    pub fn new(
        request: &PresentedSessionRequest,
        packet: &GraphicsScenePacket,
        world: &WorldArtifact,
    ) -> Result<Self, GraphicsWorkerError> {
        request.validate().map_err(GraphicsWorkerError::contract)?;
        crate::validate_scene_packet(packet).map_err(GraphicsWorkerError::contract)?;
        validate_world_artifact(world).map_err(|error| {
            GraphicsWorkerError::provenance(format!("world artifact failed validation: {error}"))
        })?;
        if packet.body.world_artifact_id != world.artifact_id
            || packet.body.world_artifact_sha256 != world.artifact_sha256
            || packet.body.spatial_fields_sha256 != world.body.fields.spatial_sha256
        {
            return Err(GraphicsWorkerError::provenance(
                "presented session packet world provenance is detached from the validated world artifact",
            ));
        }
        if packet.body.camera.width_px != request.width_px
            || packet.body.camera.height_px != request.height_px
        {
            return Err(GraphicsWorkerError::provenance(
                "session camera extent must match the requested window extent",
            ));
        }
        Ok(Self {
            world_artifact_id: packet.body.world_artifact_id.clone(),
            world_artifact_sha256: packet.body.world_artifact_sha256.clone(),
            spatial_fields_sha256: packet.body.spatial_fields_sha256.clone(),
            base_packet: packet.clone(),
            session_id: request.session_id.clone(),
            width_px: request.width_px,
            height_px: request.height_px,
            vsync: request.vsync,
            camera_position: packet.body.camera.position_xyz_m,
            camera_forward: packet.body.camera.forward_xyz,
            camera_up: None,
            camera_projection: None,
            instance_overrides: BTreeMap::new(),
            tick: 0,
            frames_presented: 0,
            window_presented_frames: 0,
            window_open: false,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn frames_presented(&self) -> u64 {
        self.frames_presented
    }

    pub fn window_presented_frames(&self) -> u64 {
        self.window_presented_frames
    }

    pub fn camera_position(&self) -> [f32; 3] {
        self.camera_position
    }

    pub fn camera_forward(&self) -> [f32; 3] {
        self.camera_forward
    }

    pub fn camera_up(&self) -> [f32; 3] {
        self.camera_up
            .unwrap_or(self.base_packet.body.camera.up_xyz)
    }

    pub fn camera_projection(&self) -> CameraProjection {
        self.camera_projection
            .clone()
            .unwrap_or_else(|| self.base_packet.body.camera.projection.clone())
    }

    pub fn base_packet(&self) -> &GraphicsScenePacket {
        &self.base_packet
    }

    pub fn instance_overrides(&self) -> &BTreeMap<String, Transform3d> {
        &self.instance_overrides
    }

    pub fn instance_override(&self, instance_id: &str) -> Option<&Transform3d> {
        self.instance_overrides.get(instance_id)
    }

    pub fn is_window_open(&self) -> bool {
        self.window_open
    }

    /// Open the persistent window through the worker. A second open without an
    /// intervening close is a typed error, not a silent reuse.
    pub fn open_window(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
    ) -> Result<PresentedSessionOpenReceipt, GraphicsWorkerError> {
        if self.window_open {
            return Err(GraphicsWorkerError::protocol(
                "window session is already open; close it before reopening",
            ));
        }
        let response = supervisor.request(json!({
            "op": "open_window",
            "width_px": self.width_px,
            "height_px": self.height_px,
            "vsync": self.vsync,
        }))?;
        let window_width_px = u32::try_from(
            response["window_width_px"]
                .as_u64()
                .ok_or_else(|| GraphicsWorkerError::protocol("window receipt has no width"))?,
        )
        .map_err(|_| GraphicsWorkerError::protocol("window receipt width is not a u32"))?;
        let window_height_px = u32::try_from(
            response["window_height_px"]
                .as_u64()
                .ok_or_else(|| GraphicsWorkerError::protocol("window receipt has no height"))?,
        )
        .map_err(|_| GraphicsWorkerError::protocol("window receipt height is not a u32"))?;
        let vsync = response["vsync"]
            .as_bool()
            .ok_or_else(|| GraphicsWorkerError::protocol("window receipt has no vsync state"))?;
        if window_width_px != self.width_px
            || window_height_px != self.height_px
            || vsync != self.vsync
        {
            return Err(GraphicsWorkerError::provenance(
                "opened window extent or vsync state does not match the session request",
            ));
        }
        self.window_open = true;
        Ok(PresentedSessionOpenReceipt {
            window_width_px,
            window_height_px,
            vsync,
        })
    }

    /// Teleport the Rust-owned camera. Forward must be finite with positive
    /// length; the adapter validates the basis again at render time.
    pub fn set_camera(
        &mut self,
        position: [f32; 3],
        forward: [f32; 3],
    ) -> Result<(), GraphicsWorkerError> {
        for value in position.iter().chain(forward.iter()) {
            if !value.is_finite() {
                return Err(GraphicsWorkerError::protocol(
                    "camera position and forward must be finite",
                ));
            }
        }
        let length =
            (forward[0] * forward[0] + forward[1] * forward[1] + forward[2] * forward[2]).sqrt();
        if length <= f32::EPSILON {
            return Err(GraphicsWorkerError::protocol(
                "camera forward must have positive length",
            ));
        }
        self.camera_position = position;
        self.camera_forward = [
            forward[0] / length,
            forward[1] / length,
            forward[2] / length,
        ];
        Ok(())
    }

    /// Teleport the Rust-owned camera including an explicit up basis. Used by
    /// top-down play, where world-up is collinear with the look direction.
    pub fn set_camera_basis(
        &mut self,
        position: [f32; 3],
        forward: [f32; 3],
        up: [f32; 3],
    ) -> Result<(), GraphicsWorkerError> {
        self.set_camera(position, forward)?;
        let mut camera = self.camera_packet();
        camera.up_xyz = up;
        validate_camera(&camera).map_err(GraphicsWorkerError::contract)?;
        self.camera_up = Some(camera.up_xyz);
        Ok(())
    }

    /// Override the presented projection (orthographic top-down, or restore
    /// the packet's perspective). Travels on `camera_override`.
    pub fn set_camera_projection(
        &mut self,
        projection: CameraProjection,
    ) -> Result<(), GraphicsWorkerError> {
        let mut camera = self.camera_packet();
        camera.projection = projection;
        validate_camera(&camera).map_err(GraphicsWorkerError::contract)?;
        self.camera_projection = Some(camera.projection);
        Ok(())
    }

    /// Restore packet up and projection while keeping the current pose.
    pub fn clear_camera_shape(&mut self) {
        self.camera_up = None;
        self.camera_projection = None;
    }

    /// Bind a request-side transform for one instance in the sealed packet.
    /// The packet identity does not change.
    pub fn set_instance_override(
        &mut self,
        instance_id: &str,
        transform: Transform3d,
    ) -> Result<(), GraphicsWorkerError> {
        let mut matches = self
            .base_packet
            .body
            .instances
            .iter()
            .filter(|instance| instance.instance_id == instance_id);
        let Some(_) = matches.next() else {
            return Err(GraphicsWorkerError::provenance(format!(
                "instance override {instance_id} is missing from the base packet"
            )));
        };
        if matches.next().is_some() {
            return Err(GraphicsWorkerError::provenance(format!(
                "instance override {instance_id} is ambiguous in the base packet"
            )));
        }
        validate_transform(&transform).map_err(GraphicsWorkerError::contract)?;
        self.instance_overrides
            .insert(instance_id.to_owned(), transform);
        Ok(())
    }

    pub fn clear_instance_override(&mut self, instance_id: &str) {
        self.instance_overrides.remove(instance_id);
    }

    pub fn clear_instance_overrides(&mut self) {
        self.instance_overrides.clear();
    }

    /// Walk a deterministic waypoint path: for each waypoint the camera moves
    /// to it and faces the direction of travel. Waypoint ordering is the input
    /// trace; a repeated waypoint keeps the previous forward.
    pub fn walk_path(&mut self, waypoints: &[[f32; 3]]) -> Result<(), GraphicsWorkerError> {
        for waypoint in waypoints {
            let previous = self.camera_position;
            let mut forward = [
                waypoint[0] - previous[0],
                waypoint[1] - previous[1],
                waypoint[2] - previous[2],
            ];
            if forward.iter().all(|value| value.abs() <= f32::EPSILON) {
                forward = self.camera_forward;
            }
            self.set_camera(*waypoint, forward)?;
            self.tick = self.tick.saturating_add(1);
        }
        Ok(())
    }

    fn camera_packet(&self) -> GraphicsCamera {
        let base = &self.base_packet.body.camera;
        GraphicsCamera {
            camera_id: base.camera_id.clone(),
            projection: self
                .camera_projection
                .clone()
                .unwrap_or_else(|| base.projection.clone()),
            position_xyz_m: self.camera_position,
            forward_xyz: self.camera_forward,
            up_xyz: self.camera_up.unwrap_or(base.up_xyz),
            near_plane_m: base.near_plane_m,
            far_plane_m: base.far_plane_m,
            width_px: base.width_px,
            height_px: base.height_px,
        }
    }

    fn instance_override_list(&self) -> Vec<InstanceOverride> {
        self.instance_overrides
            .iter()
            .map(|(instance_id, transform)| InstanceOverride {
                instance_id: instance_id.clone(),
                transform: transform.clone(),
            })
            .collect()
    }

    /// Worker-free snapshot packet: the sealed base packet with the current
    /// camera and instance overrides baked in. Presented frames never send
    /// this; only offscreen capture reseals pose.
    pub fn capture_packet(&self) -> Result<GraphicsScenePacket, GraphicsWorkerError> {
        let camera = self.camera_packet();
        let camera_changed = camera != self.base_packet.body.camera;
        if !camera_changed && self.instance_overrides.is_empty() {
            return Ok(self.base_packet.clone());
        }
        let mut body = self.base_packet.body.clone();
        if camera_changed {
            body.camera = camera;
        }
        if !self.instance_overrides.is_empty() {
            for instance in &mut body.instances {
                if let Some(transform) = self.instance_overrides.get(&instance.instance_id) {
                    instance.transform = transform.clone();
                }
            }
        }
        seal_scene_packet(body).map_err(GraphicsWorkerError::contract)
    }

    /// Present `frame_count` frames of the current camera state. The packet
    /// identity never changes; the camera override travels beside it, and the
    /// base packet is sent untouched so the worker's identity check holds.
    pub fn present_frames(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
        frame_count: u32,
    ) -> Result<WindowFrameReport, GraphicsWorkerError> {
        self.present_frames_internal(supervisor, frame_count, false)
            .map(|(report, _)| report)
    }

    /// Present frames while the worker samples raw HID state once per frame.
    /// The returned samples are device-raw; only the input-session layer
    /// derives semantics from them. Sample ordering matches frame order.
    pub fn present_frames_with_input(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
        frame_count: u32,
    ) -> Result<(WindowFrameReport, Vec<crate::input_session::InputSample>), GraphicsWorkerError>
    {
        self.present_frames_internal(supervisor, frame_count, true)
    }

    fn present_frames_internal(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
        frame_count: u32,
        collect_input: bool,
    ) -> Result<(WindowFrameReport, Vec<crate::input_session::InputSample>), GraphicsWorkerError>
    {
        if !self.window_open {
            return Err(GraphicsWorkerError::protocol(
                "presented window is not open; open it before presenting frames",
            ));
        }
        if frame_count == 0 {
            return Err(GraphicsWorkerError::protocol(
                "frame_count must be at least one",
            ));
        }
        let camera_override = self.camera_packet();
        let unchanged_camera = camera_override == self.base_packet.body.camera;
        let mut request = json!({
            "op": "render_window",
            "packet": self.base_packet,
            "frame_count": frame_count,
            "expected_packet_sha256": self.base_packet.packet_sha256,
        });
        if collect_input {
            request["report_input"] = json!(true);
        }
        if !unchanged_camera {
            request["camera_override"] =
                serde_json::to_value(&camera_override).map_err(|error| {
                    GraphicsWorkerError::protocol(format!(
                        "camera override serialization failed: {error}"
                    ))
                })?;
        }
        if !self.instance_overrides.is_empty() {
            request["instance_overrides"] = serde_json::to_value(self.instance_override_list())
                .map_err(|error| {
                    GraphicsWorkerError::protocol(format!(
                        "instance override serialization failed: {error}"
                    ))
                })?;
        }
        let response = supervisor.request(request)?;
        let frames_presented =
            u32::try_from(response["frames_presented"].as_u64().ok_or_else(|| {
                GraphicsWorkerError::protocol("frame report has no presented count")
            })?)
            .map_err(|_| GraphicsWorkerError::protocol("presented frame count is not a u32"))?;
        let window_presented_frames = response["window_presented_frames"]
            .as_u64()
            .ok_or_else(|| GraphicsWorkerError::protocol("frame report has no window total"))?;
        let frame_times_us: Vec<u64> = serde_json::from_value(response["frame_times_us"].clone())
            .map_err(|error| {
            GraphicsWorkerError::protocol(format!("frame report times are not typed: {error}"))
        })?;
        let report = WindowFrameReport {
            frames_presented,
            window_presented_frames,
            frame_times_us,
        };
        if report.frames_presented == 0 || report.frames_presented > frame_count {
            return Err(GraphicsWorkerError::provenance(
                "presented frame count is outside the requested batch",
            ));
        }
        if report.window_presented_frames < u64::from(report.frames_presented) {
            return Err(GraphicsWorkerError::provenance(
                "window presented frame count is inconsistent with the batch",
            ));
        }
        self.frames_presented += u64::from(report.frames_presented);
        self.window_presented_frames = report.window_presented_frames;
        self.tick = self.tick.saturating_add(u64::from(report.frames_presented));
        let input_samples = if collect_input {
            match response.get("input_samples") {
                Some(value) => {
                    let samples: Vec<crate::input_session::InputSample> =
                        serde_json::from_value(value.clone()).map_err(|error| {
                            GraphicsWorkerError::protocol(format!(
                                "worker input samples are not typed: {error}"
                            ))
                        })?;
                    accept_presented_samples(samples, report.frames_presented)?
                }
                None => {
                    return Err(GraphicsWorkerError::protocol(
                        "input collection was requested but the response carries none",
                    ));
                }
            }
        } else {
            Vec::new()
        };
        Ok((report, input_samples))
    }

    /// Revalidate a world artifact against the session's bound world identity.
    /// Pure and worker-free so the binding invariant is testable in isolation.
    pub fn validate_bound_world(&self, world: &WorldArtifact) -> Result<(), GraphicsWorkerError> {
        validate_world_artifact(world).map_err(|error| {
            GraphicsWorkerError::provenance(format!("world artifact failed validation: {error}"))
        })?;
        if world.artifact_id != self.world_artifact_id
            || world.artifact_sha256 != self.world_artifact_sha256
            || world.body.fields.spatial_sha256 != self.spatial_fields_sha256
        {
            return Err(GraphicsWorkerError::provenance(
                "world artifact no longer matches the session's bound world identity",
            ));
        }
        Ok(())
    }

    /// Sample Tier-A evidence: promote an independently re-measured offscreen
    /// capture of the CURRENT session camera through the full Rust authority
    /// path. The presented loop itself never carries evidence. The world
    /// artifact is supplied by the caller (who built the session from it) and
    /// is revalidated here and again inside the supervisor.
    pub fn capture_and_promote(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
        world: &WorldArtifact,
    ) -> Result<PromotedFrame, GraphicsWorkerError> {
        self.validate_bound_world(world)?;
        let packet = self.capture_packet()?;
        supervisor.render_and_promote(&packet, world)
    }

    /// Close the window. The worker reports whether a window was actually
    /// open; closing an already-closed session is honest, not a fault.
    pub fn close_window(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
    ) -> Result<u64, GraphicsWorkerError> {
        let response = supervisor.request(json!({ "op": "close_window" }))?;
        self.window_open = false;
        let closed: bool = serde_json::from_value(response["closed"].clone()).map_err(|error| {
            GraphicsWorkerError::protocol(format!("close receipt is not typed: {error}"))
        })?;
        let presented_frames: u64 = serde_json::from_value(response["presented_frames"].clone())
            .map_err(|error| {
                GraphicsWorkerError::protocol(format!(
                    "close receipt has no presented frame count: {error}"
                ))
            })?;
        if closed || presented_frames > 0 {
            self.window_presented_frames = presented_frames;
        }
        // A repeat close gets the last known total: the worker honestly reports
        // zero for a session it no longer holds, but the session's own ledger
        // of presented frames is the durable answer the caller compares against.
        Ok(if closed {
            presented_frames
        } else {
            self.window_presented_frames
        })
    }
}
