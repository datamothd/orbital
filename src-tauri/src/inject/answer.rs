use super::image::error;
use crate::files::MAX_DOCUMENT_SIZE;
use quick_xml::{NsReader, XmlVersion, events::Event, name::ResolveResult};
use std::io;

pub(super) fn decode(bytes: &[u8]) -> io::Result<String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_DOCUMENT_SIZE {
        return Err(error("Choose a non-empty answer file under 1 MiB."));
    }
    let text = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if !bytes.len().is_multiple_of(2) {
            return Err(error("The answer file contains invalid UTF-16."));
        }
        let little = bytes[0] == 0xff;
        let chars: Vec<_> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                if little {
                    u16::from_le_bytes(*pair)
                } else {
                    u16::from_be_bytes(*pair)
                }
            })
            .collect();
        String::from_utf16(&chars).map_err(|_| error("The answer file contains invalid UTF-16."))?
    } else {
        String::from_utf8(
            bytes
                .strip_prefix(&[0xef, 0xbb, 0xbf])
                .unwrap_or(bytes)
                .to_vec(),
        )
        .map_err(|_| error("The answer file must use UTF-8 or UTF-16."))?
    };
    Ok(text)
}

pub(super) fn validate_answer(bytes: &[u8]) -> io::Result<()> {
    let text = decode(bytes)?;
    if text
        .chars()
        .any(|ch| ch < ' ' && !matches!(ch, '\t' | '\n' | '\r'))
    {
        return Err(error("The answer file contains invalid XML characters."));
    }
    let mut reader = NsReader::from_str(&text);
    let mut depth = 0usize;
    let mut roots = 0;
    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|reason| error(format!("Invalid answer XML: {reason}")))?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if matches!(namespace, ResolveResult::Unknown(_)) {
                    return Err(error("The answer file uses an undeclared XML namespace."));
                }
                if depth == 0 {
                    roots += 1;
                    if roots != 1
                        || element.local_name().as_ref() != "unattend"
                        || !matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == "urn:schemas-microsoft-com:unattend")
                    {
                        return Err(error(
                            "Choose a Windows unattend XML file with the correct namespace.",
                        ));
                    }
                }
                for attribute in element.attributes() {
                    attribute
                        .map_err(io::Error::other)?
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(io::Error::other)?;
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| error("Invalid answer XML."))?;
            }
            Event::Text(text) => {
                let text = text.xml_content(XmlVersion::Implicit1_0);
                if depth == 0 && !text.trim().is_empty() {
                    return Err(error("Unexpected text outside the unattend element."));
                }
            }
            Event::CData(_) if depth == 0 => {
                return Err(error("Unexpected data outside the unattend element."));
            }
            Event::GeneralRef(reference) => {
                if depth == 0
                    || reference
                        .resolve_char_ref()
                        .map_err(io::Error::other)?
                        .is_none()
                        && !matches!(reference.as_ref(), "amp" | "lt" | "gt" | "apos" | "quot")
                {
                    return Err(error("The answer file contains an invalid XML entity."));
                }
            }
            Event::DocType(_) => {
                return Err(error(
                    "Answer files with DTDs or external entities are not supported.",
                ));
            }
            Event::Decl(_) if roots != 0 => return Err(error("Invalid XML declaration.")),
            Event::Eof => break,
            _ => {}
        }
    }
    if roots != 1 || depth != 0 {
        return Err(error("The answer file is incomplete."));
    }
    Ok(())
}
