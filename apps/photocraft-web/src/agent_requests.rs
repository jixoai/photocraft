use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::agent_error::Failure;

const MAX_REQUESTS: usize = 1024;
const MAX_REPLY_BYTES: usize = 16 * 1024;
const MAX_LEDGER_BYTES: usize = 16 * 1024 * 1024;

struct Record {
    fingerprint: String,
    state: &'static str,
    reply: Option<Value>,
    truncated: bool,
}

pub struct Ledger {
    instance: String,
    records: BTreeMap<String, Record>,
    reserved: usize,
}

impl Ledger {
    pub fn new(instance: String) -> Self {
        Self { instance, records: BTreeMap::new(), reserved: 0 }
    }

    fn check_instance(&self, instance: &str) -> Result<(), Failure> {
        if self.instance.is_empty() {
            return Err(Failure::rejected(
                "TRACKING_UNAVAILABLE",
                "A secure page identity could not be created",
                "Use a secure browser context; do not submit tracked edits without a valid instance.",
            ));
        }
        if instance != self.instance {
            return Err(Failure::rejected(
                "INSTANCE_MISMATCH",
                "The page instance has changed",
                "Do not replay the old plan. Inspect/recover the document and reconcile its state first.",
            ));
        }
        Ok(())
    }

    pub fn fingerprint(command: &str, params: &Value) -> Result<String, Failure> {
        serde_json::to_string(&json!({"command":command,"params":params})).map_err(|error| Failure::execution(error.to_string()))
    }

    pub fn existing(&self, instance: &str, id: &str, fingerprint: &str) -> Result<Option<Value>, Failure> {
        self.check_instance(instance)?;
        let Some(record) = self.records.get(id) else { return Ok(None) };
        if record.fingerprint != fingerprint {
            return Err(Failure::rejected(
                "REQUEST_ID_CONFLICT",
                "requestId was already used with different arguments",
                "Use a new ID only for a genuinely new edit; inspect the original request before changing the plan.",
            ));
        }
        Ok(Some(self.record_status(id, record)))
    }

    pub fn reserve(&mut self, instance: &str, id: &str, fingerprint: String) -> Result<Value, Failure> {
        if let Some(status) = self.existing(instance, id, &fingerprint)? {
            return Ok(status);
        }
        let bytes = fingerprint.len().saturating_add(id.len()).saturating_add(MAX_REPLY_BYTES);
        if self.records.len() >= MAX_REQUESTS || self.reserved.saturating_add(bytes) > MAX_LEDGER_BYTES {
            return Err(Failure::rejected(
                "LEDGER_FULL",
                "The page's tracked-request ledger is full",
                "Save the project and reconcile the plan before starting a new page instance. Records are never silently evicted.",
            ));
        }
        self.reserved = self.reserved.saturating_add(bytes);
        let record = Record { fingerprint, state: "queued", reply: None, truncated: false };
        let status = self.record_status(id, &record);
        self.records.insert(id.into(), record);
        Ok(status)
    }

    pub fn started(&mut self, id: &str) {
        if let Some(record) = self.records.get_mut(id)
            && record.state == "queued"
        {
            record.state = "running";
        }
    }

    pub fn finish(&mut self, id: &str, reply: Value) {
        let Some(record) = self.records.get_mut(id) else { return };
        if !matches!(record.state, "queued" | "running") {
            return;
        }
        record.state = if reply["ok"] == true {
            "succeeded"
        } else if reply["errorInfo"]["execution"] == "unknown" {
            "unknown"
        } else {
            "failed"
        };
        let size = serde_json::to_vec(&reply).map_or(usize::MAX, |encoded| encoded.len());
        if size <= MAX_REPLY_BYTES {
            record.reply = Some(reply);
        } else {
            record.truncated = true;
        }
    }

    fn record_status(&self, id: &str, record: &Record) -> Value {
        json!({"instanceId":self.instance,"requestId":id,"state":record.state,"reply":record.reply,"replyTruncated":record.truncated})
    }

    pub fn status(&self, instance: Option<&str>, id: Option<&str>) -> Result<Value, Failure> {
        self.check_instance(instance.unwrap_or(&self.instance))?;
        if let Some(instance) = instance {
            self.check_instance(instance)?;
        }
        let Some(id) = id else {
            return Ok(
                json!({"instanceId":self.instance,"tracked":self.records.len(),"maxRequests":MAX_REQUESTS,"maxBytes":MAX_LEDGER_BYTES,"maxReplyBytes":MAX_REPLY_BYTES,"persistent":false}),
            );
        };
        let record = self.records.get(id).ok_or_else(|| {
            Failure::rejected(
                "REQUEST_NOT_FOUND",
                "No tracked request with that ID in this page",
                "Verify the page instance and submission arguments before deciding whether to submit.",
            )
        })?;
        Ok(self.record_status(id, record))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrying_submission_never_replaces_or_restarts_a_request() {
        let mut ledger = Ledger::new("page-a".into());
        let fingerprint = Ledger::fingerprint("layer.new.layer", &json!({"name":"editable"})).unwrap();
        assert!(ledger.existing("page-a", "step-1", &fingerprint).unwrap().is_none());
        assert_eq!(ledger.reserve("page-a", "step-1", fingerprint.clone()).unwrap()["state"], "queued");
        ledger.started("step-1");
        assert_eq!(ledger.existing("page-a", "step-1", &fingerprint).unwrap().unwrap()["state"], "running");
        ledger.finish("step-1", json!({"ok":true,"result":{"layer":8}}));
        let status = ledger.reserve("page-a", "step-1", fingerprint).unwrap();
        assert_eq!(status["state"], "succeeded");
        assert_eq!(status["reply"]["result"]["layer"], 8);
        assert_eq!(ledger.records.len(), 1);
        assert!(ledger.reserve("page-a", "step-1", "different".into()).is_err());
        assert!(ledger.reserve("page-b", "step-2", "new".into()).is_err());
    }

    #[test]
    fn uncertain_failure_is_terminal_and_never_safe_to_replay_automatically() {
        let mut ledger = Ledger::new("page-a".into());
        ledger.reserve("page-a", "step-1", "command".into()).unwrap();
        ledger.started("step-1");
        ledger.finish("step-1", Failure::interrupted("deadline", false).reply());
        ledger.finish("step-1", json!({"ok":true}));
        let status = ledger.status(Some("page-a"), Some("step-1")).unwrap();
        assert_eq!(status["state"], "unknown");
        assert_eq!(status["reply"]["errorInfo"]["retrySafe"], false);
        assert!(ledger.status(Some("page-b"), Some("step-1")).is_err());
        assert!(ledger.status(Some("page-a"), Some("missing")).is_err());
    }

    #[test]
    fn bounded_ledger_never_evicts_deduplication_records() {
        let mut ledger = Ledger::new("page-a".into());
        ledger.reserve("page-a", "first", "first".into()).unwrap();
        ledger.finish("first", json!({"ok":true,"result":"x".repeat(MAX_REPLY_BYTES)}));
        let status = ledger.status(Some("page-a"), Some("first")).unwrap();
        assert_eq!(status["state"], "succeeded");
        assert_eq!(status["replyTruncated"], true);
        assert!(status["reply"].is_null());
        for index in 1..MAX_REQUESTS {
            if ledger.reserve("page-a", &format!("step-{index}"), "command".into()).is_err() {
                break;
            }
        }
        assert!(ledger.reserve("page-a", "overflow", "command".into()).is_err());
        assert!(ledger.existing("page-a", "first", "first").unwrap().is_some());
        assert!(ledger.reserved <= MAX_LEDGER_BYTES);
    }
}
