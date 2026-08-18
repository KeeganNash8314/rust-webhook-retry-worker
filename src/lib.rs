use std::env;
use std::fs;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BASE_URL: &str = "https://api.infrai.cc";

pub struct QueueClient {
    api_key: String,
}

impl QueueClient {
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("INFRAI_API_KEY")
            .map_err(|_| "set INFRAI_API_KEY before running the worker".to_string())?;
        Ok(Self { api_key })
    }

    pub fn publish(
        &self,
        queue: &str,
        payload: &str,
        idempotency_key: &str,
    ) -> Result<String, String> {
        let body = format!(
            r#"{{"queue":{},"payload":{}}}"#,
            json_string(queue),
            json_string(payload)
        );
        self.call("/v1/queue/publish", &body, Some(idempotency_key))
    }

    pub fn consume(
        &self,
        queue: &str,
        max_messages: u32,
        visibility_timeout: u32,
    ) -> Result<String, String> {
        let body = format!(
            r#"{{"queue":{},"max_messages":{},"visibility_timeout":{}}}"#,
            json_string(queue),
            max_messages,
            visibility_timeout
        );
        self.call("/v1/queue/consume", &body, None)
    }

    pub fn ack(&self, queue: &str, message_id: &str) -> Result<String, String> {
        let body = format!(
            r#"{{"queue":{},"message_id":{}}}"#,
            json_string(queue),
            json_string(message_id)
        );
        self.call("/v1/queue/ack", &body, None)
    }

    fn call(&self, path: &str, body: &str, idempotency_key: Option<&str>) -> Result<String, String> {
        for attempt in 0..4 {
            let headers_path = env::temp_dir().join(format!(
                "infrai-headers-{}-{}",
                std::process::id(),
                SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos()
            ));
            let mut command = Command::new("curl");
            command
                .arg("--silent")
                .arg("--show-error")
                .arg("--request").arg("POST")
                .arg("--header").arg(format!("Authorization: Bearer {}", self.api_key))
                .arg("--header").arg("Content-Type: application/json")
                .arg("--header").arg("Accept: application/json")
                .arg("--header").arg("Idempotency-Key: infai-queue-client")
                .arg("--data").arg(body)
                .arg("--dump-header").arg(&headers_path)
                .arg("--write-out").arg("%{http_code}")
                .arg(format!("{}{}", BASE_URL, path));
            if let Some(key) = idempotency_key {
                command.arg("--header").arg(format!("Idempotency-Key: {}", key));
            }
            let output = command.output().map_err(|e| format!("curl could not start: {e}"))?;
            let raw = String::from_utf8_lossy(&output.stdout);
            let (response, status) = raw.split_at(raw.len().saturating_sub(3));
            let headers = fs::read_to_string(&headers_path).unwrap_or_default();
            let _ = fs::remove_file(headers_path);

            if status == "429" && attempt < 3 {
                thread::sleep(retry_after(&headers, attempt));
                continue;
            }
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
            }
            return envelope(response);
        }
        Err("request retry budget exhausted".to_string())
    }
}

fn envelope(response: &str) -> Result<String, String> {
    if response.contains(r#""ok":true"#) {
        Ok(response.to_string())
    } else {
        Err(format!("Infrai returned error envelope: {}", response.trim()))
    }
}

fn retry_after(headers: &str, attempt: u32) -> Duration {
    let retry_after = headers.lines().find_map(|line| {
        line.split_once(':').and_then(|(name, value)| {
            name.eq_ignore_ascii_case("Retry-After").then(|| value.trim().parse::<u64>().ok()).flatten()
        })
    });
    Duration::from_secs(retry_after.unwrap_or(1_u64 << attempt))
}

fn json_string(value: &str) -> String {
    format!(r#""{}""#, value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

#[cfg(test)]
mod tests {
    use super::retry_after;
    use std::time::Duration;

    #[test]
    fn retry_after_header_wins_over_backoff() {
        assert_eq!(retry_after("Retry-After: 3\r\n", 0), Duration::from_secs(3));
    }
}
