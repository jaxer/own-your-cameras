//! The camera's name on the network: `[camera]` in `cam.toml`. Every MQTT topic, Home Assistant id and the
//! DHCP hostname derive from it.

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Identity {
    id: String,
    name: String,
}

impl Default for Identity {
    fn default() -> Self {
        Self {
            id: "e1zoom".into(),
            name: "E1 Zoom".into(),
        }
    }
}

impl Identity {
    /// `id` goes into topics and hostnames, so it is `[a-z0-9_]{1,32}`; `name` is what Home Assistant shows.
    pub fn new(id: &str, name: &str) -> Result<Self, String> {
        let identity = Self {
            id: id.into(),
            name: name.into(),
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), String> {
        let id_ok = (1..=32).contains(&self.id.len())
            && self
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !id_ok {
            return Err("[camera] id must be 1-32 of a-z, 0-9 and _".into());
        }
        let chars = self.name.chars().count();
        if !(1..=64).contains(&chars) || self.name.chars().any(char::is_control) {
            return Err("[camera] name must be 1-64 printable characters".into());
        }
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// `camera/<id>/<leaf>`.
    pub fn topic(&self, leaf: &str) -> String {
        format!("camera/{}/{leaf}", self.id)
    }

    pub fn availability(&self) -> String {
        self.topic("availability")
    }

    /// The MQTT client id, which is also the Home Assistant device identifier.
    pub fn client_id(&self) -> String {
        format!("cam_{}", self.id)
    }

    pub fn hostname(&self) -> &str {
        &self.id
    }
}
