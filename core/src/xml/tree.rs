//! XML Tree structure

use crate::avm1::xml_object::XMLObject;
use crate::avm1::Object;
use crate::xml;
use crate::xml::{Error, XMLDocument, XMLName};
use gc_arena::{Collect, GcCell, MutationContext};
use quick_xml::events::{BytesStart, BytesText};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::mem::swap;

/// Represents a node in the XML tree.
#[derive(Copy, Clone, Collect)]
#[collect(no_drop)]
pub struct XMLNode<'gc>(GcCell<'gc, XMLNodeData<'gc>>);

#[derive(Clone, Collect)]
#[collect(no_drop)]
pub enum XMLNodeData<'gc> {
    /// A text node in the XML tree.
    Text {
        /// The script object associated with this XML node, if any.
        script_object: Option<Object<'gc>>,

        /// The document that this tree node currently belongs to.
        document: XMLDocument<'gc>,

        /// The parent node of this one.
        parent: Option<XMLNode<'gc>>,

        /// The previous sibling node to this one.
        prev_sibling: Option<XMLNode<'gc>>,

        /// The next sibling node to this one.
        next_sibling: Option<XMLNode<'gc>>,

        /// The string representation of the text.
        contents: String,
    },

    /// A comment node in the XML tree.
    Comment {
        /// The script object associated with this XML node, if any.
        script_object: Option<Object<'gc>>,

        /// The document that this tree node currently belongs to.
        document: XMLDocument<'gc>,

        /// The parent node of this one.
        parent: Option<XMLNode<'gc>>,

        /// The previous sibling node to this one.
        prev_sibling: Option<XMLNode<'gc>>,

        /// The next sibling node to this one.
        next_sibling: Option<XMLNode<'gc>>,

        /// The string representation of the comment.
        contents: String,
    },

    /// An element node in the XML tree.
    ///
    /// Element nodes are non-leaf nodes: they can store additional data as
    /// either attributes (for key/value pairs) or child nodes (for more
    /// structured data).
    Element {
        /// The script object associated with this XML node, if any.
        script_object: Option<Object<'gc>>,

        /// The document that this tree node currently belongs to.
        document: XMLDocument<'gc>,

        /// The parent node of this one.
        parent: Option<XMLNode<'gc>>,

        /// The previous sibling node to this one.
        prev_sibling: Option<XMLNode<'gc>>,

        /// The next sibling node to this one.
        next_sibling: Option<XMLNode<'gc>>,

        /// The tag name of this element.
        tag_name: XMLName,

        /// Attributes of the element.
        attributes: BTreeMap<XMLName, String>,

        /// Child nodes of this element.
        children: Vec<XMLNode<'gc>>,
    },

    /// The root level of an XML document. Has no parent.
    DocumentRoot {
        /// The script object associated with this XML node, if any.
        script_object: Option<Object<'gc>>,

        /// The document that this is the root of.
        document: XMLDocument<'gc>,

        /// Child nodes of this element.
        children: Vec<XMLNode<'gc>>,
    },
}

impl<'gc> XMLNode<'gc> {
    /// Construct a new XML text node.
    pub fn new_text(
        mc: MutationContext<'gc, '_>,
        contents: &str,
        document: XMLDocument<'gc>,
    ) -> Self {
        XMLNode(GcCell::allocate(
            mc,
            XMLNodeData::Text {
                script_object: None,
                document,
                parent: None,
                prev_sibling: None,
                next_sibling: None,
                contents: contents.to_string(),
            },
        ))
    }

    /// Construct a new XML element node.
    pub fn new_element(
        mc: MutationContext<'gc, '_>,
        element_name: &str,
        document: XMLDocument<'gc>,
    ) -> Result<Self, Error> {
        Ok(XMLNode(GcCell::allocate(
            mc,
            XMLNodeData::Element {
                script_object: None,
                document,
                parent: None,
                prev_sibling: None,
                next_sibling: None,
                tag_name: XMLName::from_str(element_name)?,
                attributes: BTreeMap::new(),
                children: Vec::new(),
            },
        )))
    }

