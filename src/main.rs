use log::debug;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    fs::File,
    io::BufReader,
    rc::{Rc, Weak},
};
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
enum ParseEvent {
    Start(String),
    End(String),
}
struct EventSource<T>(EventReader<T>)
where
    T: std::io::Read;

// Translate event source into simpler events for easier testing
impl<T> Iterator for EventSource<T>
where
    T: std::io::Read,
{
    type Item = ParseEvent;

    fn next(&mut self) -> Option<ParseEvent> {
        let event;
        loop {
            match self.0.next() {
                Ok(XmlEvent::StartElement { name, .. }) => {
                    event = Some(ParseEvent::Start(name.local_name));
                    break;
                }
                Ok(XmlEvent::EndElement { name, .. }) => {
                    event = Some(ParseEvent::End(name.local_name));
                    break;
                }
                Ok(XmlEvent::EndDocument) => {
                    event = None;
                    break;
                }
                Err(e) => {
                    eprintln!("{}", e);
                    event = None;
                    break;
                }
                _ => {}
            };
        }

        event
    }
}

#[derive(Debug)]
struct Node {
    children: RefCell<HashMap<String, Rc<Self>>>,
    parent: Option<Weak<Self>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.print(0))
    }
}

// for Display
impl Node {
    fn print(&self, depth: usize) -> String {
        let mut out = "".to_string();

        for (name, node) in self.children.borrow().iter() {
            let indent = "  ".repeat(depth);

            if node.children.borrow().len() < 1 {
                out.push_str(&format!("{}<{} />\n", indent, name))
            } else {
                out.push_str(&format!(
                    "{}<{}>\n{}{}</{}>\n",
                    indent,
                    name,
                    node.print(depth + 1),
                    indent,
                    name
                ));
            }
        }

        out
    }
}

// Main implementation of the thin parsing logic
fn parse<T>(source: &mut T) -> Rc<Node>
where
    T: Iterator<Item = ParseEvent>,
{
    let root = Rc::new(Node {
        children: RefCell::new(HashMap::new()),
        parent: None,
    });
    let mut node = root.clone();

    for e in source {
        match e {
            ParseEvent::Start(name) => {
                // Create a new child if it doesn't exist
                let child = node
                    .children
                    .borrow_mut()
                    .entry(name.clone())
                    .or_insert_with(|| {
                        Rc::new(Node {
                            children: RefCell::new(HashMap::new()),
                            parent: Some(Rc::downgrade(&node)),
                        })
                    })
                    .clone(); // Copy the Rc

                node = child;

                debug!(
                    "> Entering node: {}, ref count: {} strong, {} weak",
                    name,
                    Rc::strong_count(&node),
                    Rc::weak_count(&node)
                );
            }
            ParseEvent::End(name) => {
                debug!(
                    "< Exiting node {} ref count: {} strong, {} weak",
                    name,
                    Rc::strong_count(&node),
                    Rc::weak_count(&node)
                );

                node = node.parent.as_ref().unwrap().upgrade().unwrap();
            }
        }
    }

    root
}

