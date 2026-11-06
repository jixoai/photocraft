use js_sys::{Array, Uint8Array};
use wasm_bindgen::{JsCast as _, JsValue};
use web_sys::{Blob, IdbDatabase, IdbObjectStore, IdbTransactionMode};

use crate::indexed_presets::{before, open_database, request, transaction};
use crate::recovery_bridge::{Entry, MAX_DOCUMENTS, MAX_FILE_BYTES, MAX_METADATA_BYTES, MAX_STORE_BYTES, valid_key};

const DATABASE: &str = "photocraft.recovery";
const FILES: &str = "files";
const TIMEOUT_MS: f64 = 10_000.0;

fn js(error: JsValue) -> String {
    error.dyn_ref::<web_sys::DomException>().map(|error| format!("{}: {}", error.name(), error.message())).unwrap_or_else(|| format!("{error:?}"))
}

async fn inventory(store: &IdbObjectStore, deadline: f64) -> Result<Vec<(String, Blob)>, String> {
    let req = store.open_cursor().map_err(js)?;
    let mut next = request(&req);
    let mut records = Vec::new();
    let mut bytes = 0u64;
    loop {
        let value = before(&next.promise, deadline).await?;
        if value.is_null() {
            break;
        }
        if records.len() >= MAX_DOCUMENTS.saturating_mul(2) {
            return Err("Recovery store entry limit exceeded".into());
        }
        let cursor = value.dyn_into::<web_sys::IdbCursorWithValue>().map_err(|_| "Invalid recovery cursor")?;
        let key = cursor.key().map_err(js)?;
        if !key.is_string() || js_sys::JsString::from(key.clone()).length() > 140 {
            return Err("Invalid recovery record key".into());
        }
        let key = key.as_string().ok_or("Invalid recovery record key")?;
        let limit = if let Some(key) = key.strip_suffix(".json") {
            if !valid_key(key) {
                return Err("Invalid recovery metadata key".into());
            }
            MAX_METADATA_BYTES
        } else if let Some(key) = key.strip_suffix(".pcraft") {
            if !valid_key(key) {
                return Err("Invalid recovery snapshot key".into());
            }
            MAX_FILE_BYTES
        } else {
            return Err("Unknown recovery record type".into());
        };
        let blob = cursor.value().map_err(js)?.dyn_into::<Blob>().map_err(|_| "Recovery record is not a Blob")?;
        if blob.size() > limit as f64 {
            return Err("Recovery record exceeds size limit".into());
        }
        bytes = bytes.saturating_add(blob.size() as u64);
        if bytes > MAX_STORE_BYTES {
            return Err("Recovery store exceeds its byte limit".into());
        }
        records.push((key, blob));
        next = request(&req);
        cursor.continue_().map_err(js)?;
    }
    Ok(records)
}

pub async fn load() -> Result<(IdbDatabase, Vec<Entry>), String> {
    let deadline = js_sys::Date::now() + TIMEOUT_MS;
    let db = open_database(DATABASE, deadline).await?;
    let result = catalog(&db, deadline).await;
    match result {
        Ok(entries) => Ok((db, entries)),
        Err(error) => {
            db.close();
            Err(error)
        }
    }
}

async fn catalog(db: &IdbDatabase, deadline: f64) -> Result<Vec<Entry>, String> {
    let tx = db.transaction_with_str_and_mode(FILES, IdbTransactionMode::Readonly).map_err(js)?;
    let records = inventory(&tx.object_store(FILES).map_err(js)?, deadline).await?;
    let mut entries = Vec::new();
    for (key, blob) in &records {
        let Some(stem) = key.strip_suffix(".json") else { continue };
        let bytes = Uint8Array::new(&before(&blob.array_buffer(), deadline).await?).to_vec();
        let entry: Entry = serde_json::from_slice(&bytes).map_err(|error| format!("Recovery metadata: {error}"))?;
        entry.validate()?;
        if entry.key != stem || !records.iter().any(|(key, blob)| key == &format!("{stem}.pcraft") && blob.size() as usize == entry.bytes) {
            return Err("Recovery snapshot and metadata do not match".into());
        }
        entries.push(entry);
    }
    if entries.len().saturating_mul(2) != records.len() {
        return Err("Incomplete recovery records; stored data has been retained".into());
    }
    Ok(entries)
}

