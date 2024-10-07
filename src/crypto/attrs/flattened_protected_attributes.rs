use super::{
    flattened_encrypted_attributes::FlattenedEncryptedAttributes, normalized_protected_attributes::NormalizedKey, NormalizedProtectedAttributes
};
use crate::crypto::SealError;
use cipherstash_client::{
    credentials::{service_credentials::ServiceToken, Credentials},
    encryption::{BytesWithDescriptor, Encryption, Plaintext},
};
use itertools::Itertools;

// TODO: This thing is confusingly named - it holds unencrypted attributes that are intended for encryption
/// Describes a set of flattened protected attributes intended for encryption.
#[derive(PartialEq, Debug)]
pub(crate) struct FlattenedProtectedAttributes(pub(super) Vec<FlattenedProtectedAttribute>);

impl FlattenedProtectedAttributes {
    pub(crate) fn new_with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn into_iter(self) -> impl Iterator<Item = FlattenedProtectedAttribute> {
        self.0.into_iter()
    }

    /// Encrypt all attributes in the set and return a list of [FlattenedEncryptedAttributes] objects.
    /// Each [FlattenedEncryptedAttributes] object contains `chunk_size` encrypted attributes.
    pub(crate) async fn encrypt_all(
        self,
        cipher: &Encryption<impl Credentials<Token = ServiceToken>>,
        chunk_size: usize,
    ) -> Result<Vec<FlattenedEncryptedAttributes>, SealError> {
        println!("Encrypting all attributes, chunk size: {}", chunk_size);

        let x = cipher
            .encrypt(self.0.into_iter())
            .await?;

        dbg!(&x);

        x
            .into_iter()
            .chunks(chunk_size)
            .into_iter()
            .map(|chunk| Ok(chunk.collect::<FlattenedEncryptedAttributes>()))
            .collect()
    }
}

impl Extend<FlattenedProtectedAttribute> for FlattenedProtectedAttributes {
    fn extend<T: IntoIterator<Item = FlattenedProtectedAttribute>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

/// Allows us to collect a list of (Plaintext, String) tuples into a [FlattenedProtectedAttributes] object.
impl FromIterator<(Plaintext, String)> for FlattenedProtectedAttributes {
    fn from_iter<T: IntoIterator<Item = (Plaintext, String)>>(iter: T) -> Self {
        Self(iter.into_iter().map(|(plaintext, key)| FlattenedProtectedAttribute::new(plaintext, key)).collect())
    }
}

/// Describes a flattened protected attribute intended for encryption.
/// It is composed of a [Plaintext] and a [FlattenedKey].
///
// TODO: Only implement Debug in tests
#[derive(PartialEq, Debug)]
pub(crate) struct FlattenedProtectedAttribute {
    plaintext: Plaintext,
    key: FlattenedKey,
}

impl FlattenedProtectedAttribute {
    pub(super) fn new(plaintext: impl Into<Plaintext>, key: impl Into<FlattenedKey>) -> Self {
        Self {
            plaintext: plaintext.into(),
            key: key.into(),
        }
    }

    /// Consume and return the [Plaintext], key and subkey (if one is set) of the attribute.
    pub(crate) fn normalize_into_parts(self) -> (Plaintext, NormalizedKey, Option<String>) {
        let (normalized, subkey) = self.key.normalize();
        (self.plaintext, normalized, subkey)
    }

    fn descriptor(&self) -> String {
        self.key.descriptor()
    }
}

impl Into<BytesWithDescriptor> for FlattenedProtectedAttribute {
    fn into(self) -> BytesWithDescriptor {
        BytesWithDescriptor {
            bytes: self.plaintext.to_vec(),
            descriptor: self.descriptor(),
        }
    }
}

/// Describes a flattened key in a set of [FlattenedProtectedAttributes].
///
/// The key is composed of a prefix, a key, and an optional subkey.
/// A Map would have a key and a subkey, while a scalar would only have a key.
// TODO: Only implement Debug in tests
#[derive(PartialEq, Hash, Eq, Clone, Debug)]
pub(super) struct FlattenedKey {
    prefix: Option<String>,
    key: String,
    subkey: Option<String>,
}

impl FlattenedKey {
    pub(super) fn new(prefix: Option<String>, key: impl Into<String>) -> Self {
        Self {
            prefix,
            key: key.into(),
            subkey: None,
        }
    }

    /// Converts this into a [NormalizedKey] based on whether it has a subkey or not.
    /// If it has a subkey, it is a map, otherwise it is a scalar.
    /// The subkey is returned along with the normalized key (if it exists).
    /// Prefix is discarded as it is not needed after decryption.
    pub(super) fn normalize(self) -> (NormalizedKey, Option<String>) {
        match self.subkey {
            Some(_) => (NormalizedKey::new_map(self.key), self.subkey),
            None => (NormalizedKey::new_scalar(self.key), None),
        }
    }

