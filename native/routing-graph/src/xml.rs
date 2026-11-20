use std::collections::BTreeMap;

use quick_xml::{Reader, events::Event};

const MAX_XML_NODES: usize = 200_000;
pub const MAX_XML_DEPTH: usize = 96;

#[derive(Debug, Default)]
pub struct XmlNode {
    pub name: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<XmlNode>,
}

impl XmlNode {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }

    pub fn child_id(&self, id: &str) -> Option<&Self> {
        self.children.iter().find(|n| n.attr("x:id") == Some(id))
    }

    pub fn descendant_id(&self, id: &str) -> Option<&Self> {
        if self.attr("x:id") == Some(id) {
            return Some(self);
        }
        self.children.iter().find_map(|n| n.descendant_id(id))
    }

    pub fn uid(&self, id: &str) -> Option<String> {
        self.child_id(id)?.attr("uid").map(str::to_owned)
    }
}

pub fn parse_xml(bytes: &[u8]) -> Result<XmlNode, String> {
    std::str::from_utf8(bytes).map_err(|_| "XML metadata is not UTF-8")?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut stack = vec![XmlNode::default()];
    let mut count = 0usize;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let empty = matches!(&event, Event::Empty(_));
                let tag = match event {
                    Event::Start(tag) | Event::Empty(tag) => tag,
                    _ => unreachable!(),
                };
                count += 1;
                if count > MAX_XML_NODES || stack.len() > MAX_XML_DEPTH {
                    return Err("XML node or nesting limit exceeded".into());
                }
                let mut node = XmlNode {
                    name: String::from_utf8_lossy(tag.name().as_ref()).into_owned(),
                    ..XmlNode::default()
                };
                for attr in tag.attributes() {
                    let attr = attr.map_err(|e| e.to_string())?;
                    node.attrs.insert(
                        String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                        attr.decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned(),
                    );
                }
                if empty {
                    stack
                        .last_mut()
                        .ok_or("invalid XML stack")?
                        .children
                        .push(node);
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                if stack.len() < 2 {
                    return Err("unexpected XML closing tag".into());
                }
                let node = stack.pop().ok_or("invalid XML stack")?;
                stack
                    .last_mut()
                    .ok_or("invalid XML stack")?
                    .children
                    .push(node);
            }
            Event::DocType(_) => return Err("DTD declarations are not supported".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err("unclosed XML element".into());
    }
    let mut doc = stack.pop().ok_or("empty XML")?;
    if doc.children.len() != 1 {
        return Err("XML must contain one root element".into());
    }
    Ok(doc.children.remove(0))
}
