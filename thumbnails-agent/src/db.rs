use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize)]
pub struct Run {
    pub id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub topic: String,
    pub summary: String,
    pub channel_url: String,
    pub assets: serde_json::Value,
    pub concept_count: u8,
    pub model_research: String,
    pub model_critique: String,
    pub model_fix: String,
    pub model_image: String,
    pub generate_images: bool,
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub id: i64,
    pub created_at: i64,
    pub phase: String,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Feedback {
    pub stage: String,
    pub overall: String,
    pub items: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewRun {
    pub topic: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub channel_url: String,
    #[serde(default)]
    pub assets: serde_json::Value,
    #[serde(default = "default_concept_count")]
    pub concept_count: u8,
    #[serde(default)]
    pub model_research: String,
    #[serde(default)]
    pub model_critique: String,
    #[serde(default)]
    pub model_fix: String,
    #[serde(default)]
    pub model_image: String,
    /// When false the run stops after concept approval so the prompts can be
    /// copied out instead of generating images.
    #[serde(default = "default_generate_images")]
    pub generate_images: bool,
}

fn default_concept_count() -> u8 { 3 }
fn default_generate_images() -> bool { true }

pub struct Db {
    path: String,
}

impl Clone for Db {
    fn clone(&self) -> Self {
        Db {
            path: self.path.clone(),
        }
    }
}

impl Db {
    pub fn open(path: &str) -> Result<Db, String> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("cannot create data dir: {e}"))?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.busy_timeout(std::time::Duration::from_secs(10)).ok();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS runs(
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                topic TEXT NOT NULL,
                summary TEXT NOT NULL,
                channel_url TEXT NOT NULL DEFAULT '',
                assets TEXT NOT NULL DEFAULT '[]',
                concept_count INTEGER NOT NULL DEFAULT 3,
                model_research TEXT NOT NULL,
                model_critique TEXT NOT NULL,
                model_fix TEXT NOT NULL,
                model_image TEXT NOT NULL DEFAULT '',
                generate_images INTEGER NOT NULL DEFAULT 1,
                status TEXT NOT NULL,
                result TEXT,
                error TEXT
            );
            CREATE TABLE IF NOT EXISTS messages(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                phase TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_messages_run ON messages(run_id, id);
            CREATE TABLE IF NOT EXISTS feedback(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                stage TEXT NOT NULL DEFAULT 'concepts',
                overall TEXT NOT NULL DEFAULT '',
                items TEXT NOT NULL DEFAULT '{}'
            );",
        )
        .map_err(|e| e.to_string())?;
        // Databases created before image generation became optional lack the
        // column; existing runs always generated images.
        let _ = conn.execute(
            "ALTER TABLE runs ADD COLUMN generate_images INTEGER NOT NULL DEFAULT 1",
            [],
        );
        drop(conn);
        Ok(Db {
            path: path.to_string(),
        })
    }

