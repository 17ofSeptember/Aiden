use crate::{require, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub params: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub port: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
pub const KINDS: &[&str] = &[
    "Muscle Action",
    "Left Click",
    "Right Click",
    "Middle Click",
    "Mouse Down",
    "Mouse Up",
    "Move Mouse",
    "Scroll Vertical",
    "Scroll Horizontal",
    "Key Press",
    "Key Down",
    "Key Up",
    "Shortcut",
    "Type Text",
    "Launch Application",
    "Open File",
    "Open Folder",
    "Open URL",
    "Approved Command",
    "Play/Pause",
    "Next Track",
    "Previous Track",
    "Volume Up",
    "Volume Down",
    "Mute",
    "Delay",
    "Timer",
    "Interval",
    "Debounce",
    "Cooldown",
    "Pulse",
    "Hold",
    "Repeat",
    "AND",
    "OR",
    "NOT",
    "Gate",
    "Toggle",
    "Latch",
    "Counter",
    "Compare",
    "Branch",
    "Debug",
];
impl Node {
    pub fn text(&self, key: &str) -> &str {
        self.params.get(key).and_then(|v| v.as_str()).unwrap_or("")
    }
    pub fn number(&self, key: &str, default: f64) -> f64 {
        self.params
            .get(key)
            .and_then(|v| v.as_f64())
            .unwrap_or(default)
    }
}
impl Graph {
    pub fn connected_outputs(&self, profiles: &[crate::model::Profile]) -> Vec<&Node> {
        let mut reachable: BTreeSet<&str> = self
            .nodes
            .iter()
            .filter(|node| {
                node.kind == "Muscle Action"
                    && profiles.iter().any(|profile| {
                        profile.id == node.text("profile_id")
                            && profile.enabled
                            && profile.examples.iter().any(|example| !example.negative)
                    })
            })
            .map(|node| node.id.as_str())
            .collect();
        loop {
            let before = reachable.len();
            for edge in &self.edges {
                if reachable.contains(edge.source.as_str()) {
                    reachable.insert(edge.target.as_str());
                }
            }
            if reachable.len() == before {
                break;
            }
        }
        self.nodes
            .iter()
            .filter(|node| {
                reachable.contains(node.id.as_str()) && crate::actions::is_output(&node.kind)
            })
            .collect()
    }
    pub fn validate(&self) -> Result<()> {
        require(
            self.nodes.len() <= 256 && self.edges.len() <= 1024,
            "Graph exceeds node/edge limit",
        )?;
        let ids: BTreeSet<_> = self.nodes.iter().map(|n| &n.id).collect();
        require(ids.len() == self.nodes.len(), "Duplicate node IDs")?;
        let edge_ids: BTreeSet<_> = self.edges.iter().map(|e| &e.id).collect();
        require(edge_ids.len() == self.edges.len(), "Duplicate edge IDs")?;
        for n in &self.nodes {
            require(
                !n.id.is_empty()
                    && n.id.len() <= 100
                    && n.label.len() <= 100
                    && n.x.is_finite()
                    && n.y.is_finite()
                    && KINDS.contains(&n.kind.as_str())
                    && n.params.is_object()
                    && n.params.to_string().len() < 20000,
                "Invalid node",
            )?;
            for key in [
                "ms",
                "interval",
                "count",
                "x",
                "y",
                "value",
                "speed",
                "max_speed",
                "dead_zone",
                "acceleration",
            ] {
                if let Some(v) = n.params.get(key) {
                    require(
                        v.as_f64()
                            .is_some_and(|v| v.is_finite() && v.abs() <= 60000.),
                        "Invalid numeric node parameter",
                    )?;
                }
            }
            require(
                (1. ..=60000.).contains(&n.number("ms", 500.))
                    && (10. ..=60000.).contains(&n.number("interval", 100.))
                    && (1. ..=100.).contains(&n.number("count", 1.)),
                "Timer/count limits exceeded",
            )?;
            if [
                "Launch Application",
                "Approved Command",
                "Open File",
                "Open Folder",
                "Open URL",
            ]
            .contains(&n.kind.as_str())
            {
                crate::actions::validate_system(n)?;
            }
        }
        let mut indegree: BTreeMap<&String, usize> = ids.iter().map(|id| (*id, 0)).collect();
        for e in &self.edges {
            require(
                ids.contains(&e.source)
                    && ids.contains(&e.target)
                    && [
                        "any", "TRIGGER", "START", "ACTIVE", "END", "true", "false", "start",
                        "stop", "reset",
                    ]
                    .contains(&e.port.as_str()),
                "Invalid edge endpoint/port",
            )?;
            if let Some(d) = indegree.get_mut(&e.target) {
                *d += 1;
            }
        }
        let mut queue: VecDeque<_> = indegree
            .iter()
            .filter(|(_, n)| **n == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0;
        while let Some(id) = queue.pop_front() {
            visited += 1;
            for e in self.edges.iter().filter(|e| &e.source == id) {
                if let Some(d) = indegree.get_mut(&e.target) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(&e.target);
                    }
                }
            }
        }
        require(
            visited == self.nodes.len(),
            "Cycles are not supported; remove feedback connections",
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub source: String,
    pub kind: String,
    pub timestamp_ms: u64,
    pub confidence: f64,
    pub strength: f64,
    pub value: f64,
    pub duration_ms: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    pub time_ms: u64,
    pub node: String,
    pub kind: String,
    pub result: String,
}
#[derive(Clone)]
struct Scheduled {
    at: u64,
    order: u64,
    node: String,
    event: Event,
    repeat: u32,
    interval: u64,
    emit: bool,
}
#[derive(Default)]
struct State {
    last: Option<u64>,
    value: f64,
    inputs: BTreeMap<String, bool>,
    generation: u64,
}
pub struct Runtime {
    pub graph: Graph,
    queue: Vec<Scheduled>,
    states: BTreeMap<String, State>,
    order: u64,
    pub traces: VecDeque<Trace>,
    pub dispatched: u64,
}
impl Runtime {
    pub fn new(graph: Graph) -> Result<Self> {
        graph.validate()?;
        Ok(Self {
            graph,
            queue: vec![],
            states: BTreeMap::new(),
            order: 0,
            traces: VecDeque::with_capacity(256),
            dispatched: 0,
        })
    }
    pub fn stop(&mut self) {
        self.queue.clear();
        self.states.clear();
    }
    pub fn trace(&mut self, now: u64, node: &str, kind: &str, result: &str) {
        if self.traces.len() == 256 {
            self.traces.pop_front();
        }
        self.traces.push_back(Trace {
            time_ms: now,
            node: node.into(),
            kind: kind.into(),
            result: result.into(),
        });
    }
    fn schedule(&mut self, mut item: Scheduled) -> Result<()> {
        require(self.queue.len() < 4096, "Graph queue limit exceeded")?;
        self.order += 1;
        item.order = self.order;
        self.queue.push(item);
        Ok(())
    }
    pub fn inject(&mut self, profile: &str, event: Event) -> Result<()> {
        let ids: Vec<_> = self
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == "Muscle Action" && n.text("profile_id") == profile)
            .map(|n| n.id.clone())
            .collect();
        for id in ids {
            self.fanout(&id, &event, event.timestamp_ms)?;
        }
        Ok(())
    }
    fn fanout(&mut self, id: &str, event: &Event, now: u64) -> Result<()> {
        self.trace(now, id, &event.kind, "routed");
        let edges: Vec<_> = self
            .graph
            .edges
            .iter()
            .filter(|e| {
                e.source == id
                    && (e.port == "any"
                        || e.port == event.kind
                        || (e.port == "start" && event.kind == "START")
                        || (e.port == "stop" && event.kind == "END"))
            })
            .cloned()
            .collect();
        for edge in edges {
            let mut event = event.clone();
            event.source = id.into();
            self.schedule(Scheduled {
                at: now,
                order: 0,
                node: edge.target,
                event,
                repeat: 0,
                interval: 0,
                emit: false,
            })?;
        }
        Ok(())
    }
    pub fn tick(
        &mut self,
        now: u64,
        dispatch: &mut dyn FnMut(&Node, &Event) -> Result<()>,
    ) -> Result<()> {
        let mut steps = 0;
        loop {
            let next = self
                .queue
                .iter()
                .enumerate()
                .filter(|(_, e)| e.at <= now)
                .min_by_key(|(_, e)| (e.at, e.order))
                .map(|(i, _)| i);
            let Some(i) = next else { break };
            steps += 1;
            if steps > 512 {
                self.stop();
                return Err(Error::Invalid("Graph event budget exceeded".into()));
            }
            let mut item = self.queue.remove(i);
            if item.emit {
                self.fanout(&item.node, &item.event, now)?;
                if item.repeat > 0 {
                    if item.repeat != u32::MAX {
                        item.repeat -= 1;
                    }
                    item.at = now + item.interval;
                    self.schedule(item)?;
                }
                continue;
            }
            let Some(node) = self.graph.nodes.iter().find(|n| n.id == item.node).cloned() else {
                continue;
            };
            self.dispatched += 1;
            let ms = node.number("ms", 500.) as u64;
            let state = self.states.entry(node.id.clone()).or_default();
            let mut event = item.event.clone();
            let mut output = true;
            match node.kind.as_str() {
                "Delay" | "Timer" | "Hold" => {
                    if node.kind == "Hold" && event.kind == "END" {
                        self.queue.retain(|q| q.node != node.id || !q.emit);
                        output = false;
                    } else {
                        item.at = now + ms;
                        item.emit = true;
                        self.schedule(item)?;
                        output = false;
                    }
                }
                "Interval" => {
                    self.queue.retain(|q| q.node != node.id || !q.emit);
                    if event.kind != "END" && event.kind != "stop" {
                        item.at = now + ms;
                        item.emit = true;
                        item.event.kind = "TRIGGER".into();
                        item.repeat = u32::MAX;
                        item.interval = ms.max(10);
                        self.schedule(item)?;
                    }
                    output = false;
                }
                "Repeat" => {
                    item.at = now;
                    item.emit = true;
                    item.repeat = node.number("count", 2.) as u32 - 1;
                    item.interval = node.number("interval", 100.) as u64;
                    self.schedule(item)?;
                    output = false;
                }
                "Pulse" => {
                    event.kind = "START".into();
                    item.event.kind = "END".into();
                    item.at = now + ms;
                    item.emit = true;
                    self.schedule(item)?;
                }
                "Debounce" | "Cooldown" => {
                    if state.last.is_some_and(|last| now.saturating_sub(last) < ms) {
                        output = false;
                    } else {
                        state.last = Some(now);
                    }
                }
                "Toggle" => {
                    state.value = if state.value == 0. { 1. } else { 0. };
                    event.value = state.value;
                    event.kind = if state.value == 1. { "START" } else { "END" }.into();
                }
                "Latch" => {
                    state.value = if event.kind == "END" || event.kind == "reset" {
                        0.
                    } else {
                        1.
                    };
                    event.value = state.value;
                }
                "Counter" => {
                    state.value = if event.kind == "reset" {
                        0.
                    } else {
                        state.value + 1.
                    };
                    event.value = state.value;
                }
                "AND" | "OR" => {
                    state.inputs.insert(
                        event.source.clone(),
                        event.kind != "END" && event.value != 0.,
                    );
                    let count = self
                        .graph
                        .edges
                        .iter()
                        .filter(|e| e.target == node.id)
                        .map(|e| &e.source)
                        .collect::<BTreeSet<_>>()
                        .len();
                    output = if node.kind == "AND" {
                        state.inputs.len() == count && state.inputs.values().all(|x| *x)
                    } else {
                        state.inputs.values().any(|x| *x)
                    };
                }
                "NOT" => {
                    event.value = if event.value == 0. { 1. } else { 0. };
                }
                "Gate" => {
                    output = node
                        .params
                        .get("open")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                }
                "Compare" | "Branch" => {
                    let v = node.number("value", 1.);
                    let condition = match node.text("operator") {
                        "lt" => event.value < v,
                        "eq" => (event.value - v).abs() < 1e-9,
                        _ => event.value >= v,
                    };
                    event.kind = if condition { "true" } else { "false" }.into();
                    event.value = f64::from(condition);
                }
                "Muscle Action" | "Debug" => {}
                _ => {
                    state.generation += 1;
                    match dispatch(&node, &event) {
                        Ok(()) => self.trace(now, &node.id, &event.kind, "dispatched"),
                        Err(e) => {
                            self.trace(now, &node.id, &event.kind, &e.to_string());
                            tracing::error!(node=%node.id,error=%e,"action failed");
                            return Err(e);
                        }
                    }
                }
            }
            if output {
                self.fanout(&node.id, &event, now)?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn node(id: &str, kind: &str) -> Node {
        Node {
            id: id.into(),
            kind: kind.into(),
            label: id.into(),
            x: 0.,
            y: 0.,
            params: json!({"profile_id":"p","ms":500}),
        }
    }
    fn event() -> Event {
        Event {
            source: "p".into(),
            kind: "TRIGGER".into(),
            timestamp_ms: 0,
            confidence: 1.,
            strength: 1.,
            value: 1.,
            duration_ms: 100.,
        }
    }
    #[test]
    fn fanout_timer_and_cancel() {
        let g = Graph {
            nodes: vec![
                node("a", "Muscle Action"),
                node("b", "Left Click"),
                node("c", "Timer"),
                node("d", "Key Press"),
            ],
            edges: vec![
                Edge {
                    id: "1".into(),
                    source: "a".into(),
                    target: "b".into(),
                    port: "any".into(),
                },
                Edge {
                    id: "2".into(),
                    source: "a".into(),
                    target: "c".into(),
                    port: "any".into(),
                },
                Edge {
                    id: "3".into(),
                    source: "c".into(),
                    target: "d".into(),
                    port: "any".into(),
                },
            ],
        };
        let mut r = Runtime::new(g).expect("graph");
        r.inject("p", event()).expect("inject");
        let mut calls = vec![];
        r.tick(0, &mut |n, _| {
            calls.push(n.kind.clone());
            Ok(())
        })
        .expect("tick");
        assert_eq!(calls, vec!["Left Click"]);
        r.tick(500, &mut |n, _| {
            calls.push(n.kind.clone());
            Ok(())
        })
        .expect("tick");
        assert_eq!(calls, vec!["Left Click", "Key Press"]);
        r.stop();
        assert!(r.queue.is_empty());
    }
    #[test]
    fn interval_continues_until_end_and_preserves_queued_stop() {
        let graph = Graph {
            nodes: vec![
                node("input", "Muscle Action"),
                node("timer", "Interval"),
                node("out", "Debug"),
            ],
            edges: vec![
                Edge {
                    id: "1".into(),
                    source: "input".into(),
                    target: "timer".into(),
                    port: "any".into(),
                },
                Edge {
                    id: "2".into(),
                    source: "timer".into(),
                    target: "out".into(),
                    port: "TRIGGER".into(),
                },
            ],
        };
        let mut runtime = Runtime::new(graph).expect("graph");
        let mut start = event();
        start.kind = "START".into();
        runtime.inject("p", start.clone()).expect("start");
        runtime.tick(0, &mut |_, _| Ok(())).expect("arm");
        for tick in 1..=150 {
            runtime.tick(tick * 500, &mut |_, _| Ok(())).expect("tick");
            assert_eq!(runtime.queue.len(), 1, "one pending interval event");
        }
        assert_eq!(runtime.dispatched, 151, "all 150 trigger outputs delivered");
        start.timestamp_ms = 75000;
        runtime.inject("p", start.clone()).expect("restart");
        start.kind = "END".into();
        runtime.inject("p", start).expect("queued stop");
        runtime.tick(75000, &mut |_, _| Ok(())).expect("stop");
        assert!(
            runtime.queue.is_empty(),
            "restart must not discard a queued END"
        );
        let count = runtime.dispatched;
        runtime.tick(80000, &mut |_, _| Ok(())).expect("idle");
        assert_eq!(runtime.dispatched, count);
    }
    #[test]
    fn cycle_rejected() {
        let g = Graph {
            nodes: vec![node("a", "Timer")],
            edges: vec![Edge {
                id: "e".into(),
                source: "a".into(),
                target: "a".into(),
                port: "any".into(),
            }],
        };
        assert!(g.validate().is_err());
    }
}
