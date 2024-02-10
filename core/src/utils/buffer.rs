use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// A buffer of serializable/deserializable objects.
#[derive(Serialize, Deserialize)]
pub struct Buffer {
    pub data: Vec<u8>,
    pub ptr: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            ptr: 0,
        }
    }

    pub fn from(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
            ptr: 0,
        }
    }

    /// Set the position ptr to the beggining of the buffer.
    pub fn head(&mut self) {
        self.ptr = 0;
    }

    /// Read the serializable object from the buffer.
    pub fn read<T: Serialize + DeserializeOwned>(&mut self) -> T {
        let result: T =
            bincode::deserialize(&self.data[self.ptr..]).expect("failed to deserialize");
        let nb_bytes = bincode::serialized_size(&result).expect("failed to get serialized size");
        self.ptr += nb_bytes as usize;
        result
    }

    /// Write the serializable object from the buffer.
    pub fn write<T: Serialize + DeserializeOwned>(&mut self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serialize_into(&mut tmp, data).expect("serialization failed");
        self.data.extend(tmp);
    }
}