    fn conn(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.path).map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_secs(10)).ok();
        Ok(conn)
    }

    pub fn create_run(&self, r: &NewRun, id: &str) -> Result<(), String> {
        let t = now();
        self.conn()?
            .execute(
                "INSERT INTO runs(id, created_at, updated_at, topic, summary, channel_url, assets, concept_count,
                               model_research, model_critique, model_fix, model_image, generate_images, status)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'queued')",
                params![
                    id,
                    t,
                    t,
                    r.topic,
                    r.summary,
                    r.channel_url,
                    r.assets.to_string(),
                    r.concept_count.clamp(1, 12),
                    r.model_research,
                    r.model_critique,
                    r.model_fix,
                    r.model_image,
                    r.generate_images as i64
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn row_to_run(row: &rusqlite::Row) -> rusqlite::Result<Run> {
        Ok(Run {
            id: row.get(0)?,
            created_at: row.get(1)?,
            updated_at: row.get(2)?,
            topic: row.get(3)?,
            summary: row.get(4)?,
            channel_url: row.get(5)?,
            assets: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(serde_json::json!([])),
            concept_count: row.get(7)?,
            model_research: row.get(8)?,
            model_critique: row.get(9)?,
            model_fix: row.get(10)?,
            model_image: row.get(11)?,
            generate_images: row.get(12)?,
            status: row.get(13)?,
            result: row.get(14)?,
            error: row.get(15)?,
        })
    }

    const RUN_COLS: &'static str =
        "id, created_at, updated_at, topic, summary, channel_url, assets, concept_count, model_research, model_critique, model_fix, model_image, generate_images, status, result, error";

    pub fn get_run(&self, id: &str) -> Result<Option<Run>, String> {
        let sql = format!("SELECT {} FROM runs WHERE id = ?1", Self::RUN_COLS);
        self.conn()?
            .query_row(&sql, params![id], Self::row_to_run)
            .optional_run()
    }

    pub fn list_runs(&self) -> Result<Vec<Run>, String> {
        let sql = format!(
            "SELECT {} FROM runs ORDER BY created_at DESC, id DESC",
            Self::RUN_COLS
        );
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], Self::row_to_run)
            .map_err(|e| e.to_string())?;
        let mut v = Vec::new();
        for r in rows {
            v.push(r.map_err(|e| e.to_string())?);
        }
        Ok(v)
    }

    pub fn delete_run(&self, id: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM messages WHERE run_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM feedback WHERE run_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM runs WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn cas_status(&self, id: &str, from: &str, to: &str) -> bool {
        let n = self.conn().ok().and_then(|c| {
            c.execute(
                "UPDATE runs SET status = ?2, updated_at = ?3 WHERE id = ?1 AND status = ?4",
                params![id, to, now(), from],
            )
            .ok()
        });
        n.unwrap_or(0) == 1
    }

    pub fn set_status(&self, id: &str, status: &str) {
        let _ = self.conn().map(|c| {
            c.execute(
                "UPDATE runs SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, status, now()],
            )
        });
    }

    pub fn set_result(&self, id: &str, result: &str) {
        let _ = self.conn().map(|c| {
            c.execute(
                "UPDATE runs SET result = ?2, error = NULL, updated_at = ?3 WHERE id = ?1",
                params![id, result, now()],
            )
        });
    }

    pub fn fail_run(&self, id: &str, error: &str) {
        let _ = self.conn().map(|c| {
            c.execute(
                "UPDATE runs SET status = 'error', error = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, error, now()],
            )
        });
    }

    fn run_exists(&self, id: &str) -> bool {
        self.conn()
            .ok()
            .and_then(|c| {
                c.query_row("SELECT 1 FROM runs WHERE id = ?1", params![id], |_| Ok(()))
                    .ok()
            })
            .is_some()
    }

    pub fn append_message(&self, run_id: &str, phase: &str, role: &str, content: &str) {
        if !self.run_exists(run_id) {
            return;
        }
        let _ = self.conn().map(|c| {
            c.execute(
                "INSERT INTO messages(run_id, created_at, phase, role, content) VALUES (?1,?2,?3,?4,?5)",
                params![run_id, now(), phase, role, content],
            )
        });
    }

    pub fn list_messages(&self, run_id: &str) -> Result<Vec<Message>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, created_at, phase, role, content FROM messages WHERE run_id = ?1 ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![run_id], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    phase: row.get(2)?,
                    role: row.get(3)?,
                    content: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut v = Vec::new();
        for r in rows {
            v.push(r.map_err(|e| e.to_string())?);
        }
        Ok(v)
    }

    pub fn save_feedback(&self, run_id: &str, stage: &str, overall: &str, items: &serde_json::Value) {
        let _ = self.conn().map(|c| {
            c.execute(
                "INSERT INTO feedback(run_id, created_at, stage, overall, items) VALUES (?1,?2,?3,?4,?5)",
                params![run_id, now(), stage, overall, items.to_string()],
            )
        });
    }

    pub fn latest_feedback(&self, run_id: &str) -> Option<Feedback> {
        let conn = self.conn().ok()?;
        conn.query_row(
            "SELECT stage, overall, items FROM feedback WHERE run_id = ?1 ORDER BY id DESC LIMIT 1",
            params![run_id],
            |row| {
                Ok(Feedback {
                    stage: row.get(0)?,
                    overall: row.get(1)?,
                    items: serde_json::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or(serde_json::json!({})),
                })
            },
        )
        .ok()
    }
}

trait OptionalRow<T> {
    fn optional_run(self) -> Result<Option<T>, String>;
}
impl<T> OptionalRow<T> for Result<T, rusqlite::Error> {
    fn optional_run(self) -> Result<Option<T>, String> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}
