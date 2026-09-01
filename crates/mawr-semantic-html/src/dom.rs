use std::borrow::Cow;
use std::cell::{Ref, RefCell};
use std::ops::Deref;

use ego_tree::{NodeId, NodeRef, Tree};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeBuilderOpts, TreeSink};
use html5ever::{Attribute, ParseOpts, QualName, expanded_name, local_name, ns};

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Node {
    Document,
    Fragment,
    Doctype,
    Comment,
    Text(StrTendril),
    Element(Element),
    ProcessingInstruction,
}

impl Node {
    pub(crate) fn is_element(&self) -> bool {
        matches!(self, Self::Element(_))
    }

    pub(crate) fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn as_element(&self) -> Option<&Element> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Document => formatter.write_str("Document"),
            Self::Fragment => formatter.write_str("Fragment"),
            Self::Doctype => formatter.write_str("Doctype"),
            Self::Comment => formatter.write_str("Comment"),
            Self::Text(_) => formatter.write_str("Text(<web-content>)"),
            Self::Element(element) => formatter
                .debug_tuple("Element")
                .field(&element.name())
                .finish(),
            Self::ProcessingInstruction => formatter.write_str("ProcessingInstruction"),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Element {
    pub(crate) name: QualName,
    pub(crate) attrs: Vec<(QualName, StrTendril)>,
}

impl Element {
    fn new(name: QualName, attrs: Vec<Attribute>) -> Self {
        let mut attrs = attrs
            .into_iter()
            .map(|attribute| (attribute.name, attribute.value))
            .collect::<Vec<_>>();
        attrs.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        Self { name, attrs }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name.local
    }

    pub(crate) fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(attribute, _)| attribute.ns.is_empty() && &*attribute.local == name)
            .map(|(_, value)| &**value)
    }

    pub(crate) fn id(&self) -> Option<&str> {
        self.attr("id")
    }
}

pub(crate) struct Html {
    pub(crate) tree: Tree<Node>,
    quirks_mode: QuirksMode,
}

impl Html {
    pub(crate) fn parse_document(source: &str) -> Self {
        html5ever::parse_document(
            HtmlSink::new(Self {
                tree: Tree::new(Node::Document),
                quirks_mode: QuirksMode::NoQuirks,
            }),
            ParseOpts {
                tree_builder: TreeBuilderOpts {
                    scripting_enabled: false,
                    ..TreeBuilderOpts::default()
                },
                ..ParseOpts::default()
            },
        )
        .one(source)
    }

    pub(crate) fn root_element(&self) -> ElementRef<'_> {
        self.tree
            .root()
            .children()
            .find_map(ElementRef::wrap)
            .expect("html5ever creates an html element")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ElementRef<'a>(NodeRef<'a, Node>);

impl<'a> ElementRef<'a> {
    pub(crate) fn wrap(node: NodeRef<'a, Node>) -> Option<Self> {
        node.value().is_element().then_some(Self(node))
    }

    pub(crate) fn value(self) -> &'a Element {
        self.0.value().as_element().expect("element reference")
    }

    pub(crate) fn attr(self, name: &str) -> Option<&'a str> {
        self.value().attr(name)
    }

    pub(crate) fn child_elements(self) -> impl Iterator<Item = Self> + 'a {
        self.children().filter_map(Self::wrap)
    }

    pub(crate) fn descendent_elements(self) -> impl Iterator<Item = Self> + 'a {
        self.descendants().filter_map(Self::wrap)
    }
}

impl<'a> Deref for ElementRef<'a> {
    type Target = NodeRef<'a, Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct HtmlSink(RefCell<Html>);

impl HtmlSink {
    fn new(html: Html) -> Self {
        Self(RefCell::new(html))
    }
}

impl TreeSink for HtmlSink {
    type Output = Html;
    type Handle = NodeId;
    type ElemName<'a> = Ref<'a, QualName>;

    fn finish(self) -> Self::Output {
        self.0.into_inner()
    }

