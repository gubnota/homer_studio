use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

use super::project_store::CommandError;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub status: String,
    pub progress: u8,
    pub message: Option<String>,
    pub created_at_ms: u64,
    pub events: Vec<JobEvent>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub at_ms: u64,
    pub status: String,
    pub progress: u8,
    pub message: String,
}

#[derive(Clone)]
pub struct JobControl {
    pub cancelled: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl JobControl {
    pub fn preview() -> Self {
        Self { cancelled: Arc::new(AtomicBool::new(false)), paused: Arc::new(AtomicBool::new(false)) }
    }
    pub fn boundary(&self) -> Result<(), CommandError> {
        while self.paused.load(Ordering::SeqCst) && !self.cancelled.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if self.cancelled.load(Ordering::SeqCst) {
            Err(CommandError::new(
                "JOB_CANCELLED",
                "The operation was cancelled.",
            ))
        } else {
            Ok(())
        }
    }
}

struct ActiveControl {
    cancelled: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

pub struct JobStore {
    records: Arc<Mutex<Vec<JobRecord>>>,
    controls: Arc<Mutex<HashMap<String, ActiveControl>>>,
    worker: Arc<Mutex<()>>,
}

impl JobStore {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
            controls: Arc::new(Mutex::new(HashMap::new())),
            worker: Arc::new(Mutex::new(())),
        }
    }

    pub fn list(&self) -> Vec<JobRecord> {
        self.records
            .lock()
            .map(|records| records.iter().rev().cloned().collect())
            .unwrap_or_default()
    }

    pub fn enqueue<F>(&self, kind: &str, label: String, task: F) -> String
    where
        F: FnOnce(JobControl, Box<dyn Fn(u8) + Send>) -> Result<(), CommandError> + Send + 'static,
    {
        self.enqueue_with_report(kind, label, move |control, progress, _report| task(control, progress))
    }

    pub fn enqueue_with_report<F>(&self, kind: &str, label: String, task: F) -> String
    where
        F: FnOnce(JobControl, Box<dyn Fn(u8) + Send>, Box<dyn Fn(String) + Send>) -> Result<(), CommandError> + Send + 'static,
    {
        let id = Uuid::new_v4().to_string();
        let cancelled = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        self.records.lock().unwrap().push(JobRecord {
            id: id.clone(),
            kind: kind.into(),
            label,
            status: "queued".into(),
            progress: 0,
            message: None,
            created_at_ms: now_ms(),
            events: vec![JobEvent { at_ms: now_ms(), status: "queued".into(), progress: 0, message: "Waiting for the local worker".into() }],
        });
        self.controls.lock().unwrap().insert(
            id.clone(),
            ActiveControl {
                cancelled: cancelled.clone(),
                paused: paused.clone(),
            },
        );
        let records = self.records.clone();
        let controls = self.controls.clone();
        let worker = self.worker.clone();
        let task_id = id.clone();
        std::thread::spawn(move || {
            let _worker = worker.lock().unwrap();
            update(&records, &task_id, "running", 0, None);
            let progress_records = records.clone();
            let progress_id = task_id.clone();
            let progress = Box::new(move |value: u8| {
                update(
                    &progress_records,
                    &progress_id,
                    "running",
                    value.min(100),
                    None,
                )
            });
            let report_records = records.clone();
            let report_id = task_id.clone();
            let report = Box::new(move |message: String| {
                if let Ok(mut records) = report_records.lock() {
                    if let Some(record) = records.iter_mut().find(|record| record.id == report_id) {
                        record.message = Some(message.clone());
                        record.events.push(JobEvent { at_ms: now_ms(), status: record.status.clone(), progress: record.progress, message });
                        if record.events.len() > 100 { record.events.remove(0); }
                    }
                }
            });
            let result = task(JobControl { cancelled, paused }, progress, report);
            match result {
                Ok(()) => update(&records, &task_id, "completed", 100, None),
                Err(error) if error.code == "JOB_CANCELLED" => {
                    update(&records, &task_id, "cancelled", 0, Some(error.message))
                }
                Err(error) => update(&records, &task_id, "failed", 0, Some(error.message)),
            }
            controls.lock().unwrap().remove(&task_id);
        });
        id
    }

