use crate::event::EventHandler;
use crate::key::Key;

/// A virtual node: the IR produced by [`crate::rsx!`] and consumed by renderers.
#[derive(Clone)]
pub enum VNode {
    /// A DOM-like element (`div`, `span`, custom elements, ...).
    Element(VElement),
    /// A text node.
    Text(String),
    /// A user component (see [`crate::component`]).
    Component(VComponent),
    /// A group of nodes with no DOM representation of its own.
    Fragment(Vec<VNode>),
    /// A context provider (see [`crate::hooks::Context`]). Transparent to the
    /// DOM; renderers push the value while traversing its children.
    Provider(ProviderNode),
    /// Renders nothing.
    Empty,
}

/// A context provider node.
#[derive(Clone)]
pub struct ProviderNode {
    pub ctx_id: u64,
    pub value: std::rc::Rc<dyn std::any::Any>,
    pub children: Vec<VNode>,
}

/// An element node: tag name, attributes, listeners, children and key.
#[derive(Clone)]
pub struct VElement {
    pub tag: String,
    pub attrs: Vec<(String, AttrValue)>,
    pub listeners: Vec<(String, EventHandler)>,
    pub children: Vec<VNode>,
    pub key: Option<Key>,
}

/// A typed attribute value.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    Str(String),
    Bool(bool),
    Int(i64),
    Float(f64),
}

impl std::fmt::Display for AttrValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttrValue::Str(s) => write!(f, "{}", s),
            AttrValue::Bool(b) => write!(f, "{}", b),
            AttrValue::Int(i) => write!(f, "{}", i),
            AttrValue::Float(x) => write!(f, "{}", x),
        }
    }
}

impl From<&str> for AttrValue {
    fn from(v: &str) -> Self {
        AttrValue::Str(v.to_string())
    }
}
impl From<String> for AttrValue {
    fn from(v: String) -> Self {
        AttrValue::Str(v)
    }
}
impl From<&String> for AttrValue {
    fn from(v: &String) -> Self {
        AttrValue::Str(v.clone())
    }
}
impl From<bool> for AttrValue {
    fn from(v: bool) -> Self {
        AttrValue::Bool(v)
    }
}
impl From<i32> for AttrValue {
    fn from(v: i32) -> Self {
        AttrValue::Int(v as i64)
    }
}
impl From<u32> for AttrValue {
    fn from(v: u32) -> Self {
        AttrValue::Int(v as i64)
    }
}
impl From<u64> for AttrValue {
    fn from(v: u64) -> Self {
        AttrValue::Int(v as i64)
    }
}
impl From<i64> for AttrValue {
    fn from(v: i64) -> Self {
        AttrValue::Int(v)
    }
}
impl From<usize> for AttrValue {
    fn from(v: usize) -> Self {
        AttrValue::Int(v as i64)
    }
}
impl From<f64> for AttrValue {
    fn from(v: f64) -> Self {
        AttrValue::Float(v)
    }
}

/// A user component instance: name, key and type-erased props.
#[derive(Clone)]
pub struct VComponent {
    pub name: &'static str,
    pub key: Option<Key>,
    pub props: std::rc::Rc<dyn ComponentProps>,
}

impl std::fmt::Debug for VNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VNode::Element(e) => write!(f, "Element({:?})", e),
            VNode::Text(t) => write!(f, "Text({:?})", t),
            VNode::Component(c) => write!(f, "Component({:?})", c),
            VNode::Fragment(ch) => write!(f, "Fragment({:?})", ch),
            VNode::Provider(p) => write!(f, "Provider({:?})", p.children),
            VNode::Empty => write!(f, "Empty"),
        }
    }
}

impl std::fmt::Debug for VElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let listener_names: Vec<&str> = self.listeners.iter().map(|(n, _)| n.as_str()).collect();
        f.debug_struct("VElement")
            .field("tag", &self.tag)
            .field("attrs", &self.attrs)
            .field("listeners", &listener_names)
            .field("children", &self.children)
            .field("key", &self.key)
            .finish()
    }
}

impl std::fmt::Debug for VComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponent")
            .field("name", &self.name)
            .field("key", &self.key)
            .field("props", &"<dyn ComponentProps>")
            .finish()
    }
}

/// Implemented by `#[component]` structs; renders the component body.
pub trait ComponentProps {
    fn render(&self) -> VNode;
}

impl VNode {
    pub fn element(tag: impl Into<String>) -> VElement {
        VElement {
            tag: tag.into(),
            attrs: Vec::new(),
            listeners: Vec::new(),
            children: Vec::new(),
            key: None,
        }
    }

    pub fn text(s: impl Into<String>) -> VNode {
        VNode::Text(s.into())
    }

    pub fn fragment(children: Vec<VNode>) -> VNode {
        VNode::Fragment(children)
    }

    pub fn component(
        name: &'static str,
        key: Option<Key>,
        props: impl ComponentProps + 'static,
    ) -> VNode {
        VNode::Component(VComponent {
            name,
            key,
            props: std::rc::Rc::new(props),
        })
    }

    /// Stable key of this node, if any (elements and components only).
    pub fn key(&self) -> Option<Key> {
        match self {
            VNode::Element(e) => e.key,
            VNode::Component(c) => c.key,
            _ => None,
        }
    }
}

impl From<&str> for VNode {
    fn from(v: &str) -> Self {
        VNode::Text(v.to_string())
    }
}
impl From<String> for VNode {
    fn from(v: String) -> Self {
        VNode::Text(v)
    }
}
impl From<&String> for VNode {
    fn from(v: &String) -> Self {
        VNode::Text(v.clone())
    }
}
impl From<i32> for VNode {
    fn from(v: i32) -> Self {
        VNode::Text(v.to_string())
    }
}
impl From<u32> for VNode {
    fn from(v: u32) -> Self {
        VNode::Text(v.to_string())
    }
}
impl From<i64> for VNode {
    fn from(v: i64) -> Self {
        VNode::Text(v.to_string())
    }
}
impl From<usize> for VNode {
    fn from(v: usize) -> Self {
        VNode::Text(v.to_string())
    }
}
impl From<bool> for VNode {
    fn from(v: bool) -> Self {
        VNode::Text(v.to_string())
    }
}
impl From<f64> for VNode {
    fn from(v: f64) -> Self {
        VNode::Text(v.to_string())
    }
}
impl From<Vec<VNode>> for VNode {
    fn from(v: Vec<VNode>) -> Self {
        VNode::Fragment(v)
    }
}
impl From<&Vec<VNode>> for VNode {
    fn from(v: &Vec<VNode>) -> Self {
        VNode::Fragment(v.clone())
    }
}
impl From<VElement> for VNode {
    fn from(v: VElement) -> Self {
        VNode::Element(v)
    }
}