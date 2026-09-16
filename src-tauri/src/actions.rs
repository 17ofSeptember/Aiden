use crate::{
    graph::{Event, Node},
    require, Error, Result,
};
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::{
    collections::BTreeSet,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
pub trait InputController {
    fn dispatch(&mut self, node: &Node, event: &Event) -> Result<()>;
    fn release_all(&mut self) -> Result<()>;
}
pub fn is_system(kind: &str) -> bool {
    matches!(
        kind,
        "Launch Application" | "Approved Command" | "Open File" | "Open Folder" | "Open URL"
    )
}
pub fn is_output(kind: &str) -> bool {
    is_system(kind)
        || matches!(
            kind,
            "Left Click"
                | "Right Click"
                | "Middle Click"
                | "Mouse Down"
                | "Mouse Up"
                | "Move Mouse"
                | "Scroll Vertical"
                | "Scroll Horizontal"
                | "Key Down"
                | "Key Up"
                | "Key Press"
                | "Shortcut"
                | "Type Text"
                | "Play/Pause"
                | "Next Track"
                | "Previous Track"
                | "Volume Up"
                | "Volume Down"
                | "Mute"
        )
}
pub fn validate_ready(node: &Node) -> Result<()> {
    if is_system(&node.kind) {
        validate_system(node)?;
        require(
            !node.text("target").is_empty(),
            "Set and save a target first",
        )?;
        let target = Path::new(node.text("target"));
        match node.kind.as_str() {
            "Launch Application" | "Approved Command" | "Open File" => require(
                target.is_file(),
                "Target file does not exist; enter one absolute path",
            )?,
            "Open Folder" => require(target.is_dir(), "Target folder does not exist")?,
            _ => {}
        }
    } else if matches!(node.kind.as_str(), "Key Down" | "Key Up" | "Key Press") {
        parse_key(node.text("key"))?;
    } else if node.kind == "Shortcut" {
        let keys: Vec<_> = node.text("key").split('+').map(str::trim).collect();
        require(keys.len() <= 8, "Shortcut has too many keys")?;
        for key in keys {
            parse_key(key)?;
        }
    } else if matches!(node.kind.as_str(), "Mouse Down" | "Mouse Up") {
        button(node.text("button"))?;
    }
    Ok(())
}
pub fn capability() -> String {
    if cfg!(target_os = "windows") {
        "Windows SendInput (same/lower integrity windows)".into()
    } else if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        "Unavailable: Wayland global injection is blocked; log into an X11 session".into()
    } else if std::env::var_os("DISPLAY").is_some() {
        "X11 XTest (connection checked when armed)".into()
    } else {
        "Unavailable: no X11 display".into()
    }
}
pub fn validate_system(n: &Node) -> Result<()> {
    let target = n.text("target");
    if target.is_empty() {
        return Ok(());
    }
    require(
        target.len() <= 4096 && !target.contains('\0'),
        "Invalid system target",
    )?;
    if n.kind == "Open URL" {
        require(
            target.starts_with("https://") || target.starts_with("http://"),
            "Only http/https URLs are allowed",
        )?;
    } else {
        require(
            Path::new(target).is_absolute(),
            "System targets must use absolute paths",
        )?;
    }
    if let Some(args) = n.params.get("args") {
        let args = args
            .as_array()
            .ok_or_else(|| Error::Invalid("Arguments must be an array".into()))?;
        require(
            args.len() <= 64
                && args.iter().all(|a| {
                    a.as_str()
                        .is_some_and(|s| s.len() <= 4096 && !s.contains('\0'))
                }),
            "Invalid command arguments",
        )?;
    }
    Ok(())
}
pub struct NativeInput {
    enigo: Enigo,
    keys: BTreeSet<String>,
    buttons: BTreeSet<String>,
    approvals: BTreeSet<String>,
    children: Vec<Child>,
    pending: Vec<(Instant, String)>,
    last_action: Instant,
    burst: u32,
}
impl NativeInput {
    pub fn new() -> Result<Self> {
        require(!capability().starts_with("Unavailable"), &capability())?;
        let enigo = Enigo::new(&Settings::default()).map_err(|e| Error::Input(e.to_string()))?;
        Ok(Self {
            enigo,
            keys: BTreeSet::new(),
            buttons: BTreeSet::new(),
            approvals: BTreeSet::new(),
            children: vec![],
            pending: vec![],
            last_action: Instant::now(),
            burst: 0,
        })
    }
    pub fn approve(&mut self, n: &Node) -> Result<()> {
        require(is_system(&n.kind), "Only system targets require approval")?;
        validate_ready(n)?;
        require(
            !n.text("target").is_empty(),
            "Choose an executable or target first",
        )?;
        self.approvals.insert(approval_key(n)?);
        Ok(())
    }
    pub fn check_approval(&self, node: &Node) -> Result<()> {
        require(
            !is_system(&node.kind) || self.approvals.contains(&approval_key(node)?),
            "Review and approve this saved target in Node settings before enabling control",
        )
    }
    fn key(&mut self, name: &str, direction: Direction) -> Result<()> {
        let key = parse_key(name)?;
        self.enigo
            .key(key, direction)
            .map_err(|e| Error::Input(e.to_string()))?;
        match direction {
            Direction::Press => {
                self.keys.insert(name.into());
            }
            Direction::Release => {
                self.keys.remove(name);
            }
            _ => {}
        }
        Ok(())
    }
    pub fn housekeeping(&mut self) -> Result<()> {
        let now = Instant::now();
        let due: Vec<_> = self
            .pending
            .iter()
            .filter(|(at, _)| *at <= now)
            .map(|(_, k)| k.clone())
            .collect();
        self.pending.retain(|(at, _)| *at > now);
        for key in due {
            self.key(&key, Direction::Release)?;
        }
        let mut i = 0;
        while i < self.children.len() {
            if let Some(status) = self.children[i].try_wait()? {
                if !status.success() {
                    tracing::warn!(code=?status.code(),"launched process exited with failure");
                }
                self.children.remove(i).wait()?;
            } else {
                i += 1;
            }
        }
        Ok(())
    }
}
impl InputController for NativeInput {
    fn dispatch(&mut self, n: &Node, e: &Event) -> Result<()> {
        self.housekeeping()?;
        if self.last_action.elapsed() > Duration::from_secs(1) {
            self.burst = 0;
            self.last_action = Instant::now();
        }
        self.burst += 1;
        require(self.burst <= 120, "Native action rate limit exceeded")?;
        match n.kind.as_str() {
            "Left Click" | "Right Click" | "Middle Click" | "Mouse Down" | "Mouse Up" => {
                let name = match n.kind.as_str() {
                    "Left Click" => "left",
                    "Right Click" => "right",
                    "Middle Click" => "middle",
                    _ => n.text("button"),
                };
                let b = button(name)?;
                let d = match n.kind.as_str() {
                    "Mouse Down" => Direction::Press,
                    "Mouse Up" => Direction::Release,
                    _ => Direction::Click,
                };
                self.enigo
                    .button(b, d)
                    .map_err(|e| Error::Input(e.to_string()))?;
                if d == Direction::Press {
                    self.buttons.insert(name.into());
                }
                if d == Direction::Release {
                    self.buttons.remove(name);
                }
            }
            "Move Mouse" => {
                let strength = if n.text("mode") == "strength" {
                    e.strength.clamp(0., 1.)
                } else {
                    1.
                };
                if strength >= n.number("dead_zone", 0.05) {
                    let speed = n
                        .number("speed", 10.)
                        .min(n.number("max_speed", 100.))
                        .clamp(0., 1000.);
                    let x = (n.number("x", 1.).clamp(-1., 1.) * speed * strength) as i32;
                    let y = (n.number("y", 0.).clamp(-1., 1.) * speed * strength) as i32;
                    self.enigo
                        .move_mouse(x, y, Coordinate::Rel)
                        .map_err(|e| Error::Input(e.to_string()))?;
                }
            }
            "Scroll Vertical" | "Scroll Horizontal" => self
                .enigo
                .scroll(
                    n.number("value", 1.).clamp(-100., 100.) as i32,
                    if n.kind == "Scroll Vertical" {
                        Axis::Vertical
                    } else {
                        Axis::Horizontal
                    },
                )
                .map_err(|e| Error::Input(e.to_string()))?,
            "Key Down" => self.key(n.text("key"), Direction::Press)?,
            "Key Up" => self.key(n.text("key"), Direction::Release)?,
            "Key Press" => {
                let key = n.text("key");
                self.key(key, Direction::Press)?;
                require(self.pending.len() < 256, "Too many pending key releases")?;
                self.pending.push((
                    Instant::now() + Duration::from_millis(n.number("ms", 30.) as u64),
                    key.into(),
                ));
            }
            "Shortcut" => {
                let keys: Vec<_> = n.text("key").split('+').map(str::trim).collect();
                require(keys.len() <= 8, "Shortcut has too many keys")?;
                for k in &keys {
                    parse_key(k)?;
                }
                for k in &keys {
                    self.key(k, Direction::Press)?;
                }
                for k in keys.iter().rev() {
                    self.key(k, Direction::Release)?;
                }
            }
            "Type Text" => {
                require(
                    n.text("text").len() <= 4096,
                    "Text is limited to 4096 bytes",
                )?;
                self.enigo
                    .text(n.text("text"))
                    .map_err(|e| Error::Input(e.to_string()))?;
            }
            "Play/Pause" | "Next Track" | "Previous Track" | "Volume Up" | "Volume Down"
            | "Mute" => self.key(&n.kind, Direction::Click)?,
            "Launch Application" | "Approved Command" | "Open File" | "Open Folder"
            | "Open URL" => {
                validate_system(n)?;
                require(self.approvals.contains(&approval_key(n)?),"This exact target and arguments require approval in the inspector for this session")?;
                if n.kind == "Approved Command" || n.kind == "Launch Application" {
                    require(
                        Path::new(n.text("target")).is_file(),
                        "Executable does not exist",
                    )?;
                    require(
                        self.children.len() < 8,
                        "Concurrent application launch limit reached",
                    )?;
                    let args: Vec<_> = n
                        .params
                        .get("args")
                        .and_then(|a| a.as_array())
                        .map(|a| a.iter().filter_map(|s| s.as_str()).collect())
                        .unwrap_or_default();
                    let child = Command::new(n.text("target"))
                        .args(args)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()?;
                    self.children.push(child);
                } else {
                    open::that_detached(n.text("target"))?;
                }
            }
            _ => return Err(Error::Input(format!("Unsupported action {}", n.kind))),
        }
        tracing::info!(node=%n.id, kind=%n.kind, "Native action dispatched");
        Ok(())
    }
    fn release_all(&mut self) -> Result<()> {
        self.pending.clear();
        let mut errors = vec![];
        for name in self.keys.clone() {
            if let Err(e) = self.key(&name, Direction::Release) {
                errors.push(e.to_string());
            }
        }
        for name in self.buttons.clone() {
            match button(&name).and_then(|b| {
                self.enigo
                    .button(b, Direction::Release)
                    .map_err(|e| Error::Input(e.to_string()))
            }) {
                Ok(()) => {
                    self.buttons.remove(&name);
                }
                Err(e) => errors.push(e.to_string()),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Input(errors.join("; ")))
        }
    }
}
impl Drop for NativeInput {
    fn drop(&mut self) {
        if let Err(e) = self.release_all() {
            tracing::error!(error=%e,"input cleanup failed");
        }
    }
}
fn approval_key(node: &Node) -> Result<String> {
    Ok(serde_json::to_string(&(&node.kind, &node.params))?)
}

