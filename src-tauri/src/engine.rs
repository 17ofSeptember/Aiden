mod requests;
use crate::{
    actions::{InputController, NativeInput},
    graph::{Event, Runtime},
    model::{EventMode, Point, Sample},
    recognition,
    serial::{ArduinoSource, SignalSource},
    signal::Pipeline,
    simulation::Simulation,
    storage::{Storage, Workspace},
    training::Training,
    Error, Result,
};
use crossbeam_channel::{bounded, Receiver, Sender};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
pub struct Request {
    pub operation: String,
    pub data: Value,
    pub reply: Sender<std::result::Result<Value, String>>,
}
#[derive(Clone)]
pub struct Handle {
    pub commands: Sender<Request>,
    pub emergency: Arc<AtomicBool>,
    snapshot: Arc<Mutex<Value>>,
}
impl Handle {
    pub fn request(&self, operation: String, data: Value) -> std::result::Result<Value, String> {
        if operation == "snapshot" {
            return self
                .snapshot
                .lock()
                .map(|s| s.clone())
                .map_err(|_| "Snapshot unavailable".into());
        }
        if operation == "stop" {
            self.emergency.store(true, Ordering::SeqCst);
        }
        let (tx, rx) = bounded(1);
        self.commands
            .send_timeout(
                Request {
                    operation,
                    data,
                    reply: tx,
                },
                Duration::from_secs(1),
            )
            .map_err(|_| "Engine is busy".to_string())?;
        rx.recv_timeout(Duration::from_secs(10))
            .map_err(|_| "Engine response timed out".to_string())?
    }
}
enum SourceMessage {
    Ready(String, u16),
    Samples(Vec<Sample>, u64, u64),
    Failed(String),
}
struct SourceWorker {
    stop: Arc<AtomicBool>,
    rx: Receiver<SourceMessage>,
    join: Option<thread::JoinHandle<()>>,
}
impl SourceWorker {
    fn spawn(
        kind: String,
        port: String,
        baud: u32,
        rate: u32,
        playback: Vec<Sample>,
        emergency: Arc<AtomicBool>,
    ) -> Self {
        let (tx, rx) = bounded(64);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let join = thread::spawn(move || {
            let source: Result<(Box<dyn SignalSource>, u16)> = if kind == "hardware" {
                ArduinoSource::connect(&port, baud, rate).map(|s| {
                    let fw = s.firmware;
                    (Box::new(s) as Box<dyn SignalSource>, fw)
                })
            } else {
                Ok((Box::new(Simulation::new(rate, kind.clone(), playback)), 0))
            };
            match source {
                Ok((mut source, fw)) => {
                    if tx.send(SourceMessage::Ready(kind, fw)).is_err() {
                        return;
                    }
                    while !worker_stop.load(Ordering::Relaxed) {
                        match source.read() {
                            Ok(samples) => {
                                if !samples.is_empty() {
                                    let (d, c) = source.health();
                                    if tx.try_send(SourceMessage::Samples(samples, d, c)).is_err() {
                                        emergency.store(true, Ordering::SeqCst);
                                        let _ = tx.send_timeout(
                                            SourceMessage::Failed(
                                                "Acquisition queue overflow; monitoring stopped"
                                                    .into(),
                                            ),
                                            Duration::from_millis(100),
                                        );
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                emergency.store(true, Ordering::SeqCst);
                                let _ = tx.send_timeout(
                                    SourceMessage::Failed(e.to_string()),
                                    Duration::from_millis(100),
                                );
                                break;
                            }
                        }
                        thread::sleep(Duration::from_millis(1));
                    }
                    if let Err(e) = source.stop() {
                        tracing::warn!(error=%e,"source stop");
                    }
                }
                Err(e) => {
                    let _ = tx.send_timeout(
                        SourceMessage::Failed(e.to_string()),
                        Duration::from_millis(100),
                    );
                }
            }
        });
        Self {
            stop,
            rx,
            join: Some(join),
        }
    }
    fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for SourceWorker {
    fn drop(&mut self) {
        self.stop();
        if let Some(j) = self.join.take() {
            if j.join().is_err() {
                tracing::error!("Acquisition worker panicked");
            }
        }
    }
}
struct Engine {
    storage: Storage,
    workspace: Workspace,
    pipeline: Pipeline,
    graph: Runtime,
    training: Training,
    input: Option<NativeInput>,
    source: Option<SourceWorker>,
    mode: String,
    device: String,
    source_kind: String,
    error: String,
    firmware: u16,
    samples: u64,
    dropped: u64,
    corrupt: u64,
    leads: u8,
    adc: u16,
    points: VecDeque<Point>,
    recording: Option<Vec<Sample>>,
    record_name: String,
    scores: Vec<(String, f64)>,
    last_match: String,
    recognition_status: String,
    cooldowns: BTreeMap<String, u64>,
    active: Option<(String, f64)>,
    start: Instant,
    last_signal: Instant,
    dirty: bool,
    last_save: Instant,
    dsp_us: u64,
    recognition_us: u64,
    graph_us: u64,
    action_us: u64,
    shutdown: bool,
    emergency: Arc<AtomicBool>,
    reconnect: bool,
    last_connection: Option<Value>,
    retry_at: Instant,
}
pub fn start(path: &Path) -> Result<Handle> {
    let storage = Storage::open(path)?;
    let workspace = storage.load()?;
    let pipeline = Pipeline::new(workspace.dsp.clone())?;
    let graph = Runtime::new(workspace.graph.clone())?;
    let (commands, rx) = bounded(64);
    let snapshot = Arc::new(Mutex::new(json!({})));
    let emergency = Arc::new(AtomicBool::new(false));
    let handle = Handle {
        commands,
        emergency: emergency.clone(),
        snapshot: snapshot.clone(),
    };
    let mut engine = Engine {
        storage,
        workspace,
        pipeline,
        graph,
        training: Training::default(),
        input: None,
        source: None,
        mode: "Off".into(),
        device: "Disconnected".into(),
        source_kind: "none".into(),
        error: String::new(),
        firmware: 0,
        samples: 0,
        dropped: 0,
        corrupt: 0,
        leads: 0,
        adc: 512,
        points: VecDeque::with_capacity(1500),
        recording: None,
        record_name: String::new(),
        scores: vec![],
        last_match: String::new(),
        recognition_status: "Waiting for a completed contraction".into(),
        cooldowns: BTreeMap::new(),
        active: None,
        start: Instant::now(),
        last_signal: Instant::now(),
        dirty: false,
        last_save: Instant::now(),
        dsp_us: 0,
        recognition_us: 0,
        graph_us: 0,
        action_us: 0,
        shutdown: false,
        emergency,
        reconnect: false,
        last_connection: None,
        retry_at: Instant::now(),
    };
    thread::Builder::new()
        .name("aiden-engine".into())
        .spawn(move || {
            let mut publish = Instant::now();
            while !engine.shutdown {
                if engine.emergency.swap(false, Ordering::SeqCst) {
                    engine.stop();
                }
                for _ in 0..8 {
                    let Ok(request) = rx.try_recv() else { break };
                    let result = engine
                        .request(&request.operation, request.data)
                        .map_err(|e| e.to_string());
                    if let Err(e) = &result {
                        engine.error = e.clone();
                        tracing::warn!(operation=%request.operation, error=%e, "request failed");
                    }
                    let _ = request.reply.send(result);
                    if engine.shutdown {
                        break;
                    }
                }
                if engine.shutdown {
                    break;
                }
                engine.poll();
                if engine.dirty && engine.last_save.elapsed() > Duration::from_secs(1) {
                    match engine.storage.save(&engine.workspace) {
                        Ok(()) => engine.dirty = false,
                        Err(e) => engine.fail(e.to_string()),
                    }
                    engine.last_save = Instant::now();
                }
                if publish.elapsed() > Duration::from_millis(100) {
                    if let Ok(mut s) = snapshot.lock() {
                        *s = engine.snapshot();
                    }
                    publish = Instant::now();
                }
                thread::sleep(Duration::from_millis(2));
            }
            engine.stop();
        })?;
    Ok(handle)
}
impl Engine {
    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    fn stop(&mut self) {
        if self.mode != "Off" {
            tracing::info!("Monitoring stopped; pending actions cancelled");
        }
        self.mode = "Off".into();
        self.graph.stop();
        self.active = None;
        self.cooldowns.clear();
        if let Some(input) = self.input.as_mut() {
            if let Err(e) = input.release_all() {
                self.error = e.to_string();
                tracing::error!(error=%e,"release failed");
            }
        }
    }
    fn fail(&mut self, message: String) {
        self.stop();
        self.training.status.stage = "Idle".into();
        self.error = message;
        tracing::error!(error=%self.error,"engine stopped");
    }
    fn poll(&mut self) {
        for _ in 0..8 {
            let msg = self.source.as_ref().and_then(|s| s.rx.try_recv().ok());
            match msg {
                Some(SourceMessage::Ready(kind, fw)) => {
                    self.device = "Connected".into();
                    self.source_kind = kind;
                    self.firmware = fw;
                    self.last_signal = Instant::now();
                    tracing::info!(firmware = fw, "source connected");
                }
                Some(SourceMessage::Failed(e)) => {
                    self.fail(e);
                    self.device = "Disconnected".into();
                    self.source = None;
                    self.pipeline.reset();
                    self.retry_at = Instant::now() + Duration::from_secs(3);
                }
                Some(SourceMessage::Samples(samples, d, c)) => {
                    self.last_signal = Instant::now();
                    if d > self.dropped || c > self.corrupt {
                        self.fail("Serial integrity loss; monitoring stopped. Inspect diagnostics before rearming.".into());
                        self.pipeline.reset();
                    }
                    self.dropped = d;
                    self.corrupt = c;
                    for s in samples {
                        self.sample(s);
                    }
                }
                None => break,
            }
        }
        if self.mode != "Off" && self.last_signal.elapsed() > Duration::from_millis(500) {
            self.fail("Signal stale; monitoring stopped".into());
        }
        if self.device == "Disconnected" && self.reconnect && Instant::now() >= self.retry_at {
            if let Some(c) = self.last_connection.clone() {
                if let Err(e) = self.connect(c) {
                    self.error = e.to_string();
                }
                self.retry_at = Instant::now() + Duration::from_secs(3);
            }
        }
        let now = self.now();
        if self.mode != "Off" {
            let started = Instant::now();
            let native = self.mode == "Live";
            let input = &mut self.input;
            let action_us = &mut self.action_us;
            let result = self.graph.tick(now, &mut |n, e| {
                if native {
                    let start = Instant::now();
                    let result = input
                        .as_mut()
                        .ok_or_else(|| Error::Input("Native backend not armed".into()))?
                        .dispatch(n, e);
                    *action_us = start.elapsed().as_micros() as u64;
                    result
                } else {
                    Ok(())
                }
            });
            self.graph_us = started.elapsed().as_micros() as u64;
            if let Err(e) = result {
                self.fail(e.to_string());
            }
        }
        if let Some(input) = self.input.as_mut() {
            if let Err(e) = input.housekeeping() {
                self.fail(e.to_string());
            }
        }
    }
    fn sample(&mut self, s: Sample) {
        self.samples += 1;
        self.leads = s.leads;
        self.adc = s.adc;

        if s.leads != 0 || s.adc < 2 || s.adc > 1021 {
            tracing::error!(
                leads = s.leads,
                adc = s.adc,
                "BAD SAMPLE: lead-off or ADC clipping"
            );

            // Ignore invalid samples during training instead of silently
            // cancelling the entire training session.
            return;
        }
        let timer = Instant::now();
        self.pipeline.step(&s);
        self.dsp_us = timer.elapsed().as_micros() as u64;
        if let Some(recording) = self.recording.as_mut() {
            if recording.len() < 600000 {
                recording.push(s.clone());
            } else {
                self.error = "Recording reached 600000 samples; stop and save it".into();
            }
        }
        self.training.point(&self.pipeline.last);
        if self
            .samples
            .is_multiple_of((self.workspace.dsp.sample_rate / 100) as u64)
        {
            if self.points.len() == 1500 {
                self.points.pop_front();
            }
            self.points.push_back(self.pipeline.last.clone());
        }
        if let Some(e) = self.pipeline.completed.clone() {
            if let Some(id) = self.training.status.profile_id.clone() {
                if let Some(p) = self.workspace.profiles.iter_mut().find(|p| p.id == id) {
                    self.training.candidate(e.clone(), p);
                    if ["Train", "Negative", "Validate"]
                        .contains(&self.training.status.stage.as_str())
                    {
                        self.dirty = true;
                    }
                }
            }
            let timer = Instant::now();
            self.scores = recognition::ranked(&e, &self.workspace.profiles);
            self.recognition_status =
                if let Some((id, score)) = recognition::winner(&e, &self.workspace.profiles) {
                    let name = self
                        .workspace
                        .profiles
                        .iter()
                        .find(|profile| profile.id == id)
                        .map_or(id.as_str(), |profile| profile.name.as_str());
                    format!(
                        "Matched {name} ({:.1}%); monitoring {}",
                        score * 100.,
                        self.mode
                    )
                } else if let Some((id, score)) = self.scores.first() {
                    let profile = self
                        .workspace
                        .profiles
                        .iter()
                        .find(|profile| profile.id == *id);
                    match profile {
                    Some(profile) if *score < profile.threshold => format!(
                        "No trigger: best match {} {:.1}% is below {:.1}% threshold",
                        profile.name,
                        score * 100.,
                        profile.threshold * 100.
                    ),
                    Some(_) => {
                        "No trigger: competing profiles are too similar; ambiguity margin not met"
                            .into()
                    }
                    None => "No enabled action profile".into(),
                }
                } else {
                    "No enabled action profile".into()
                };
            if self.active.is_none() {
                if let Some((id, confidence)) = recognition::winner(&e, &self.workspace.profiles) {
                    let discrete = self
                        .workspace
                        .profiles
                        .iter()
                        .find(|p| p.id == id)
                        .is_some_and(|p| p.mode == EventMode::Discrete);
                    if discrete {
                        self.emit(&id, "TRIGGER", confidence, e.peak, e.duration_ms);
                    }
                }
            }
            self.recognition_us = timer.elapsed().as_micros() as u64;
        }
        if self.pipeline.released {
            if let Some((id, confidence)) = self.active.take() {
                self.emit(&id, "END", confidence, 0., 0.);
            }
        }
        if self.mode != "Off"
            && self.pipeline.active()
            && self
                .samples
                .is_multiple_of((self.workspace.dsp.sample_rate / 20) as u64)
        {
            if let Some((id, confidence)) = self.active.clone() {
                self.emit(&id, "ACTIVE", confidence, self.pipeline.last.envelope, 0.);
            } else if let Some(e) = self.pipeline.candidate() {
                if let Some((id, confidence)) = recognition::winner(&e, &self.workspace.profiles) {
                    if self
                        .workspace
                        .profiles
                        .iter()
                        .any(|p| p.id == id && p.mode != EventMode::Discrete)
                        && self.cooldowns.get(&id).is_none_or(|at| self.now() >= *at)
                    {
                        self.active = Some((id.clone(), confidence));
                        self.emit(&id, "START", confidence, e.peak, e.duration_ms);
                    }
                }
            }
        }
    }
    fn emit(&mut self, id: &str, kind: &str, confidence: f64, amplitude: f64, duration: f64) {
        let now = self.now();
        if kind == "TRIGGER" || kind == "START" {
            if self.cooldowns.get(id).is_some_and(|at| now < *at) {
                return;
            }
            if let Some(p) = self.workspace.profiles.iter().find(|p| p.id == id) {
                self.last_match = p.name.clone();
                self.cooldowns.insert(id.into(), now + p.cooldown_ms as u64);
            }
        }
        if self.mode == "Off" {
            return;
        }
        let event = Event {
            source: id.into(),
            kind: kind.into(),
            timestamp_ms: now,
            confidence,
            strength: (amplitude / self.workspace.dsp.onset / 4.).clamp(0., 1.),
            value: if kind == "END" { 0. } else { 1. },
            duration_ms: duration,
        };
        if let Err(e) = self.graph.inject(id, event) {
            self.fail(e.to_string());
        }
    }
    fn snapshot(&self) -> Value {
        json!({"mode":self.mode,"device":self.device,"source":self.source_kind,"firmware":self.firmware,"protocol":1,"rate":self.workspace.dsp.sample_rate,"samples":self.samples,"dropped":self.dropped,"corrupt":self.corrupt,"leads":self.leads,"adc":self.adc,"noise":self.pipeline.noise,"envelope":self.pipeline.last.envelope,"ready":self.pipeline.ready(),"points":self.points,"training":self.training.status,"scores":self.scores,"last_match":self.last_match,"recognition_status":self.recognition_status,"traces":self.graph.traces,"error":self.error,"recording":self.recording.as_ref().map(|r|r.len()),"diagnostics":{"version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"input":crate::actions::capability(),"database":self.storage.path,"schema":1,"logs":self.storage.path.parent().map(|p|p.join("logs")),"recovered":self.storage.recovered,"dsp_us":self.dsp_us,"recognition_us":self.recognition_us,"graph_us":self.graph_us,"action_us":self.action_us,"acquisition_latency":"Not measurable without synchronized device and PC clocks","graph_events":self.graph.dispatched}})
    }
}
