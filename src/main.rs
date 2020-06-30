use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use xml::reader::{EventReader, XmlEvent};

// debugging helper ignore this
fn indent(size: usize) -> String {
    const INDENT: &'static str = "  ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

#[derive(Debug)]
struct Node {
    name: String,
    depth: u32,
    children: HashMap<String, Self>,
}

impl Node {
    fn new(name: String, depth: u32) -> Self {
        Self {
            name,
            depth,
            children: HashMap::new(),
        }
    }

    fn create_child(&mut self, name: String, depth: u32) -> &mut Self {
        self.children
            .entry(name.clone())
            .or_insert_with(|| Self::new(name.clone(), depth))
    }
}

/**
 * Parse a XML file
 * Find all unique keys and create a tree of Nodes with a root out of them
 *
 * I use Stack(Vec) to track the current element with it's level/depth
 * And create children while I traverse the XML file
 * The problem is the double mutable borrow when I try to push new Node onto the node_stack
 **/
fn main() {
    let file = File::open("sitemap.xml").unwrap();
    let file = BufReader::new(file);

    let mut root = Node::new(String::from("ROOT"), 0);
    let mut node_stack = vec![&mut root];

    let parser = EventReader::new(file);
    let mut depth = 0;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                let next_node = node_stack
                    .last_mut()
                    .expect("Root is missing. This should not happen")
                    .create_child(name.local_name, depth as u32);

                node_stack.push(next_node);

                depth += 1;
            }
            Ok(XmlEvent::EndElement { .. }) => {
                if depth > 1 {
                    depth -= 1;
                    node_stack.pop();
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }
    println!("{:#?}", node_stack.pop().expect("Node stack is empty"));
}
