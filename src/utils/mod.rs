use log::debug;
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};

pub fn parse_soap(
    response: &[u8],
    element_to_find: &str,
    parent: Option<&str>,
    is_single: bool,
    is_attributes: bool,
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
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                let element = name.local_name;

                if !parent_found && element == parent.unwrap() {
                    debug!("Start PARENT element found: {element}");
                    parent_found = true;
                }

                if parent_found && element == element_to_find {
                    debug!("START element found: {element}");
                    element_found = true;
                }

                if element_found && !attributes.is_empty() && is_attributes {
                    let attrs: Vec<_> = attributes
                        .iter()
                        .map(|a| format!("{}={:?}", &a.name, a.value))
                        .collect();

                    return attrs;
                }
            }
            Ok(XmlEvent::EndElement { name, .. }) => {
                let element = name.local_name;

                if element_found && element == element_to_find {
                    debug!("END element found: {element}");
                    element_found = false;
                }
            }
            Ok(XmlEvent::Characters(chars)) => {
                if !is_attributes && element_found {
                    debug!("CHARS found: {chars}");
                    result.push(chars);

                    if is_single {
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