    /// Construct a new XML root node.
    pub fn new_document_root(mc: MutationContext<'gc, '_>, document: XMLDocument<'gc>) -> Self {
        XMLNode(GcCell::allocate(
            mc,
            XMLNodeData::DocumentRoot {
                script_object: None,
                document,
                children: Vec::new(),
            },
        ))
    }

    /// Construct an XML Element node from a `quick_xml` `BytesStart` event.
    ///
    /// The returned node will always be an `Element`, and it must only contain
    /// valid encoded UTF-8 data. (Other encoding support is planned later.)
    pub fn from_start_event<'a>(
        mc: MutationContext<'gc, '_>,
        bs: BytesStart<'a>,
        document: XMLDocument<'gc>,
    ) -> Result<Self, Error> {
        let tag_name = XMLName::from_bytes_cow(bs.unescaped()?)?;
        let mut attributes = BTreeMap::new();

        for a in bs.attributes() {
            let attribute = a?;
            attributes.insert(
                XMLName::from_bytes(attribute.key)?,
                String::from_utf8(attribute.value.to_owned().to_vec())?,
            );
        }

        let children = Vec::new();

        Ok(XMLNode(GcCell::allocate(
            mc,
            XMLNodeData::Element {
                script_object: None,
                document,
                parent: None,
                prev_sibling: None,
                next_sibling: None,
                tag_name,
                attributes,
                children,
            },
        )))
    }

    /// Construct an XML Text node from a `quick_xml` `BytesText` event.
    ///
    /// The returned node will always be `Text`, and it must only contain
    /// valid encoded UTF-8 data. (Other encoding support is planned later.)
    pub fn text_from_text_event<'a>(
        mc: MutationContext<'gc, '_>,
        bt: BytesText<'a>,
        document: XMLDocument<'gc>,
    ) -> Result<Self, Error> {
        Ok(XMLNode(GcCell::allocate(
            mc,
            XMLNodeData::Text {
                script_object: None,
                document,
                parent: None,
                prev_sibling: None,
                next_sibling: None,
                contents: match bt.unescaped()? {
                    Cow::Borrowed(ln) => Cow::Borrowed(std::str::from_utf8(ln)?),
                    Cow::Owned(ln) => Cow::Owned(String::from_utf8(ln)?),
                }
                .to_owned()
                .to_string(),
            },
        )))
    }

    /// Construct an XML Comment node from a `quick_xml` `BytesText` event.
    ///
    /// The returned node will always be `Comment`, and it must only contain
    /// valid encoded UTF-8 data. (Other encoding support is planned later.)
    pub fn comment_from_text_event<'a>(
        mc: MutationContext<'gc, '_>,
        bt: BytesText<'a>,
        document: XMLDocument<'gc>,
    ) -> Result<Self, Error> {
        Ok(XMLNode(GcCell::allocate(
            mc,
            XMLNodeData::Comment {
                script_object: None,
                document,
                parent: None,
                prev_sibling: None,
                next_sibling: None,
                contents: match bt.unescaped()? {
                    Cow::Borrowed(ln) => Cow::Borrowed(std::str::from_utf8(ln)?),
                    Cow::Owned(ln) => Cow::Owned(String::from_utf8(ln)?),
                }
                .to_owned()
                .to_string(),
            },
        )))
    }

    /// Return the XML document that this tree node belongs to.
    ///
    /// Every XML node belongs to a document object (see `XMLDocument`) which
    /// stores global information about the document, such as namespace URIs.
    pub fn document(self) -> XMLDocument<'gc> {
        match &*self.0.read() {
            XMLNodeData::Text { document, .. } => *document,
            XMLNodeData::Comment { document, .. } => *document,
            XMLNodeData::Element { document, .. } => *document,
            XMLNodeData::DocumentRoot { document, .. } => *document,
        }
    }

    /// Adopt a new child node into the current node.
    ///
    /// This does not add the node to any internal lists; it merely updates the
    /// child to ensure that it considers this node it's parent. This function
    /// should always be called after a child node is added to this one. If
    /// you adopt a node that is NOT already added to the children list, bad
    /// things may happen.
    ///
    /// The `new_child_position` parameter is the position of the new child in
    /// this node's child list. This is used to find and link the child's
    /// siblings to each other.
    fn adopt_child(
        &mut self,
        mc: MutationContext<'gc, '_>,
        mut child: XMLNode<'gc>,
        new_child_position: usize,
    ) -> Result<(), Error> {
        if GcCell::ptr_eq(self.0, child.0) {
            return Err("Cannot adopt child into itself".into());
        }

        match &mut *self.0.write(mc) {
            XMLNodeData::Element {
                document, children, ..
            }
            | XMLNodeData::DocumentRoot {
                document, children, ..
            } => {
                {
                    let mut write = child.0.write(mc);
                    let (child_document, child_parent) = match &mut *write {
                        XMLNodeData::Element {
                            document, parent, ..
                        } => Ok((document, parent)),
                        XMLNodeData::Text {
                            document, parent, ..
                        } => Ok((document, parent)),
                        XMLNodeData::Comment {
                            document, parent, ..
                        } => Ok((document, parent)),
                        XMLNodeData::DocumentRoot { .. } => {
                            Err("Cannot adopt other document roots")
                        }
                    }?;

                    if let Some(parent) = child_parent {
                        parent.orphan_child(mc, child)?;
                    }

                    *child_document = *document;
                    *child_parent = Some(*self);
                }
                child.disown_siblings(mc)?;

                let new_prev = new_child_position
                    .checked_sub(1)
                    .and_then(|p| children.get(p).cloned());
                let new_next = new_child_position
                    .checked_add(1)
                    .and_then(|p| children.get(p).cloned());

                child.adopt_siblings(mc, new_prev, new_next)?
            }
            _ => return Err("Cannot adopt children into non-child-bearing node".into()),
        }

        Ok(())
    }

    /// Get the parent, if this node has one.
    ///
    /// If the node cannot have a parent, then this function yields Err.
    pub fn parent(self) -> Result<Option<XMLNode<'gc>>, Error> {
        match *self.0.read() {
            XMLNodeData::Element { parent, .. } => Ok(parent),
            XMLNodeData::Text { parent, .. } => Ok(parent),
            XMLNodeData::Comment { parent, .. } => Ok(parent),
            XMLNodeData::DocumentRoot { .. } => Err("Document roots cannot have parents".into()),
        }
    }

    /// Get the previous sibling, if this node has one.
    ///
    /// If the node cannot have siblings, then this function yields Err.
    pub fn prev_sibling(self) -> Result<Option<XMLNode<'gc>>, Error> {
        match *self.0.read() {
            XMLNodeData::Element { prev_sibling, .. } => Ok(prev_sibling),
            XMLNodeData::Text { prev_sibling, .. } => Ok(prev_sibling),
            XMLNodeData::Comment { prev_sibling, .. } => Ok(prev_sibling),
            XMLNodeData::DocumentRoot { .. } => Err("Document roots cannot have siblings".into()),
        }
    }

    /// Set this node's previous sibling.
    fn set_prev_sibling(
        &mut self,
        mc: MutationContext<'gc, '_>,
        new_prev: Option<XMLNode<'gc>>,
    ) -> Result<(), Error> {
        match &mut *self.0.write(mc) {
            XMLNodeData::Element { prev_sibling, .. } => *prev_sibling = new_prev,
            XMLNodeData::Text { prev_sibling, .. } => *prev_sibling = new_prev,
            XMLNodeData::Comment { prev_sibling, .. } => *prev_sibling = new_prev,
            XMLNodeData::DocumentRoot { .. } => {
                return Err("Document roots cannot have siblings".into())
            }
        };

        Ok(())
    }

    /// Get the next sibling, if this node has one.
    ///
    /// If the node cannot have siblings, then this function yields Err.
    pub fn next_sibling(self) -> Result<Option<XMLNode<'gc>>, Error> {
        match *self.0.read() {
            XMLNodeData::Element { next_sibling, .. } => Ok(next_sibling),
            XMLNodeData::Text { next_sibling, .. } => Ok(next_sibling),
            XMLNodeData::Comment { next_sibling, .. } => Ok(next_sibling),
            XMLNodeData::DocumentRoot { .. } => Err("Document roots cannot have siblings".into()),
        }
    }

    /// Set this node's next sibling.
    fn set_next_sibling(
        &mut self,
        mc: MutationContext<'gc, '_>,
        new_next: Option<XMLNode<'gc>>,
    ) -> Result<(), Error> {
        match &mut *self.0.write(mc) {
            XMLNodeData::Element { next_sibling, .. } => *next_sibling = new_next,
            XMLNodeData::Text { next_sibling, .. } => *next_sibling = new_next,
            XMLNodeData::Comment { next_sibling, .. } => *next_sibling = new_next,
            XMLNodeData::DocumentRoot { .. } => {
                return Err("Document roots cannot have siblings".into())
            }
        };

        Ok(())
    }

    /// Remove node from it's current siblings list.
    ///
    /// If a former sibling exists, we will also adopt it to the opposing side
    /// of this node, so as to maintain a coherent sibling list.
    ///
    /// This is the opposite of `adopt_siblings` - the former adds a node to a
    /// new sibling list, and this removes it from the current one.
    fn disown_siblings(&mut self, mc: MutationContext<'gc, '_>) -> Result<(), Error> {
        let old_prev = self.prev_sibling()?;
        let old_next = self.next_sibling()?;

        if let Some(mut prev) = old_prev {
            prev.set_next_sibling(mc, old_next)?;
        }

        if let Some(mut next) = old_next {
            next.set_prev_sibling(mc, old_prev)?;
        }

        self.set_prev_sibling(mc, None)?;
        self.set_next_sibling(mc, None)?;

        Ok(())
    }

    /// Add node to a new siblings list.
    ///
    /// If a given sibling exists, we will also ensure this node is adopted as
    /// it's sibling, so as to maintain a coherent sibling list.
    ///
    /// This is the opposite of `disown_siblings` - the former removes a
    /// sibling from it's current list, and this adds the sibling to a new one.
    fn adopt_siblings(
        &mut self,
        mc: MutationContext<'gc, '_>,
        new_prev: Option<XMLNode<'gc>>,
        new_next: Option<XMLNode<'gc>>,
    ) -> Result<(), Error> {
        if let Some(mut prev) = new_prev {
            prev.set_next_sibling(mc, Some(*self))?;
        }

        if let Some(mut next) = new_next {
            next.set_prev_sibling(mc, Some(*self))?;
        }

        self.set_prev_sibling(mc, new_prev)?;
        self.set_next_sibling(mc, new_next)?;

        Ok(())
    }

    /// Remove node from this node's child list.
    fn orphan_child(
        &mut self,
        mc: MutationContext<'gc, '_>,
        child: XMLNode<'gc>,
    ) -> Result<(), Error> {
        for (i, other_child) in self
            .children()
            .ok_or("Cannot orphan child if I have no children")?
            .enumerate()
        {
            if GcCell::ptr_eq(child.0, other_child.0) {
                match &mut *self.0.write(mc) {
                    XMLNodeData::Element { children, .. } => children.remove(i),
                    XMLNodeData::DocumentRoot { children, .. } => children.remove(i),
                    XMLNodeData::Text { .. } => return Err("Text node has no child nodes!".into()),
                    XMLNodeData::Comment { .. } => {
                        return Err("Comment node has no child nodes!".into())
                    }
                };

                break;
            }
        }

        Ok(())
    }

    /// Append a child element to an Element node.
    ///
    /// The child will be adopted into the current tree: all child references
    /// to other nodes or documents will be adjusted to reflect it's new
    /// position in the tree. This may remove it from any existing trees or
    /// documents.
    ///
    /// This function yields an error if appending to a Node that cannot accept
    /// children. In that case, no modification will be made to the node.
    pub fn append_child(
        &mut self,
        mc: MutationContext<'gc, '_>,
        child: XMLNode<'gc>,
    ) -> Result<(), Error> {
        let position = match &mut *self.0.write(mc) {
            XMLNodeData::Element {
                ref mut children, ..
            }
            | XMLNodeData::DocumentRoot {
                ref mut children, ..
            } => {
                children.push(child);
                children.len() - 1
            }
            _ => return Err("Not an Element".into()),
        };

        self.adopt_child(mc, child, position)?;

        Ok(())
    }

    /// Returns the type of this node as an integer.
    ///
    /// This is primarily intended to match W3C DOM L1 specifications and
    /// should not be used in lieu of a proper `match` statement.
    pub fn node_type(self) -> u8 {
        match &*self.0.read() {
            XMLNodeData::Element { .. } => xml::ELEMENT_NODE,
            XMLNodeData::DocumentRoot { .. } => xml::ELEMENT_NODE,
            XMLNodeData::Text { .. } => xml::TEXT_NODE,
            XMLNodeData::Comment { .. } => xml::COMMENT_NODE,
        }
    }

    /// Returns the tagname, if the element has one.
    pub fn tag_name(self) -> Option<XMLName> {
        match &*self.0.read() {
            XMLNodeData::Element { ref tag_name, .. } => Some(tag_name.clone()),
            _ => None,
        }
    }

    /// Returns the string contents of the node, if the element has them.
    pub fn node_value(self) -> Option<String> {
        match &*self.0.read() {
            XMLNodeData::Text { ref contents, .. } => Some(contents.clone()),
            XMLNodeData::Comment { ref contents, .. } => Some(contents.clone()),
            _ => None,
        }
    }

    /// Returns the number of children of the current tree node.
    ///
    /// Nodes that cannot hold children always yield `0`.
    pub fn children_len(self) -> usize {
        match &*self.0.read() {
            XMLNodeData::Element { children, .. } | XMLNodeData::DocumentRoot { children, .. } => {
                children.len()
            }
            _ => 0,
        }
    }

    /// Returns an iterator that yields child nodes.
    ///
    /// Yields None if this node cannot accept children.
    pub fn children(self) -> Option<impl DoubleEndedIterator<Item = XMLNode<'gc>>> {
        struct ChildIter<'gc> {
            base: XMLNode<'gc>,
            index: usize,
            back_index: usize,
        };

        impl<'gc> ChildIter<'gc> {
            fn for_node(base: XMLNode<'gc>) -> Self {
                Self {
                    base,
                    index: 0,
                    back_index: base.children_len(),
                }
            }
        }

        impl<'gc> Iterator for ChildIter<'gc> {
            type Item = XMLNode<'gc>;

            fn next(&mut self) -> Option<Self::Item> {
                let read = self.base.0.read();
                let children = match &*read {
                    XMLNodeData::Element { children, .. }
                    | XMLNodeData::DocumentRoot { children, .. } => Some(children),
                    _ => None,
                };

                if let Some(children) = children {
                    if self.index < self.back_index {
                        let item = children.get(self.index).cloned();
                        self.index += 1;

                        return item;
                    }
                }

                None
            }
        }

        impl<'gc> DoubleEndedIterator for ChildIter<'gc> {
            fn next_back(&mut self) -> Option<Self::Item> {
                let read = self.base.0.read();
                let children = match &*read {
                    XMLNodeData::Element { children, .. }
                    | XMLNodeData::DocumentRoot { children, .. } => Some(children),
                    _ => None,
                };

                if let Some(children) = children {
                    if self.index < self.back_index {
                        self.back_index -= 1;
                        let item = children.get(self.back_index).cloned();

                        return item;
                    }
                }

                None
            }
        }

        match &*self.0.read() {
            XMLNodeData::Element { .. } | XMLNodeData::DocumentRoot { .. } => {
                Some(ChildIter::for_node(self))
            }
            _ => None,
        }
    }

    /// Get the already-instantiated script object from the current node.
    fn get_script_object(self) -> Option<Object<'gc>> {
        match &*self.0.read() {
            XMLNodeData::Element { script_object, .. } => *script_object,
            XMLNodeData::Text { script_object, .. } => *script_object,
            XMLNodeData::Comment { script_object, .. } => *script_object,
            XMLNodeData::DocumentRoot { script_object, .. } => *script_object,
        }
    }

    /// Introduce this node to a new script object.
    ///
    /// This internal function *will* overwrite already extant objects, so only
    /// call this if you need to instantiate the script object for the first
    /// time.
    pub fn introduce_script_object(
        &mut self,
        gc_context: MutationContext<'gc, '_>,
        new_object: Object<'gc>,
    ) {
        match &mut *self.0.write(gc_context) {
            XMLNodeData::Element { script_object, .. } => *script_object = Some(new_object),
            XMLNodeData::Text { script_object, .. } => *script_object = Some(new_object),
            XMLNodeData::Comment { script_object, .. } => *script_object = Some(new_object),
            XMLNodeData::DocumentRoot { script_object, .. } => *script_object = Some(new_object),
        }
    }

    /// Obtain the script object for a given XML tree node, constructing a new
    /// script object if one does not exist.
    pub fn script_object(
        &mut self,
        gc_context: MutationContext<'gc, '_>,
        prototype: Option<Object<'gc>>,
    ) -> Object<'gc> {
        let mut object = self.get_script_object();
        if object.is_none() {
            object = Some(XMLObject::from_xml_node(gc_context, *self, prototype));
            self.introduce_script_object(gc_context, object.unwrap());
        }

        object.unwrap()
    }

    /// Swap the contents of this node with another one.
    ///
    /// After this function completes, the current `XMLNode` will contain all
    /// data present in the `other` node, and vice versa. References to the node
    /// within the tree will *not* be updated.
    pub fn swap(&mut self, gc_context: MutationContext<'gc, '_>, other: Self) {
        if !GcCell::ptr_eq(self.0, other.0) {
            swap(
                &mut *self.0.write(gc_context),
                &mut *other.0.write(gc_context),
            );
        }
    }

    /// Check if this XML node constitutes the root of a whole document.
    pub fn is_document_root(self) -> bool {
        match &*self.0.read() {
            XMLNodeData::DocumentRoot { .. } => true,
            _ => false,
        }
    }

    /// Create a duplicate copy of this node.
    ///
    /// If the `deep` flag is set true, then the entire node tree will be
    /// cloned.
    pub fn duplicate(self, gc_context: MutationContext<'gc, '_>, deep: bool) -> XMLNode<'gc> {
        let mut document = self.document().duplicate(gc_context);
        let mut clone = XMLNode(GcCell::allocate(
            gc_context,
            match &*self.0.read() {
                XMLNodeData::Text { contents, .. } => XMLNodeData::Text {
                    script_object: None,
                    document,
                    parent: None,
                    prev_sibling: None,
                    next_sibling: None,
                    contents: contents.to_string(),
                },
                XMLNodeData::Comment { contents, .. } => XMLNodeData::Comment {
                    script_object: None,
                    document,
                    parent: None,
                    prev_sibling: None,
                    next_sibling: None,
                    contents: contents.to_string(),
                },
                XMLNodeData::Element {
                    tag_name,
                    attributes,
                    ..
                } => XMLNodeData::Element {
                    script_object: None,
                    document,
                    parent: None,
                    prev_sibling: None,
                    next_sibling: None,
                    tag_name: tag_name.clone(),
                    attributes: attributes.clone(),
                    children: Vec::new(),
                },
                XMLNodeData::DocumentRoot { .. } => XMLNodeData::DocumentRoot {
                    script_object: None,
                    document,
                    children: Vec::new(),
                },
            },
        ));

        document.link_root_node(gc_context, clone);

        if deep {
            if let Some(children) = self.children() {
                for child in children {
                    clone
                        .append_child(gc_context, child.duplicate(gc_context, deep))
                        .expect("If I can see my children then my clone should accept children");
                }
            }
        }

        clone
    }

    /// Retrieve the value of a single attribute on this node.
    ///
    /// If the node does not contain attributes, then this function always
    /// yields None.
    pub fn attribute_value(self, name: &XMLName) -> Option<String> {
        match &*self.0.read() {
            XMLNodeData::Element { attributes, .. } => attributes.get(name).cloned(),
            _ => None,
        }
    }

    /// Look up the URI for the given namespace.
    ///
    /// XML namespaces are determined by `xmlns:` namespace attributes on the
    /// current node, or it's parent.
    pub fn lookup_uri_for_namespace(self, namespace: &str) -> Option<String> {
        if let Some(url) = self.attribute_value(&XMLName::from_parts(Some("xmlns"), namespace)) {
            Some(url)
        } else if let Ok(Some(parent)) = self.parent() {
            parent.lookup_uri_for_namespace(namespace)
        } else {
            None
        }
    }

    /// Retrieve the first attribute key set to a given value, if any.
    ///
    /// If the node does not contain attributes, then this function always
    /// yields None.
    ///
    /// You may restrict your value search to specific namespaces by setting
    /// `within_namespace`. If it is set to `None`, then any namespace's
    /// attributes may satisfy the search. It is it set to `""`, then
    /// the default namespace will be searched.
    pub fn value_attribute(self, value: &str, within_namespace: Option<&str>) -> Option<XMLName> {
        match &*self.0.read() {
            XMLNodeData::Element { attributes, .. } => {
                for (attr, attr_value) in attributes.iter() {
                    if let Some(namespace) = within_namespace {
                        if attr.prefix().unwrap_or("") == namespace && value == attr_value {
                            return Some(attr.clone());
                        }
                    } else if value == attr_value {
                        return Some(attr.clone());
                    }
                }

                None
            }
            _ => None,
        }
    }

    /// Look up the namespace for the given URI.
    ///
    /// XML namespaces are determined by `xmlns:` namespace attributes on the
    /// current node, or it's parent.
    ///
    /// If there are multiple namespaces that match the URI, the first
    /// mentioned on the closest node will be returned.
    pub fn lookup_namespace_for_uri(self, uri: &str) -> Option<String> {
        if let Some(xname) = self.value_attribute(uri, Some("xmlns")) {
            Some(xname.local_name().to_string())
        } else if let Ok(Some(parent)) = self.parent() {
            parent.lookup_namespace_for_uri(uri)
        } else {
            None
        }
    }
}

