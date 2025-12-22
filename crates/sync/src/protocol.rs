pub enum SyncMessage {
    Hello { device_id: String },
    RequestChanges { list_id: String },
    Changes { list_id: String, bytes: Vec<u8> },
}
