use std::{borrow::Cow, fmt};

use encoding_rs::SHIFT_JIS;

pub struct SJisStr {
    inner: Vec<u8>,
}

impl<'a> From<&'a SJisStr> for Cow<'a, str> {
    fn from(value: &'a SJisStr) -> Self {
        let (result, _, err) = SHIFT_JIS.decode(&value.inner);
        if err {
            panic!("given sjis str inner is malformed");
        }

        result
    }
}

impl<'a> From<&'a str> for SJisStr {
    fn from(value: &'a str) -> Self {
        Self {
            inner: SHIFT_JIS.encode(value).0.to_vec(),
        }
    }
}

impl fmt::Display for SJisStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = Cow::<'_, str>::from(self);
        write!(f, "{s}")
    }
}

impl SJisStr {
    pub fn from_unchecked_vec(bytes: Vec<u8>) -> Self {
        Self { inner: bytes }
    }

    pub fn get_inner(self) -> Vec<u8> {
        self.inner
    }
}