    fn parse_error(&self, _message: Cow<'static, str>) {}

    fn get_document(&self) -> Self::Handle {
        self.0.borrow().tree.root().id()
    }

    fn elem_name<'a>(&'a self, target: &Self::Handle) -> Self::ElemName<'a> {
        Ref::map(self.0.borrow(), |html| {
            &html
                .tree
                .get(*target)
                .expect("parser handle belongs to tree")
                .value()
                .as_element()
                .expect("parser requested an element name")
                .name
        })
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        let is_template = name.expanded() == expanded_name!(html "template");
        let mut html = self.0.borrow_mut();
        let mut element = html.tree.orphan(Node::Element(Element::new(name, attrs)));
        if is_template {
            element.append(Node::Fragment);
        }
        element.id()
    }

    fn create_comment(&self, _text: StrTendril) -> Self::Handle {
        self.0.borrow_mut().tree.orphan(Node::Comment).id()
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
        self.0
            .borrow_mut()
            .tree
            .orphan(Node::ProcessingInstruction)
            .id()
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        self.0.borrow_mut().tree.root_mut().append(Node::Doctype);
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let mut html = self.0.borrow_mut();
        let mut parent = html.tree.get_mut(*parent).expect("parser parent handle");
        match child {
            NodeOrText::AppendNode(child) => {
                parent.append_id(child);
            }
            NodeOrText::AppendText(text) => {
                let merged = parent.last_child().is_some_and(|mut child| {
                    if let Node::Text(existing) = child.value() {
                        existing.push_tendril(&text);
                        true
                    } else {
                        false
                    }
                });
                if !merged {
                    parent.append(Node::Text(text));
                }
            }
        }
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let mut html = self.0.borrow_mut();
        if let NodeOrText::AppendNode(child) = child {
            html.tree
                .get_mut(child)
                .expect("parser child handle")
                .detach();
        }
        let mut sibling = html.tree.get_mut(*sibling).expect("parser sibling handle");
        if sibling.parent().is_none() {
            return;
        }
        match child {
            NodeOrText::AppendNode(child) => {
                sibling.insert_id_before(child);
            }
            NodeOrText::AppendText(text) => {
                let merged = sibling.prev_sibling().is_some_and(|mut previous| {
                    if let Node::Text(existing) = previous.value() {
                        existing.push_tendril(&text);
                        true
                    } else {
                        false
                    }
                });
                if !merged {
                    sibling.insert_before(Node::Text(text));
                }
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        previous_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        let attached = self
            .0
            .borrow()
            .tree
            .get(*element)
            .expect("parser element handle")
            .parent()
            .is_some();
        if attached {
            self.append_before_sibling(element, child);
        } else {
            self.append(previous_element, child);
        }
    }

    fn same_node(&self, left: &Self::Handle, right: &Self::Handle) -> bool {
        left == right
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.0.borrow_mut().quirks_mode = mode;
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        let mut html = self.0.borrow_mut();
        let mut node = html.tree.get_mut(*target).expect("parser element handle");
        let Node::Element(element) = node.value() else {
            panic!("parser attribute target is not an element");
        };
        for attribute in attrs {
            if let Err(index) = element
                .attrs
                .binary_search_by(|(name, _)| name.cmp(&attribute.name))
            {
                element
                    .attrs
                    .insert(index, (attribute.name, attribute.value));
            }
        }
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        self.0
            .borrow_mut()
            .tree
            .get_mut(*target)
            .expect("parser target handle")
            .detach();
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        self.0
            .borrow_mut()
            .tree
            .get_mut(*new_parent)
            .expect("parser parent handle")
            .reparent_from_id_append(*node);
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        self.0
            .borrow()
            .tree
            .get(*target)
            .expect("parser template handle")
            .first_child()
            .expect("template fragment")
            .id()
    }

    fn mark_script_already_started(&self, _node: &Self::Handle) {}
}
