use crate::{
    graph::Graph,
    model::{DspConfig, Profile, Sample},
    require, Result,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub version: u32,
    pub graph: Graph,
    pub profiles: Vec<Profile>,
    pub dsp: DspConfig,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            version: 1,
            graph: Graph::default(),
            profiles: vec![],
            dsp: DspConfig::default(),
        }
    }
}
impl Workspace {
    pub fn validate(&self) -> Result<()> {
        require(
            self.version == 1 && self.profiles.len() <= 32,
            "Unsupported workspace version or profile limit exceeded",
        )?;
        self.graph.validate()?;
        self.dsp.validate()?;
        let mut ids = std::collections::BTreeSet::new();
        for p in &self.profiles {
            p.validate()?;
            require(ids.insert(&p.id), "Duplicate profile IDs")?;
            require(
                p.dsp == self.dsp,
                "Profiles must use the workspace DSP configuration; retrain after filter changes",
            )?;
        }
        for n in &self.graph.nodes {
            if n.kind == "Muscle Action" {
                require(
                    self.profiles.iter().any(|p| p.id == n.text("profile_id")),
                    "Action node references a missing profile",
                )?;
            }
        }
        Ok(())
    }
}
pub struct Storage {
    connection: Connection,
    pub path: PathBuf,
    pub recovered: bool,
}
impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='schema_version')",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            connection.execute_batch(include_str!("../migrations/001_initial.sql"))?;
        }
        let version: u32 =
            connection.query_row("SELECT max(version) FROM schema_version", [], |r| r.get(0))?;
        require(
            version == 1,
            "Database schema is newer than this application",
        )?;
        let check: String = connection.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        require(
            check == "ok",
            "Database failed integrity check; restore a backup without overwriting this file",
        )?;
        let clean: bool =
            connection.query_row("SELECT clean_shutdown FROM recovery WHERE id=1", [], |r| {
                r.get(0)
            })?;
        connection.execute("UPDATE recovery SET clean_shutdown=0 WHERE id=1", [])?;
        Ok(Self {
            connection,
            path: path.into(),
            recovered: !clean,
        })
    }
    pub fn load(&self) -> Result<Workspace> {
        let mut graph = Graph::default();
        for json in self.json_rows("SELECT definition FROM nodes ORDER BY rowid")? {
            graph.nodes.push(serde_json::from_str(&json)?);
        }
        for json in self.json_rows("SELECT definition FROM edges ORDER BY rowid")? {
            graph.edges.push(serde_json::from_str(&json)?);
        }
        let mut profiles = vec![];
        for json in self.json_rows("SELECT metadata FROM action_profiles ORDER BY rowid")? {
            let mut p: Profile = serde_json::from_str(&json)?;
            let mut statement = self
                .connection
                .prepare("SELECT data FROM training_examples WHERE profile_id=? ORDER BY rowid")?;
            for row in statement.query_map([&p.id], |r| r.get::<_, String>(0))? {
                p.examples.push(serde_json::from_str(&row?)?);
            }
            profiles.push(p);
        }
        let dsp = self
            .setting("dsp")?
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        let workspace = Workspace {
            version: 1,
            graph,
            profiles,
            dsp,
        };
        workspace.validate()?;
        Ok(workspace)
    }
    fn json_rows(&self, sql: &str) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
    pub fn save(&mut self, w: &Workspace) -> Result<()> {
        w.validate()?;
        let tx = self.connection.transaction()?;
        tx.execute_batch("DELETE FROM edges; DELETE FROM nodes; DELETE FROM training_examples; DELETE FROM action_profiles;")?;
        for p in &w.profiles {
            let mut metadata = p.clone();
            metadata.examples.clear();
            tx.execute(
                "INSERT INTO action_profiles VALUES(?1,?2)",
                params![p.id, serde_json::to_string(&metadata)?],
            )?;
            for e in &p.examples {
                tx.execute(
                    "INSERT INTO training_examples VALUES(?1,?2,?3,?4,?5)",
                    params![e.id, p.id, e.variant, e.negative, serde_json::to_string(e)?],
                )?;
            }
        }
        for n in &w.graph.nodes {
            tx.execute(
                "INSERT INTO nodes VALUES(?1,'default',?2)",
                params![n.id, serde_json::to_string(n)?],
            )?;
        }
        for e in &w.graph.edges {
            tx.execute(
                "INSERT INTO edges VALUES(?1,'default',?2,?3,?4)",
                params![e.id, e.source, e.target, serde_json::to_string(e)?],
            )?;
        }
        tx.execute(
            "INSERT OR REPLACE INTO settings VALUES('dsp',?1)",
            [serde_json::to_string(&w.dsp)?],
        )?;
        tx.execute(
            "UPDATE workspaces SET updated_at=unixepoch() WHERE id='default'",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn save_positions(&mut self, positions: &[(String, f64, f64)]) -> Result<()> {
        require(positions.len() <= 256, "Too many node positions")?;
        let tx = self.connection.transaction()?;
        {
            let mut update = tx.prepare(
                "UPDATE nodes SET definition=json_set(definition,'$.x',?1,'$.y',?2) WHERE id=?3",
            )?;
            for (id, x, y) in positions {
                require(x.is_finite() && y.is_finite(), "Invalid node position")?;
                require(
                    update.execute(params![x, y, id])? == 1,
                    "Moved node no longer exists",
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn setting(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self
            .connection
            .query_row("SELECT value FROM settings WHERE key=?", [key], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(raw.map(|s| serde_json::from_str(&s)).transpose()?)
    }
    pub fn set_setting(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO settings VALUES(?1,?2)",
            params![key, serde_json::to_string(value)?],
        )?;
        Ok(())
    }
    pub fn backup(&self) -> Result<PathBuf> {
        let path = self
            .path
            .with_extension(format!("{}.backup", crate::model::epoch_ms()));
        self.connection.backup("main", &path, None)?;
        Ok(path)
    }
    pub fn record(&self, name: &str, rate: u32, samples: &[Sample]) -> Result<String> {
        require(
            !samples.is_empty() && samples.len() <= 600000,
            "Recording must contain 1â€“600000 samples",
        )?;
        let id = uuid::Uuid::new_v4().to_string();
        self.connection.execute(
            "INSERT INTO recordings VALUES(?1,?2,?3,?4,?5)",
            params![
                id,
                name,
                rate,
                crate::model::epoch_ms(),
                serde_json::to_string(samples)?
            ],
        )?;
        Ok(id)
    }
    pub fn recordings(&self) -> Result<Vec<serde_json::Value>> {
        let mut statement=self.connection.prepare("SELECT id,name,sample_rate,created_at,json_array_length(samples) FROM recordings ORDER BY created_at DESC")?;
        let rows=statement.query_map([],|r|Ok(serde_json::json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"rate":r.get::<_,u32>(2)?,"created":r.get::<_,u64>(3)?,"count":r.get::<_,u64>(4)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(rows)
    }
    pub fn recording(&self, id: &str) -> Result<(u32, Vec<Sample>)> {
        let (rate, json): (u32, String) = self.connection.query_row(
            "SELECT sample_rate,samples FROM recordings WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok((rate, serde_json::from_str(&json)?))
    }
    pub fn clean_shutdown(&self) -> Result<()> {
        self.connection
            .execute("UPDATE recovery SET clean_shutdown=1 WHERE id=1", [])?;
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistence_backup_and_recovery() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("test.db");
        {
            let mut s = Storage::open(&path).expect("open");
            assert!(!s.recovered);
            s.save(&Workspace::default()).expect("save");
            assert!(s.backup().expect("backup").exists());
        }
        {
            let s = Storage::open(&path).expect("reopen");
            assert!(s.recovered);
            assert_eq!(s.load().expect("load").version, 1);
            s.clean_shutdown().expect("close");
        }
        assert!(!Storage::open(&path).expect("open").recovered);
    }
}
