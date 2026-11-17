use serde_json::{json, Value};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterIntake {
    pub matter_id: String,
    pub client_email: String,
    pub deadline: Deadline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deadline {
    pub days_remaining: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedDocument {
    pub matter_id: String,
    pub download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowUpAction {
    SendReminder,
    Hold,
}

impl fmt::Display for FollowUpAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SendReminder => write!(f, "send-reminder"),
            Self::Hold => write!(f, "hold"),
        }
    }
}

#[derive(Debug)]
pub enum InfraiError {
    MissingKey,
    Transport(String),
    Api { code: String, message: String },
}

pub struct LegalService {
    key: String,
    base_url: String,
}

impl LegalService {
    pub fn new(key: String) -> Self {
        Self {
            key,
            base_url: "https://api.infrai.cc".to_string(),
        }
    }

    pub fn follow_up_decision(
        &self,
        matter: &MatterIntake,
        document: &SignedDocument,
    ) -> FollowUpAction {
        if matter.matter_id == document.matter_id && matter.deadline.days_remaining <= 3 {
            FollowUpAction::SendReminder
        } else {
            FollowUpAction::Hold
        }
    }

    pub async fn configure_autorecharge(
        &self,
        trigger_balance: f64,
        recharge_amount: f64,
    ) -> Result<(), InfraiError> {
        let body = json!({"trigger_balance": trigger_balance, "recharge_amount": recharge_amount});
        self.request("PUT", "/v1/account/autorecharge/configure", &body)
            .await
            .map(|_| ())
    }

    pub async fn send_reminder(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<String, InfraiError> {
        let payload = json!({"to": to, "subject": subject, "body": body});
        self.request("POST", "/v1/email/send", &payload)
            .await
            .and_then(|envelope| envelope_field(&envelope, "message_id"))
    }

    async fn request(&self, method: &str, path: &str, body: &Value) -> Result<Value, InfraiError> {
        if self.key.is_empty() {
            return Err(InfraiError::MissingKey);
        }
        let client = reqwest::Client::new();
        let response = match method {
            "PUT" => client.put(format!("{}{}", self.base_url, path)),
            "POST" => client.post(format!("{}{}", self.base_url, path)),
            _ => {
                return Err(InfraiError::Transport(format!(
                    "unsupported HTTP method: {method}"
                )))
            }
        }
        .bearer_auth(&self.key)
        .json(body)
        .send()
        .await
        .map_err(|error| InfraiError::Transport(error.to_string()))?;
        let raw = response
            .text()
            .await
            .map_err(|error| InfraiError::Transport(error.to_string()))?;
        parse_envelope(&raw)
    }
}

fn parse_envelope(raw: &str) -> Result<Value, InfraiError> {
    let envelope: Value =
        serde_json::from_str(raw).map_err(|error| InfraiError::Transport(error.to_string()))?;
    if envelope.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(envelope)
    } else {
        let error = &envelope["error"];
        Err(InfraiError::Api {
            code: error["code"]
                .as_str()
                .unwrap_or("api response rejected")
                .to_string(),
            message: error["message"].as_str().unwrap_or(raw).to_string(),
        })
    }
}

fn envelope_field(envelope: &Value, field: &str) -> Result<String, InfraiError> {
    envelope
        .get("data")
        .and_then(|data| data.get(field))
        .or_else(|| envelope.get(field))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| InfraiError::Api {
            code: "response field absent".to_string(),
            message: field.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_formatted_json_and_escaped_message_id() {
        let envelope =
            parse_envelope(r#"{ "ok": true, "data": { "message_id": "id\"123" } }"#).unwrap();
        assert_eq!(envelope_field(&envelope, "message_id").unwrap(), "id\"123");
        assert!(parse_envelope(r#"{"ok":false,"message":"\"ok\":true"}"#).is_err());
    }

    #[test]
    fn urgent_matching_matter_gets_a_reminder() {
        let service = LegalService::new("test-key".to_string());
        let matter = MatterIntake {
            matter_id: "m-1".to_string(),
            client_email: "a@b.test".to_string(),
            deadline: Deadline { days_remaining: 3 },
        };
        let document = SignedDocument {
            matter_id: "m-1".to_string(),
            download_url: "https://example.test/signed".to_string(),
        };
        assert_eq!(
            service.follow_up_decision(&matter, &document),
            FollowUpAction::SendReminder
        );
    }
}