    pub fn control(&self, id: &str, action: &str) -> Result<(), CommandError> {
        let controls = self
            .controls
            .lock()
            .map_err(|_| CommandError::internal("job controls are unavailable"))?;
        let control = controls
            .get(id)
            .ok_or_else(|| CommandError::new("JOB_NOT_ACTIVE", "That job is no longer active."))?;
        match action {
            "cancel" => control.cancelled.store(true, Ordering::SeqCst),
            "pause" => control.paused.store(true, Ordering::SeqCst),
            "resume" => control.paused.store(false, Ordering::SeqCst),
            _ => {
                return Err(CommandError::new(
                    "INVALID_JOB_ACTION",
                    "Use pause, resume, or cancel.",
                ));
            }
        }
        Ok(())
    }

    pub fn dismiss(&self, ids: &[String]) -> Result<usize, CommandError> {
        let mut records = self.records.lock().map_err(|_| CommandError::internal("job records are unavailable"))?;
        if ids.iter().any(|id| records.iter().any(|record| &record.id == id && !matches!(record.status.as_str(), "completed" | "failed" | "cancelled"))) {
            return Err(CommandError::new("JOB_ACTIVE", "Cancel active jobs and wait for them to finish before cleaning the queue."));
        }
        let before = records.len();
        records.retain(|record| !ids.contains(&record.id));
        Ok(before - records.len())
    }
}

fn update(
    records: &Mutex<Vec<JobRecord>>,
    id: &str,
    status: &str,
    progress: u8,
    message: Option<String>,
) {
    if let Ok(mut records) = records.lock() {
        if let Some(record) = records.iter_mut().find(|record| record.id == id) {
            record.status = status.into();
            record.progress = progress;
            if let Some(message) = message { record.message = Some(message); }
            let event_message = record.message.clone().unwrap_or_else(|| match status {
                "running" => "Running locally".into(),
                "completed" => "Finished".into(),
                "cancelled" => "Cancelled".into(),
                _ => "Queued".into(),
            });
            if record.events.last().is_none_or(|last| last.status != status || last.progress != progress || last.message != event_message) {
                record.events.push(JobEvent { at_ms: now_ms(), status: status.into(), progress, message: event_message });
                if record.events.len() > 100 { record.events.remove(0); }
            }
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pauses_at_boundaries_and_cancels() {
        let store = JobStore::new();
        let id = store.enqueue("test", "Boundary test".into(), |control, progress| {
            for step in 1..=20 {
                control.boundary()?;
                progress(step * 5);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(())
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        store.control(&id, "pause").unwrap();
        let before = store.list()[0].progress;
        std::thread::sleep(std::time::Duration::from_millis(70));
        assert!(store.list()[0].progress <= before + 5);
        store.control(&id, "resume").unwrap();
        store.control(&id, "cancel").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(80));
        assert_eq!(store.list()[0].status, "cancelled");
    }

    #[test]
    fn reports_chapter_progress_in_order() {
        let store = JobStore::new();
        let id = store.enqueue_with_report("speech", "Batch".into(), |_control, progress, report| {
            report("Chapter 1 of 2: One".into());
            progress(50);
            report("Completed chapter 1 of 2: One".into());
            report("Chapter 2 of 2: Two".into());
            progress(99);
            Ok(())
        });
        for _ in 0..100 {
            if store.list()[0].status == "completed" { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let record = store.list().into_iter().find(|item| item.id == id).unwrap();
        assert_eq!(record.status, "completed");
        assert_eq!(record.progress, 100);
        let messages: Vec<&str> = record.events.iter().map(|event| event.message.as_str()).collect();
        assert!(messages.iter().position(|message| *message == "Chapter 1 of 2: One") < messages.iter().position(|message| *message == "Chapter 2 of 2: Two"));
    }

    #[test]
    fn batch_failure_is_visible_and_events_stay_bounded() {
        let store = JobStore::new();
        let id = store.enqueue_with_report("speech", "Batch".into(), |_control, _progress, report| {
            for number in 0..105 { report(format!("Chapter update {number}")); }
            Err(CommandError::new("SPEECH_FAILED", "Chapter 2 of 3 (Two): Worker stopped"))
        });
        for _ in 0..100 {
            if store.list()[0].status == "failed" { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let record = store.list().into_iter().find(|item| item.id == id).unwrap();
        assert_eq!(record.status, "failed");
        assert!(record.message.unwrap().contains("Chapter 2 of 3 (Two)"));
        assert_eq!(record.events.len(), 100);
        assert!(record.events.last().unwrap().message.contains("Worker stopped"));
    }
}
