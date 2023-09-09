use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};

pub fn parse_soap(
    response: &[u8],
    element_to_find: &str,
    parent: Option<&str>,
    only_once: bool,
) -> Vec<String> {
    let mut element_found = false;
    let mut result = Vec::new();

    let buffer = BufReader::new(response);
    let parser = EventReader::new(buffer);

    let mut parent_found = match parent {
        Some(_) => false,
        None => true,
    };

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                let element = name.local_name;

                if !parent_found && element == parent.unwrap() {
                    parent_found = true;
                }

                if parent_found && element == element_to_find {
                    element_found = true;
                }
            }
            Ok(XmlEvent::EndElement { name, .. }) => {
                if element_found && name.local_name == element_to_find {
                    element_found = false;
                }
            }
            Ok(XmlEvent::Characters(chars)) => {
                if element_found {
                    result.push(chars);

                    if only_once {
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
            // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
            _ => {}
        }
    }

    result
}
