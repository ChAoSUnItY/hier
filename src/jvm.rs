#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JvmVersion {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    V16,
    V17,
    V18,
    V19,
    V20,
    V21,
    /// PLACEHOLDER
    V22,
    /// PLACEHOLDER
    V23,
    Invalid(String),
}

impl From<String> for JvmVersion {
    /// This conversion is compatible for "java.version" and "java.specification.version"
    /// poperties.
    fn from(value: String) -> Self {
        let version_parts = value.split(".").collect::<Vec<_>>();

        if version_parts[0] == "1" {
            // Versions before Java 9
            match version_parts[1] {
                "0" => Self::V0,
                "1" => Self::V1,
                "2" => Self::V2,
                "3" => Self::V3,
                "4" => Self::V4,
                "5" => Self::V5,
                "6" => Self::V6,
                "7" => Self::V7,
                "8" => Self::V8,
                _ => Self::Invalid(value)
            }
        } else {
            match version_parts[0] {
                "9" => Self::V9,
                "10" => Self::V10,
                "11" => Self::V11,
                "12" => Self::V12,
                "13" => Self::V13,
                "14" => Self::V14,
                "15" => Self::V15,
                "16" => Self::V16,
                "17" => Self::V17,
                "18" => Self::V18,
                "19" => Self::V19,
                "20" => Self::V20,
                "21" => Self::V21,
                "22" => Self::V22,
                "23" => Self::V23,
                _ => Self::Invalid(value)
            }
        }
    }
}