impl<'gc> fmt::Debug for XMLNode<'gc> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.0.read() {
            XMLNodeData::Text { contents, .. } => f
                .debug_struct("XMLNodeData::Text")
                .field("script_object", &"<Elided>".to_string())
                .field("document", &"<Elided>".to_string())
                .field("parent", &"<Elided>".to_string())
                .field("contents", contents)
                .finish(),
            XMLNodeData::Comment { contents, .. } => f
                .debug_struct("XMLNodeData::Comment")
                .field("script_object", &"<Elided>".to_string())
                .field("document", &"<Elided>".to_string())
                .field("parent", &"<Elided>".to_string())
                .field("contents", contents)
                .finish(),
            XMLNodeData::Element {
                tag_name,
                attributes,
                children,
                ..
            } => f
                .debug_struct("XMLNodeData::Element")
                .field("script_object", &"<Elided>".to_string())
                .field("document", &"<Elided>".to_string())
                .field("parent", &"<Elided>".to_string())
                .field("tag_name", tag_name)
                .field("attributes", attributes)
                .field("children", children)
                .finish(),
            XMLNodeData::DocumentRoot { children, .. } => f
                .debug_struct("XMLNodeData::DocumentRoot")
                .field("script_object", &"<Elided>".to_string())
                .field("document", &"<Elided>".to_string())
                .field("children", children)
                .finish(),
        }
    }
}
