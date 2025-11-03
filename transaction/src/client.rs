use serde::{Serialize, Serializer, ser::SerializeStruct};

#[derive(Debug)]
pub struct ClientAccount {
    // The "client" field
    pub client_id: u16,

    // Amount available in this client account
    pub available: f64,

    // Amount held from disputes
    pub held: f64,

    // Whether this account is locked
    pub locked: bool,
}

impl ClientAccount {
    pub fn new(client_id: u16) -> Self {
        ClientAccount {
            client_id,
            available: 0.0,
            held: 0.0,
            locked: false,
        }
    }
}

impl Serialize for ClientAccount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ClientAccount", 5)?;
        state.serialize_field("client", &self.client_id)?;
        state.serialize_field("available", &self.available)?;
        state.serialize_field("held", &self.held)?;

        let total = self.available + self.held;
        state.serialize_field("total", &total)?;

        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}
