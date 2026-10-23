//! The decoder for classes the schema-directed walk reads field by field.
//!
//! Most registered classes need nothing class-specific to decode: the walk
//! reads their fields in the order `Formats/Latest` declares them, and the
//! typed views in the sibling modules (`from_decoded`) pick out the fields
//! they know. One [`SimpleDecoder`] per class name does that, so
//! [`all_decoders`](super::all_decoders) registers each of those classes
//! with its name alone.

use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, WalkerLimits};
use crate::{Error, Result};

/// Decodes instances of one class by walking its declared fields, refusing
/// a schema entry for any other class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SimpleDecoder {
    class_name: &'static str,
}

impl SimpleDecoder {
    /// The decoder for instances of the class named `class_name`, as
    /// `Formats/Latest` spells it (for example `"Duct"`).
    pub fn new(class_name: &'static str) -> Self {
        Self { class_name }
    }
}

impl ElementDecoder for SimpleDecoder {
    fn class_name(&self) -> &'static str {
        self.class_name
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        index: &HandleIndex,
    ) -> Result<DecodedElement> {
        self.decode_with_limits(bytes, schema, index, WalkerLimits::default())
    }

    fn decode_with_limits(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
        limits: WalkerLimits,
    ) -> Result<DecodedElement> {
        if schema.name != self.class_name {
            return Err(Error::SchemaMismatch {
                decoder: self.class_name,
                schema: schema.name.clone(),
            });
        }
        Ok(crate::walker::decode_instance_with_limits(
            bytes, 0, schema, limits,
        ))
    }
}