pub async fn read(db: &IdbDatabase, key: &str) -> Result<Vec<u8>, String> {
    if !valid_key(key) {
        return Err("Invalid recovery key".into());
    }
    let tx = db.transaction_with_str_and_mode(FILES, IdbTransactionMode::Readonly).map_err(js)?;
    let req = tx.object_store(FILES).map_err(js)?.get(&JsValue::from_str(&format!("{key}.pcraft"))).map_err(js)?;
    let pending = request(&req);
    let deadline = js_sys::Date::now() + TIMEOUT_MS;
    let blob = before(&pending.promise, deadline).await?.dyn_into::<Blob>().map_err(|_| "Recovery snapshot not found")?;
    if blob.size() < 1.0 || blob.size() > MAX_FILE_BYTES as f64 {
        return Err("Recovery snapshot exceeds size limit".into());
    }
    Ok(Uint8Array::new(&before(&blob.array_buffer(), deadline).await?).to_vec())
}

pub async fn write(db: &IdbDatabase, key: &str, snapshot: Option<(&Entry, &[u8])>) -> Result<(), String> {
    if !valid_key(key) {
        return Err("Invalid recovery key".into());
    }
    let metadata = snapshot
        .map(|(entry, _)| -> Result<Vec<u8>, String> {
            entry.validate()?;
            if entry.key != key {
                return Err("Recovery key mismatch".into());
            }
            let bytes = serde_json::to_vec(entry).map_err(|error| error.to_string())?;
            if bytes.len() > MAX_METADATA_BYTES {
                return Err("Recovery metadata too large".into());
            }
            Ok(bytes)
        })
        .transpose()?;
    let tx = db.transaction_with_str_and_mode(FILES, IdbTransactionMode::Readwrite).map_err(js)?;
    let completion = transaction(&tx);
    let store = tx.object_store(FILES).map_err(js)?;
    let deadline = js_sys::Date::now() + TIMEOUT_MS;
    let queued = async {
        let records = inventory(&store, deadline).await?;
        let project_key = format!("{key}.pcraft");
        let metadata_key = format!("{key}.json");
        let retained: Vec<_> = records.iter().filter(|(key, _)| key != &project_key && key != &metadata_key).collect();
        if let Some(((entry, bytes), metadata)) = snapshot.zip(metadata.as_deref()) {
            if bytes.len() != entry.bytes || bytes.len() > MAX_FILE_BYTES {
                return Err("Recovery snapshot size mismatch".into());
            }
            let total = retained
                .iter()
                .fold(0u64, |sum, (_, blob)| sum.saturating_add(blob.size() as u64))
                .saturating_add(bytes.len() as u64)
                .saturating_add(metadata.len() as u64);
            if retained.len().saturating_add(2) > MAX_DOCUMENTS.saturating_mul(2) || total > MAX_STORE_BYTES {
                return Err("Recovery storage limit reached; export your work and explicitly discard unneeded snapshots".into());
            }
            for (key, bytes) in [(&project_key, bytes), (&metadata_key, metadata)] {
                let blob = Blob::new_with_u8_array_sequence(&Array::of1(&Uint8Array::from(bytes))).map_err(js)?;
                store.put_with_key(&blob, &JsValue::from_str(key)).map_err(js)?;
            }
        } else {
            store.delete(&JsValue::from_str(&project_key)).map_err(js)?;
            store.delete(&JsValue::from_str(&metadata_key)).map_err(js)?;
        }
        Ok::<_, String>(())
    }
    .await;
    if let Err(error) = queued {
        let _ = tx.abort();
        let _ = before(&completion.promise, deadline).await;
        return Err(error);
    }
    let result = before(&completion.promise, deadline).await.map(|_| ());
    if result.is_err() {
        let _ = tx.abort();
    }
    result
}
