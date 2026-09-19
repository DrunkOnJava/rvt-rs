//! Forward-only Formats/Latest registry framing, without byte-name scanning.
//!
//! Reserved tags, flags, version-like words and trailer semantics remain opaque.
//! Successful registry parsing does not establish native instance value layouts.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub offset: usize,
    pub tag: u16,
    pub introduces_definition: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub offset: usize,
    pub name: String,
    pub descriptor_offset: usize,
    pub raw_descriptor: u32,
    pub base: u8,
    pub modifier: u8,
    pub uninterpreted_flags: u16,
    pub array_count: Option<u32>,
    pub references: Vec<Reference>,
    pub nested_descriptor: Option<Box<Field>>,
    pub end: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub tag: u16,
    pub name: String,
    pub offset: usize,
    pub parent_reference: Reference,
    pub version_like_word: u32,
    pub fields: Vec<Field>,
    pub opaque_16byte_entry_count: u32,
    pub opaque_entries_offset: usize,
    pub opaque_entries_sha256: String,
    pub end: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub source_sha256: String,
    pub consumed_bytes: usize,
    pub reference_count: usize,
    pub terminator_offset: usize,
    pub classes: Vec<Class>,
}
impl Registry {
    pub fn class(&self, tag: u16) -> Option<&Class> {
        self.classes.get(usize::from(tag.checked_sub(12)?))
    }
    pub fn named(&self, name: &str) -> Option<&Class> {
        self.classes.iter().find(|c| c.name == name)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    classes: Vec<Option<Class>>,
    references: usize,
    fields: usize,
    definition_depth: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| anyhow::anyhow!("schema length overflow at {}", self.pos))?;
        let result = self.bytes.get(self.pos..end).ok_or_else(|| {
            anyhow::anyhow!("truncated schema at {}: need {count} bytes", self.pos)
        })?;
        self.pos = end;
        Ok(result)
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into()?))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn text(&mut self, count: usize) -> Result<String> {
        ensure!(
            (1..=4096).contains(&count),
            "schema name byte budget at {}: {count}",
            self.pos
        );
        Ok(std::str::from_utf8(self.take(count)?)?.to_owned())
    }
    fn reference(&mut self) -> Result<Reference> {
        ensure!(
            self.references < 1_000_000,
            "schema reference budget exceeded"
        );
        self.references += 1;
        let offset = self.pos;
        let raw = self.u16()?;
        let tag = raw & 0x7fff;
        let introduces_definition = raw & 0x8000 != 0;
        if introduces_definition {
            ensure!(
                usize::from(tag) == 12 + self.classes.len(),
                "nonsequential schema definition tag {tag} at {offset}"
            );
            self.definition()?;
        } else {
            ensure!(
                usize::from(tag) < 12 + self.classes.len(),
                "forward/unknown schema reference {tag} at {offset}"
            );
        }
        Ok(Reference {
            offset,
            tag,
            introduces_definition,
        })
    }
    fn field(&mut self, depth: usize) -> Result<Field> {
        ensure!(
            depth <= 64 && self.fields < 100_000,
            "schema field recursion/count budget exceeded"
        );
        self.fields += 1;
        let offset = self.pos;
        let count = self.u32()? as usize;
        let name = self.text(count)?;
        let descriptor_offset = self.pos;
        let raw_descriptor = self.u32()?;
        let base = raw_descriptor as u8;
        let modifier = (raw_descriptor >> 8) as u8;
        let uninterpreted_flags = (raw_descriptor >> 16) as u16;
        ensure!(
            (1..=14).contains(&base)
                && [0, 1, 2, 3, 4, 16, 17, 18, 19, 80, 81, 82, 83, 84, 96].contains(&modifier),
            "unsupported schema descriptor {raw_descriptor:#010x} at {descriptor_offset}"
        );
        let array_count = if (16..=19).contains(&modifier) {
            let count = self.u32()?;
            ensure!(
                (1..=1_000_000).contains(&count),
                "schema array count budget at {}",
                self.pos
            );
            Some(count)
        } else {
            None
        };
        let mut references = Vec::new();
        if base == 14 && [0, 16, 80].contains(&modifier) {
            references.push(self.reference()?);
        }
        let nested_descriptor = if base == 13 {
            let nested = self.field(depth + 1)?;
            references.extend(nested.references.iter().cloned());
            for _ in 1..array_count.unwrap_or(1) {
                for expected in &nested.references {
                    let actual = self.reference()?;
                    ensure!(
                        actual.tag == expected.tag,
                        "schema composite repeated reference mismatch at {}",
                        actual.offset
                    );
                    references.push(actual);
                }
            }
            Some(Box::new(nested))
        } else {
            None
        };
        Ok(Field {
            offset,
            name,
            descriptor_offset,
            raw_descriptor,
            base,
            modifier,
            uninterpreted_flags,
            array_count,
            references,
            nested_descriptor,
            end: self.pos,
        })
    }
    fn definition(&mut self) -> Result<()> {
        ensure!(
            self.definition_depth < 128 && self.classes.len() < 32756,
            "schema definition recursion/registry budget exceeded"
        );
        self.definition_depth += 1;
        let offset = self.pos;
        ensure!(
            self.u16()? == 0,
            "unsupported schema definition header at {offset}"
        );
        let count = self.u16()? as usize;
        let name = self.text(count)?;
        let ordinal = self.classes.len();
        let tag = (12 + ordinal) as u16;
        self.classes.push(None); // Registration precedes recursive definitions.
        let parent_reference = self.reference()?;
        let version_like_word = self.u32()?;
        let count = self.u32()? as usize;
        ensure!(
            count <= 100_000,
            "schema declared field budget at {}",
            self.pos
        );
        let mut fields = Vec::new();
        for _ in 0..count {
            fields.push(self.field(0)?);
        }
        let opaque_16byte_entry_count = self.u32()?;
        ensure!(
            opaque_16byte_entry_count <= 1_000_000,
            "schema opaque trailer budget exceeded"
        );
        let opaque_entries_offset = self.pos;
        let opaque_entries_sha256 = format!(
            "{:x}",
            Sha256::digest(self.take(opaque_16byte_entry_count as usize * 16)?)
        );
        self.classes[ordinal] = Some(Class {
            tag,
            name,
            offset,
            parent_reference,
            version_like_word,
            fields,
            opaque_16byte_entry_count,
            opaque_entries_offset,
            opaque_entries_sha256,
            end: self.pos,
        });
        self.definition_depth -= 1;
        Ok(())
    }
}
/// Decode one complete inflated schema member. Budgets are implementation limits,
/// not format limits. Unsupported descriptors or any trailing bytes are errors.
pub fn parse(bytes: &[u8]) -> Result<Registry> {
    ensure!(
        bytes.len() <= 64 * 1024 * 1024,
        "schema byte budget exceeded"
    );
    let mut r = Reader {
        bytes,
        pos: 0,
        classes: Vec::new(),
        references: 0,
        fields: 0,
        definition_depth: 0,
    };
    while bytes.len().saturating_sub(r.pos) > 8 {
        r.definition()?;
    }
    let terminator_offset = r.pos;
    ensure!(
        r.take(8)? == [0; 8] && r.pos == bytes.len(),
        "unsupported schema stream terminator"
    );
    let classes = r
        .classes
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| anyhow::anyhow!("incomplete schema definition"))?;
    Ok(Registry {
        source_sha256: format!("{:x}", Sha256::digest(bytes)),
        consumed_bytes: r.pos,
        reference_count: r.references,
        terminator_offset,
        classes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn field(name: &str, descriptor: u32) -> Vec<u8> {
        let mut b = (name.len() as u32).to_le_bytes().to_vec();
        b.extend(name.as_bytes());
        b.extend(descriptor.to_le_bytes());
        b
    }
    fn definition(name: &str, parent: &[u8], fields: &[Vec<u8>]) -> Vec<u8> {
        let mut b = vec![0, 0];
        b.extend((name.len() as u16).to_le_bytes());
        b.extend(name.as_bytes());
        b.extend(parent);
        b.extend(7u32.to_le_bytes());
        b.extend((fields.len() as u32).to_le_bytes());
        for f in fields {
            b.extend(f);
        }
        b.extend(0u32.to_le_bytes());
        b
    }
    fn fixture() -> Vec<u8> {
        // Outer receives slot12 before recursively registered parent13.
        let mut parent = 0x800du16.to_le_bytes().to_vec();
        parent.extend(definition("Parent", &[0, 0], &[]));
        // Composite fixed array emits one nested field header and repeated refs.
        let mut array = field("Items", 0x100d);
        array.extend(2u32.to_le_bytes());
        array.extend(field("Item", 0x000e));
        array.extend(13u16.to_le_bytes());
        array.extend(13u16.to_le_bytes());
        let mut b = definition("Outer", &parent, &[array]);
        b.extend([0; 8]);
        b
    }
    #[test]
    fn recursive_registry_slots_and_composite_reference_repetitions() {
        let bytes = fixture();
        let r = parse(&bytes).unwrap();
        assert_eq!(r.consumed_bytes, bytes.len());
        assert_eq!(r.classes.len(), 2);
        assert_eq!(r.class(12).unwrap().name, "Outer");
        assert_eq!(r.class(13).unwrap().name, "Parent");
        assert_eq!(r.class(12).unwrap().parent_reference.tag, 13);
        assert_eq!(r.class(12).unwrap().fields[0].references.len(), 2);
        assert_eq!(r.classes[0].version_like_word, 7);
        assert!(r.class(11).is_none());
    }
    #[test]
    fn malformed_descriptors_references_and_truncations_are_rejected() {
        let bytes = fixture();
        for n in 0..bytes.len() {
            assert!(parse(&bytes[..n]).is_err(), "truncated prefix {n}");
        }
        let registry = parse(&bytes).unwrap();
        let f = &registry.classes[0].fields[0];
        let mut bad = bytes.clone();
        bad[f.references[1].offset..f.references[1].offset + 2]
            .copy_from_slice(&12u16.to_le_bytes());
        assert!(
            parse(&bad)
                .unwrap_err()
                .to_string()
                .contains("repeated reference")
        );
        bad = bytes.clone();
        bad[f.descriptor_offset] = 255;
        assert!(parse(&bad).is_err());
        bad = bytes.clone();
        bad.extend([0]);
        assert!(parse(&bad).is_err());
        bad = bytes;
        bad[f.descriptor_offset + 4..f.descriptor_offset + 8]
            .copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&bad).is_err());
    }
}