fn main() {
    env_logger::init();

    let file = File::open("sitemap.xml").unwrap();
    let file = BufReader::new(file);
    let parser = EventReader::new(file);
    let mut source = EventSource(parser);

    let tree = parse(&mut source);

    println!("{}", tree);
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::hashmap;

    fn node(children: HashMap<&'static str, Rc<Node>>) -> Rc<Node> {
        Rc::new(Node {
            children: RefCell::new(
                children
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
            ),
            parent: None,
        })
    }

    #[test]
    fn single_node() {
        let mut stream = vec![
            ParseEvent::Start("parent".to_string()),
            ParseEvent::End("parent".to_string()),
        ]
        .into_iter();

        let expected = node(hashmap! {
            "parent" => node(hashmap!{})
        });
        let actual = parse(&mut stream);

        assert_eq!(*actual, *expected);
    }

    #[test]
    fn single_node_with_single_child() {
        let mut stream = vec![
            ParseEvent::Start("parent".to_string()),
            ParseEvent::Start("child".to_string()),
            ParseEvent::End("child".to_string()),
            ParseEvent::End("parent".to_string()),
        ]
        .into_iter();

        let expected = node(hashmap! {
            "parent" =>node(hashmap!{
                "child"=> node(hashmap!{}),
            })
        });

        let actual = parse(&mut stream);

        assert_eq!(actual, expected);
    }

    #[test]
    fn list() {
        let mut stream = vec![
            ParseEvent::Start("parent".to_string()),
            ParseEvent::Start("child".to_string()),
            ParseEvent::Start("grandchild".to_string()),
            ParseEvent::End("grandchild".to_string()),
            ParseEvent::End("child".to_string()),
            ParseEvent::End("parent".to_string()),
        ]
        .into_iter();

        let expected = node(hashmap! {
            "parent" => node(hashmap!{
                "child" => node(hashmap!{
                    "grandchild" => node(hashmap!{}),
                }),
            })
        });

        let actual = parse(&mut stream);

        assert_eq!(actual, expected);
    }

    #[test]
    fn node_with_two_different_children() {
        let mut stream = vec![
            ParseEvent::Start("parent".to_string()),
            ParseEvent::Start("son".to_string()),
            ParseEvent::End("son".to_string()),
            ParseEvent::Start("daughter".to_string()),
            ParseEvent::End("daughter".to_string()),
            ParseEvent::End("parent".to_string()),
        ]
        .into_iter();

        let expected = node(hashmap! {
            "parent" => node(hashmap!{
                "son" => node(hashmap!{}),
                "daughter" => node(hashmap!{})
            })
        });

        let actual = parse(&mut stream);

        assert_eq!(actual, expected);
    }

    #[test]
    fn node_with_uniform_children() {
        let mut stream = vec![
            ParseEvent::Start("parent".to_string()),
            ParseEvent::Start("son".to_string()),
            ParseEvent::End("son".to_string()),
            ParseEvent::Start("son".to_string()),
            ParseEvent::End("son".to_string()),
            ParseEvent::End("parent".to_string()),
        ]
        .into_iter();

        let expected = node(hashmap! {
            "parent" => node(hashmap!{
                "son" => node(hashmap!{})
            })
        });

        let actual = parse(&mut stream);

        assert_eq!(actual, expected);
    }

    #[test]
    fn node_with_uniform_children_and_granchildren() {
        let mut stream = vec![
            ParseEvent::Start("parent".to_string()),
            ParseEvent::Start("son".to_string()),
            ParseEvent::Start("grandson".to_string()),
            ParseEvent::End("grandson".to_string()),
            ParseEvent::End("son".to_string()),
            ParseEvent::Start("son".to_string()),
            ParseEvent::Start("granddaughter".to_string()),
            ParseEvent::End("granddaughter".to_string()),
            ParseEvent::End("son".to_string()),
            ParseEvent::End("parent".to_string()),
        ]
        .into_iter();

        let expected = node(hashmap! {
            "parent" => node(hashmap!{
                "son" => node(hashmap! {
                    "grandson" => node(hashmap!{}),
                    "granddaughter" => node(hashmap!{})
                })
            })
        });

        let actual = parse(&mut stream);

        assert_eq!(actual, expected);
    }

    #[test]
    fn complex_tree() {
        let mut stream = vec![
            ParseEvent::Start("parent".to_string()),
            ParseEvent::Start("son".to_string()),
            ParseEvent::Start("grandson".to_string()),
            ParseEvent::End("grandson".to_string()),
            ParseEvent::Start("granddaughter".to_string()),
            ParseEvent::End("granddaughter".to_string()),
            ParseEvent::End("son".to_string()),
            ParseEvent::Start("son".to_string()),
            ParseEvent::Start("granddaughter".to_string()),
            ParseEvent::End("granddaughter".to_string()),
            ParseEvent::Start("granddaughter".to_string()),
            ParseEvent::End("granddaughter".to_string()),
            ParseEvent::End("son".to_string()),
            ParseEvent::Start("daughter".to_string()),
            ParseEvent::Start("grandson".to_string()),
            ParseEvent::End("grandson".to_string()),
            ParseEvent::Start("grandson".to_string()),
            ParseEvent::End("grandson".to_string()),
            ParseEvent::End("daughter".to_string()),
            ParseEvent::End("parent".to_string()),
        ]
        .into_iter();

        let expected = node(hashmap! {
            "parent" => node(hashmap!{
                "son" => node(hashmap!{
                    "grandson" => node(hashmap!{}),
                    "granddaughter" => node(hashmap!{})
                }),
                "daughter" => node(hashmap!{
                    "grandson" => node(hashmap!{})
                })
            })
        });

        let actual = parse(&mut stream);

        assert_eq!(actual, expected);
    }
}
