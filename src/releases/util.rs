/// Helper module to serialize and deserialize bytes as hex.
pub mod hex_string {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str_hex = String::deserialize(deserializer)?;
        let str_hex = str_hex.trim_start_matches("0x");
        hex::decode(str_hex).map_err(|err| serde::de::Error::custom(err.to_string()))
    }

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: AsRef<[u8]>,
    {
        let value = hex::encode(value);
        serializer.serialize_str(&value)
    }
}
