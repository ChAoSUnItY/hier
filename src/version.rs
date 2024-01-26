#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaVersion {
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

impl From<String> for JavaVersion {
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
                _ => Self::Invalid(value),
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
                _ => Self::Invalid(value),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{errors::HeirResult, jni_env, version::JavaVersion, HierExt};

    #[test]
    #[cfg_attr(
        not(any(
            jvm_v0, jvm_v1, jvm_v2, jvm_v3, jvm_v4, jvm_v5, jvm_v6, jvm_v7, jvm_v8, jvm_v9,
            jvm_v10, jvm_v11, jvm_v12, jvm_v13, jvm_v14, jvm_v15, jvm_v16, jvm_v17, jvm_v18,
            jvm_v19, jvm_v20, jvm_v21, jvm_v22, jvm_v23,
        )),
        ignore = "No Java version provided"
    )]
    /// Tests all possible jvm versions
    fn test_jvm_version() -> HeirResult<()> {
        let current_jvm_version: JavaVersion = if cfg!(jvm_v0) {
            JavaVersion::V0
        } else if cfg!(jvm_v1) {
            JavaVersion::V1
        } else if cfg!(jvm_v2) {
            JavaVersion::V2
        } else if cfg!(jvm_v3) {
            JavaVersion::V3
        } else if cfg!(jvm_v4) {
            JavaVersion::V4
        } else if cfg!(jvm_v5) {
            JavaVersion::V5
        } else if cfg!(jvm_v6) {
            JavaVersion::V6
        } else if cfg!(jvm_v7) {
            JavaVersion::V7
        } else if cfg!(jvm_v8) {
            JavaVersion::V8
        } else if cfg!(jvm_v9) {
            JavaVersion::V9
        } else if cfg!(jvm_v10) {
            JavaVersion::V10
        } else if cfg!(jvm_v11) {
            JavaVersion::V11
        } else if cfg!(jvm_v12) {
            JavaVersion::V12
        } else if cfg!(jvm_v13) {
            JavaVersion::V13
        } else if cfg!(jvm_v14) {
            JavaVersion::V14
        } else if cfg!(jvm_v15) {
            JavaVersion::V15
        } else if cfg!(jvm_v16) {
            JavaVersion::V16
        } else if cfg!(jvm_v17) {
            JavaVersion::V17
        } else if cfg!(jvm_v18) {
            JavaVersion::V18
        } else if cfg!(jvm_v19) {
            JavaVersion::V19
        } else if cfg!(jvm_v20) {
            JavaVersion::V20
        } else if cfg!(jvm_v21) {
            JavaVersion::V21
        } else if cfg!(jvm_v22) {
            JavaVersion::V22
        } else if cfg!(jvm_v23) {
            JavaVersion::V23
        } else {
            unreachable!()
        };

        let mut env = jni_env()?;
        let version = env.get_java_version()?;

        assert_eq!(current_jvm_version, version);

        Ok(())
    }
}
