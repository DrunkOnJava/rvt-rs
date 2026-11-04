//! A family file's own document, read record by record (RE-327, #156).
//!
//! [`crate::native_document`] reads a project through its index (the
//! creation episodes of `Global/History` and the routing rows of
//! `Global/DocumentIncrementTable`), and that index refuses every family
//! file of Revit 2016 to 2027 ("invalid increment episode range"). A family
//! file's elements are the plain groups of its own partitions, framed as a
//! project's are (RE-280), so this walks them without the index and hands
//! each record's body to the caller, who decodes the classes it needs with
//! [`crate::native_parameters`].

use crate::le_bytes;
use crate::{RevitFile, compression, native_document, native_segments};
use anyhow::{Result, ensure};

/// What a family file's records are read with.
pub struct Document {
    /// The release the file's layout is, 2015 to 2027.
    pub release: u32,
    /// The file's own `Formats/Latest`.
    pub registry: crate::schema_registry::Registry,
    /// The names of its `Partitions/NN` streams, in name order.
    pub streams: Vec<String>,
}

/// A record of a plain group, borrowed for the length of the callback.
pub struct Entry<'a> {
    pub id: u64,
    pub channel: u64,
    pub stream: &'a str,
    /// The class its body's tag names in the file's schema.
    pub class: Option<&'a str>,
    pub body: &'a [u8],
}

/// How many groups [`walk`] met.
#[derive(Debug, Default, Clone, Copy)]
pub struct Walked {
    /// Plain groups of the channels asked for.
    pub groups: usize,
    /// Keyed groups (a loaded family's document inside a project), skipped.
    pub keyed_groups: usize,
    /// Plain groups whose records did not frame, skipped.
    pub unframed_groups: usize,
    /// Records handed to the callback.
    pub records: usize,
}

/// The schema and partitions of a family file. Errs outside the releases the
/// native record framing is measured on, or where the schema does not parse.
pub fn open(file: &mut RevitFile) -> Result<Document> {
    let release = file.layout_release()?;
    ensure!(
        native_document::NATIVE_RELEASES.contains(&release),
        "native record framing is unvalidated for Revit {release}"
    );
    let registry =
        crate::schema_registry::parse(&native_document::read_single(file, "Formats/Latest")?)?;
    let mut streams: Vec<String> = file
        .stream_names()
        .iter()
        .filter(|name| name.starts_with("Partitions/"))
        .cloned()
        .collect();
    streams.sort();
    Ok(Document {
        release,
        registry,
        streams,
    })
}

/// Every record of the plain groups of `channels` in `document`'s
/// partitions, in stream order, copies in several partitions each handed
/// over. A group that does not frame is counted and skipped.
pub fn walk(
    file: &mut RevitFile,
    document: &Document,
    channels: &[u64],
    mut emit: impl FnMut(&Entry) -> Result<()>,
) -> Result<Walked> {
    let mut walked = Walked::default();
    let budget = native_document::group_budget(file, native_document::DEFAULT_MAX_GROUP_BYTES);
    for name in &document.streams {
        let stored = file.read_stream(name)?;
        let prepared = compression::prepare_stream_for_inflate(name, &stored);
        let inflated = file.inflated_partition(name).ok();
        native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            &document.registry,
            native_document::partition_budget(
                budget,
                native_document::DEFAULT_MAX_GROUP_BYTES,
                inflated.as_deref(),
            ),
            |source, bytes| {
                if source.content_key.is_some() {
                    walked.keyed_groups += 1;
                    return Ok(());
                }
                if !channels.contains(&source.channel) {
                    return Ok(());
                }
                walked.groups += 1;
                let Ok(records) = native_document::group_records(source, bytes, document.release)
                else {
                    walked.unframed_groups += 1;
                    return Ok(());
                };
                for (id, _, start, end) in records {
                    let body = &bytes[start..end];
                    let class = le_bytes::u16_at(body, 0)
                        .and_then(|tag| document.registry.class(tag))
                        .map(|class| class.name.as_str());
                    walked.records += 1;
                    emit(&Entry {
                        id,
                        channel: source.channel,
                        stream: name.as_str(),
                        class,
                        body,
                    })?;
                }
                Ok(())
            },
        )?;
    }
    Ok(walked)
}
