use super::*;
use crate::{model::Profile, require};
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PositionChange {
    id: String,
    x: f64,
    y: f64,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutChange {
    positions: Vec<PositionChange>,
}
impl Engine {
    pub(super) fn connect(&mut self, data: Value) -> Result<Value> {
        self.stop();
        require(
            self.recording.is_none(),
            "Stop and save the recording before changing sources",
        )?;
        self.training = Training::default();
        if let Some(mut s) = self.source.take() {
            s.stop();
        }
        self.pipeline.reset();
        self.samples = 0;
        self.dropped = 0;
        self.corrupt = 0;
        self.points.clear();
        self.device = "Disconnected".into();
        self.source_kind = "none".into();
        let kind = data["kind"].as_str().unwrap_or("hardware").to_string();
        require(
            ["hardware", "contraction", "sine", "noise", "playback"].contains(&kind.as_str()),
            "Unknown signal source",
        )?;
        let port = data["port"].as_str().unwrap_or("").to_string();
        let baud = u32::try_from(data["baud"].as_u64().unwrap_or(230400))
            .map_err(|_| Error::Invalid("Baud rate is out of range".into()))?;
        let mut rate = self.workspace.dsp.sample_rate;
        let mut playback = vec![];
        if kind == "playback" {
            let (r, s) = self.storage.recording(data["id"].as_str().unwrap_or(""))?;
            rate = r;
            require(
                rate == self.workspace.dsp.sample_rate,
                "Recording rate differs from workspace; change DSP rate first",
            )?;
            playback = s;
        }
        self.reconnect = data["reconnect"].as_bool().unwrap_or(false) && kind == "hardware";
        if kind == "hardware" {
            self.last_connection = Some(data.clone());
            self.storage.set_setting("device", &data)?;
        }
        self.source = Some(SourceWorker::spawn(
            kind,
            port,
            baud,
            rate,
            playback,
            self.emergency.clone(),
        ));
        self.device = "Connecting".into();
        self.error.clear();
        Ok(json!(true))
    }
    pub(super) fn request(&mut self, op: &str, data: Value) -> Result<Value> {
        match op {
            "load" => Ok(serde_json::to_value(&self.workspace)?),
            "layout" => {
                let change: LayoutChange = serde_json::from_value(data)?;
                require(change.positions.len() <= 256, "Too many node positions")?;
                let mut graph = self.workspace.graph.clone();
                let mut positions = Vec::with_capacity(change.positions.len());
                let mut ids = std::collections::BTreeSet::new();
                for position in change.positions {
                    require(
                        position.x.is_finite()
                            && position.y.is_finite()
                            && ids.insert(position.id.clone()),
                        "Invalid or duplicate node position",
                    )?;
                    let node = graph
                        .nodes
                        .iter_mut()
                        .find(|node| node.id == position.id)
                        .ok_or_else(|| Error::Invalid("Moved node no longer exists".into()))?;
                    node.x = position.x;
                    node.y = position.y;
                    positions.push((position.id, position.x, position.y));
                }
                // Positions do not alter graph behavior or replace training data.
                self.storage.save_positions(&positions)?;
                self.graph.graph = graph.clone();
                self.workspace.graph = graph;
                Ok(json!(true))
            }
            "save" => {
                self.stop();
                let workspace: Workspace = serde_json::from_value(data)?;
                workspace.validate()?;
                require(
                    !["Capture", "Train", "Negative", "Validate"]
                        .contains(&self.training.status.stage.as_str()),
                    "Stop training before editing the workspace",
                )?;
                if workspace.dsp != self.workspace.dsp {
                    require(
                        self.recording.is_none(),
                        "Stop and save the recording before changing DSP settings",
                    )?;
                    require(
                        workspace.profiles.iter().all(|p| p.examples.is_empty()),
                        "Clear or export trained profiles before changing DSP settings",
                    )?;
                    self.pipeline = Pipeline::new(workspace.dsp.clone())?;
                    if let Some(mut s) = self.source.take() {
                        s.stop();
                        self.device = "Disconnected".into();
                    }
                }
                self.graph = Runtime::new(workspace.graph.clone())?;
                self.storage.save(&workspace)?;
                self.workspace = workspace;
                self.dirty = false;
                Ok(json!(true))
            }
            "ports" => {
                let ports =
                    serialport::available_ports().map_err(|e| Error::Device(e.to_string()))?;
                Ok(json!(ports
                    .into_iter()
                    .map(|p| {
                        let description = match p.port_type {
                            serialport::SerialPortType::UsbPort(info) => format!(
                                "{} {:04x}:{:04x}",
                                info.product.unwrap_or_default(),
                                info.vid,
                                info.pid
                            ),
                            _ => "Serial device".into(),
                        };
                        json!({"port":p.port_name,"description":description})
                    })
                    .collect::<Vec<_>>()))
            }
            "device_settings" => Ok(self.storage.setting("device")?.unwrap_or(json!({}))),
            "connect" => self.connect(data),
            "disconnect" => {
                self.stop();
                self.reconnect = false;
                if let Some(mut s) = self.source.take() {
                    s.stop();
                }
                self.device = "Disconnected".into();
                self.source_kind = "none".into();
                self.pipeline.reset();
                self.training = Training::default();
                Ok(json!(true))
            }
            "stop" => {
                self.stop();
                Ok(json!(true))
            }
            "monitor" => {
                self.stop();
                let mode = data["mode"].as_str().unwrap_or("Test");
                require(["Test", "Live"].contains(&mode), "Unknown monitoring mode")?;
                require(
                    self.device == "Connected"
                        && self.pipeline.ready()
                        && self.leads == 0
                        && self.adc > 1
                        && self.adc < 1022,
                    "Connect a healthy source and wait for baseline stabilization",
                )?;
                require(
                    !["Capture", "Train", "Negative", "Validate"]
                        .contains(&self.training.status.stage.as_str()),
                    "Stop training before monitoring",
                )?;
                require(
                    self.workspace
                        .profiles
                        .iter()
                        .any(|p| p.enabled && p.examples.iter().any(|e| !e.negative)),
                    "Create and train an action profile first",
                )?;
                if mode == "Live" {
                    require(
                        self.source_kind == "hardware",
                        "Simulation and playback are restricted to Test Graph mode",
                    )?;
                    require(
                        data["confirmed"].as_bool() == Some(true),
                        "Explicit computer-control confirmation required",
                    )?;
                    if self.input.is_none() {
                        self.input = Some(NativeInput::new()?);
                    }
                    let outputs = self
                        .workspace
                        .graph
                        .connected_outputs(&self.workspace.profiles);
                    require(!outputs.is_empty(), "Connect an enabled, trained Muscle Action to a PC output before enabling control")?;
                    for node in outputs {
                        crate::actions::validate_ready(node)
                            .map_err(|error| Error::Invalid(format!("{}: {error}", node.label)))?;
                        if let Some(input) = &self.input {
                            input.check_approval(node).map_err(|error| {
                                Error::Invalid(format!("{}: {error}", node.label))
                            })?;
                        }
                    }
                }
                self.mode = mode.into();
                tracing::info!(mode=%self.mode, "Monitoring armed");
                self.pipeline.inhibit_until_rest();
                self.error.clear();
                Ok(json!(true))
            }
            "approve" => {
                self.stop();
                let id = data["id"].as_str().unwrap_or("");
                let n = self
                    .workspace
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .ok_or_else(|| Error::Invalid("Node not found".into()))?;
                require(
                    data["confirmed"].as_bool() == Some(true),
                    "Review and confirm exact executable/arguments",
                )?;
                if self.input.is_none() {
                    self.input = Some(NativeInput::new()?);
                }
                if let Some(input) = self.input.as_mut() {
                    input.approve(n)?;
                }
                Ok(json!(true))
            }
            "create_profile" => {
                self.stop();
                require(self.workspace.profiles.len() < 32, "Maximum 32 profiles")?;
                let p = Profile::new(
                    data["name"].as_str().unwrap_or("New action").into(),
                    self.workspace.dsp.clone(),
                );
                p.validate()?;
                let value = serde_json::to_value(&p)?;
                self.workspace.profiles.push(p);
                self.dirty = true;
                Ok(value)
            }
            "training" => {
                self.stop();
                let id = data["id"].as_str().unwrap_or("").to_string();
                let index = self
                    .workspace
                    .profiles
                    .iter()
                    .position(|p| p.id == id)
                    .ok_or_else(|| Error::Invalid("Profile not found".into()))?;
                let action = data["action"].as_str().unwrap_or("");
                if ["Done", "Crop", "Accept", "Reject", "Undo"].contains(&action) {
                    require(
                        self.training.status.profile_id.as_deref() == Some(&id),
                        "Training session belongs to another profile",
                    )?;
                }
                match action {
                    "Capture" | "Train" | "Negative" | "Validate" | "Paused" | "Idle" => {
                        require(
                            self.device == "Connected" || action == "Idle" || action == "Paused",
                            "Connect a signal source first",
                        )?;
                        if action == "Train" || action == "Validate" {
                            require(
                                self.workspace.profiles[index]
                                    .examples
                                    .iter()
                                    .any(|e| !e.negative),
                                "Capture and accept a reference first",
                            )?;
                        }
                        self.training.start(
                            id,
                            action.into(),
                            self.pipeline.last.time_ms,
                            data["variant"].as_str().unwrap_or("Default").into(),
                        )?;
                    }
                    "Done" => self.training.finish(
                        self.workspace.dsp.sample_rate,
                        self.workspace.dsp.onset,
                        self.workspace.dsp.release,
                    )?,
                    "Crop" => {
                        let start = data["start"].as_u64().unwrap_or(0) as usize;
                        let end = data["end"].as_u64().unwrap_or(0) as usize;
                        self.training
                            .crop(start, end, self.workspace.dsp.sample_rate)?;
                    }
                    "Accept" => self.training.accept(&mut self.workspace.profiles[index])?,
                    "Reject" => {
                        if let Some(candidate) = &self.training.status.candidate {
                            let profile = &mut self.workspace.profiles[index];
                            let old_len = profile.examples.len();
                            profile.examples.retain(|e| e.id != candidate.id);
                            if profile.examples.len() != old_len {
                                self.training.status.accepted =
                                    self.training.status.accepted.saturating_sub(1);
                            }
                        }
                        self.training.status.candidate = None;
                        self.training.status.rejected += 1;
                    }
                    "Undo" => {
                        self.workspace.profiles[index].examples.pop();
                    }
                    "Delete" => {
                        let eid = data["example_id"].as_str().unwrap_or("");
                        self.workspace.profiles[index]
                            .examples
                            .retain(|e| e.id != eid);
                    }
                    "Clear" => {
                        require(
                            data["confirmed"].as_bool() == Some(true),
                            "Confirm deletion of training data",
                        )?;
                        self.workspace.profiles[index].examples.clear();
                        self.training = Training::default();
                    }
                    "Missed" => self.workspace.profiles[index].validation.missed += 1,
                    "False" => self.workspace.profiles[index].validation.false_triggers += 1,
                    _ => return Err(Error::Invalid("Unknown training action".into())),
                }
                self.workspace.profiles[index].updated_at = crate::model::epoch_ms();
                self.dirty = true;
                Ok(serde_json::to_value(&self.workspace.profiles[index])?)
            }
            "record_start" => {
                require(self.device == "Connected", "Connect a source first")?;
                require(self.recording.is_none(), "Recording is already running")?;
                self.recording = Some(Vec::with_capacity(600000));
                self.record_name = data["name"]
                    .as_str()
                    .unwrap_or("Session")
                    .chars()
                    .take(100)
                    .collect();
                Ok(json!(true))
            }
            "record_stop" => {
                let recording = self
                    .recording
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("No active recording".into()))?;
                let id = self.storage.record(
                    &self.record_name,
                    self.workspace.dsp.sample_rate,
                    recording,
                )?;
                self.recording = None;
                Ok(json!(id))
            }
            "recordings" => Ok(json!(self.storage.recordings()?)),
            "backup" => Ok(json!(self.storage.backup()?)),
            "export" => {
                self.storage.save(&self.workspace)?;
                Ok(serde_json::to_value(&self.workspace)?)
            }
            "import" => {
                self.stop();
                let workspace: Workspace = serde_json::from_value(data)?;
                workspace.validate()?;
                self.storage.backup()?;
                self.request("save", serde_json::to_value(workspace)?)
            }
            "mark" => {
                let now = self.now();
                self.graph.trace(now, "manual", "MARK", "User-marked event");
                Ok(json!(true))
            }
            "acknowledge" => {
                self.error.clear();
                Ok(json!(true))
            }
            "shutdown" => {
                self.shutdown = true;
                self.stop();
                self.source.take();
                if let Some(recording) = self.recording.take() {
                    if !recording.is_empty() {
                        self.storage.record(
                            &self.record_name,
                            self.workspace.dsp.sample_rate,
                            &recording,
                        )?;
                    }
                }
                self.storage.save(&self.workspace)?;
                self.storage.clean_shutdown()?;
                Ok(json!(true))
            }
            _ => Err(Error::Invalid("Unknown operation".into())),
        }
    }
}
