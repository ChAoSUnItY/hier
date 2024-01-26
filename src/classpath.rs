#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassPath {
    Java(String),
    JNI(String),
}

impl ClassPath {
    pub fn convert(&self) -> Self {
        match self {
            Self::Java(cp) => {
                let mut jni_cp = cp.replace(".", "/").replace("[]", "");
                let array_dim = cp.matches("[]").count();

                if array_dim > 0 {
                    jni_cp = format!("{}L{jni_cp};", "[".repeat(array_dim));
                }

                Self::JNI(jni_cp)
            }
            Self::JNI(cp) => {
                let mut java_cp = cp.replace("/", ".");
                let array_dim = cp.matches("[").count();

                if array_dim > 0 {
                    java_cp = java_cp
                        .chars()
                        .skip_while(|c| *c == '[')
                        .skip(1)
                        .take_while(|c| *c != ';')
                        .collect();

                    java_cp = format!("{java_cp}{}", "[]".repeat(array_dim));
                }

                Self::Java(java_cp)
            }
        }
    }

    pub fn as_jni(self) -> Self {
        match self {
            Self::Java(_) => self.convert(),
            _ => self,
        }
    }

    pub fn as_java(self) -> Self {
        match self {
            Self::JNI(_) => self.convert(),
            _ => self,
        }
    }
}

impl Into<String> for ClassPath {
    fn into(self) -> String {
        match self {
            ClassPath::Java(cp) => cp,
            ClassPath::JNI(cp) => cp,
        }
    }
}

impl From<String> for ClassPath {
    /// Converts [String] into [ClassPath::Java] by default.
    fn from(value: String) -> Self {
        Self::Java(value)
    }
}

impl<'a> From<&'a str> for ClassPath {
    /// Coverts [`&str`](str) into [ClassPath::Java] by default.
    fn from(value: &'a str) -> Self {
        Self::Java(value.to_string())
    }
}