    // TODO: Rename this to try_parse
    /// Parse a descriptor into a [FlattenedKey].
    pub(super) fn parse(descriptor: &str) -> Self {
        fn split_subkey(prefix: Option<String>, key: &str) -> FlattenedKey {
            match key.split_once(".") {
                None => FlattenedKey::new(prefix, key),
                Some((key, subkey)) => FlattenedKey::new(prefix, key).with_subkey(subkey),
            }
        }
        match descriptor.split_once("/") {
            None => split_subkey(None, descriptor),
            Some((prefix, key)) => split_subkey(Some(prefix.to_string()), key),
        }
    }

    pub(super) fn with_subkey(mut self, subkey: impl Into<String>) -> Self {
        self.subkey = Some(subkey.into());
        self
    }

    pub(crate) fn descriptor(&self) -> String {
        match (self.prefix.as_ref(), self.subkey.as_ref()) {
            (Some(prefix), Some(subkey)) => format!("{}/{}.{}", prefix, self.key, subkey),
            (Some(prefix), None) => format!("{}/{}", prefix, self.key),
            (None, Some(subkey)) => format!("{}.{}", self.key, subkey),
            (None, None) => self.key.to_string(),
        }
    }

    pub(crate) fn has_subkey(&self) -> bool {
        self.subkey.is_some()
    }

    /// Consume and return the parts of the key (not including the prefix).
    pub fn into_key_parts(self) -> (String, Option<String>) {
        (self.key, self.subkey)
    }
}

// TODO: Change to TryFrom
impl From<String> for FlattenedKey {
    fn from(key: String) -> Self {
        Self::parse(key.as_str())
    }
}

impl From<&str> for FlattenedKey {
    fn from(key: &str) -> Self {
        Self::parse(key)
    }
}

impl From<(String, String)> for FlattenedKey {
    fn from((prefix, key): (String, String)) -> Self {
        // TODO: Check that neither string is empty
        Self::new(Some(prefix), key)
    }
}

impl From<(&str, &str)> for FlattenedKey {
    fn from((prefix, key): (&str, &str)) -> Self {
        Self::new(Some(prefix.to_string()), key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flattened_key_from_string() {
        assert_eq!(FlattenedKey::new(None, "foo"), "foo".into());
    }

    #[test]
    fn test_flattened_key_from_tuple() {
        assert_eq!(
            FlattenedKey::new(Some("prefix".to_string()), "foo"),
            ("prefix", "foo").into()
        );
    }

    #[test]
    fn test_flattened_key_descriptor() {
        assert_eq!(FlattenedKey::new(None, "foo").descriptor(), "foo");
        assert_eq!(
            FlattenedKey::new(Some("pref".to_string()), "foo").descriptor(),
            "pref/foo"
        );
        assert_eq!(
            FlattenedKey::new(None, "foo").with_subkey("x").descriptor(),
            "foo.x"
        );
        assert_eq!(
            FlattenedKey::new(Some("pref".to_string()), "foo")
                .with_subkey("x")
                .descriptor(),
            "pref/foo.x"
        );
    }

    // TODO: Test normalize

    #[test]
    fn test_into_iter() {
        let fpa1 = FlattenedProtectedAttribute::new("value1", "key1");
        let fpa2 = FlattenedProtectedAttribute::new("value2", "key2");
        let fpa3 = FlattenedProtectedAttribute::new("value3", "key3");

        let fpa = FlattenedProtectedAttributes(vec![fpa1, fpa2, fpa3]);

        let mut iter = fpa.into_iter();

        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value1", "key1")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value2", "key2")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value3", "key3")
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_chain_iters() {
        let fpa1 = FlattenedProtectedAttributes(vec![
            FlattenedProtectedAttribute::new("value1", "key1"),
            FlattenedProtectedAttribute::new("value2", "key2"),
            FlattenedProtectedAttribute::new("value3", "key3"),
        ]);

        let fpa2 = FlattenedProtectedAttributes(vec![
            FlattenedProtectedAttribute::new("value4", "key4"),
            FlattenedProtectedAttribute::new("value5", "key5"),
            FlattenedProtectedAttribute::new("value6", "key6"),
        ]);

        let fpa3 = FlattenedProtectedAttributes(vec![
            FlattenedProtectedAttribute::new("value7", "key7"),
            FlattenedProtectedAttribute::new("value8", "key8"),
            FlattenedProtectedAttribute::new("value9", "key9"),
        ]);

        let fpas = vec![fpa1, fpa2, fpa3];
        let mut iter = fpas.into_iter().flat_map(|fpa| fpa.into_iter());

        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value1", "key1")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value2", "key2")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value3", "key3")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value4", "key4")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value5", "key5")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value6", "key6")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value7", "key7")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value8", "key8")
        );
        assert_eq!(
            iter.next().unwrap(),
            FlattenedProtectedAttribute::new("value9", "key9")
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_flattened_key_parse() {
        assert_eq!(FlattenedKey::parse("key"), "key".into());
        assert_eq!(FlattenedKey::parse("prefix/key"), ("prefix", "key").into());
        assert_eq!(FlattenedKey::parse("key.subkey"), FlattenedKey::from("key").with_subkey("subkey"));
        assert_eq!(FlattenedKey::parse("prefix/key.subkey"), FlattenedKey::from(("prefix", "key")).with_subkey("subkey"));
    }
}