fn button(s: &str) -> Result<Button> {
    match s {
        "left" => Ok(Button::Left),
        "right" => Ok(Button::Right),
        "middle" => Ok(Button::Middle),
        _ => Err(Error::Invalid(
            "Choose left, right or middle mouse button".into(),
        )),
    }
}
pub fn parse_key(s: &str) -> Result<Key> {
    let key = match s.to_lowercase().as_str() {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" => Key::Alt,
        "meta" | "win" => Key::Meta,
        "space" => Key::Space,
        "enter" => Key::Return,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        "play/pause" => Key::MediaPlayPause,
        "next track" => Key::MediaNextTrack,
        "previous track" => Key::MediaPrevTrack,
        "volume up" => Key::VolumeUp,
        "volume down" => Key::VolumeDown,
        "mute" => Key::VolumeMute,
        _ => {
            let mut chars = s.chars();
            let c = chars
                .next()
                .ok_or_else(|| Error::Invalid("Key is empty".into()))?;
            require(chars.next().is_none(), "Unsupported key name")?;
            Key::Unicode(c)
        }
    };
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    #[test]
    fn native_dispatch_launches_only_an_approved_process() {
        // Execute one harmless test in a child process, with no mouse/keyboard
        // injection, browser navigation or interaction with the user's windows.
        let executable = std::env::current_exe().expect("test executable");
        let node = Node {
            id: "native-launch-test".into(),
            kind: "Approved Command".into(),
            label: "Native process test".into(),
            x: 0.,
            y: 0.,
            params: serde_json::json!({"target":executable, "args":["--exact", "actions::tests::approval_is_bound_to_operation_and_arguments"]}),
        };
        let event = Event {
            source: "test".into(),
            kind: "TRIGGER".into(),
            timestamp_ms: 0,
            confidence: 1.,
            strength: 1.,
            value: 1.,
            duration_ms: 100.,
        };
        let mut input = NativeInput::new().expect("Windows adapter");
        assert!(input.dispatch(&node, &event).is_err());
        assert!(
            input.children.is_empty(),
            "unapproved targets cannot launch"
        );
        input.approve(&node).expect("approve exact test target");
        input.dispatch(&node, &event).expect("native launch");
        let child = input.children.first_mut().expect("spawned process");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().expect("child status") {
                assert!(status.success(), "child process must execute successfully");
                break;
            }
            if Instant::now() >= deadline {
                child.kill().expect("stop timed-out child");
                child.wait().expect("reap child");
                panic!("native launch test timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        input.release_all().expect("cleanup");
    }
    #[test]
    fn launch_preflight_rejects_missing_targets_before_control_is_armed() {
        let mut node = Node {
            id: "launch".into(),
            kind: "Launch Application".into(),
            label: "Launch".into(),
            x: 0.,
            y: 0.,
            params: serde_json::json!({}),
        };
        assert!(validate_ready(&node).is_err());
        let dir = tempfile::tempdir().expect("directory");
        let target = dir.path().join("application");
        node.params = serde_json::json!({"target":target.to_str().expect("path"), "args":[]});
        assert!(validate_ready(&node).is_err());
        std::fs::write(&target, []).expect("fixture file");
        validate_ready(&node).expect("existing file");
        node.params["target"] =
            serde_json::json!(format!("{}{}", target.display(), target.display()));
        assert!(
            validate_ready(&node).is_err(),
            "concatenated paths are not executable targets"
        );
    }
    #[test]
    fn approval_is_bound_to_operation_and_arguments() {
        let node = Node {
            id: "one".into(),
            kind: "Open File".into(),
            label: "target".into(),
            x: 0.,
            y: 0.,
            params: serde_json::json!({"target":"/example", "args":[]}),
        };
        let mut changed = node.clone();
        changed.kind = "Approved Command".into();
        assert_ne!(
            approval_key(&node).expect("key"),
            approval_key(&changed).expect("key")
        );
        changed = node.clone();
        changed.params["args"] = serde_json::json!(["--changed"]);
        assert_ne!(
            approval_key(&node).expect("key"),
            approval_key(&changed).expect("key")
        );
    }
}
