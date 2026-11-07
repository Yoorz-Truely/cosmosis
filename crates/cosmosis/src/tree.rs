//! A parser-neutral node tree: the shape of a Null file (Null spec, section 5) without
//! Null's parser. Cosmósis reads its own files (the manifest first) from this tree, so the
//! reading is written and tested now and Null's parser only has to fill it (Sau task 1a-11).
//!
//! A file is a list of nodes. A node is a name, then entries (values and `key=value`
//! properties, in the order written), then an optional block of child nodes. A value is a
//! quoted string, a bare word or a list. Numbers, handles and enum values are words: the
//! schema decides what a word means, and keeping the text as written keeps it exact.

/// A byte range in the source file, `start..end`. Hand-made trees use `0..0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    /// The first byte.
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
}

impl Span {
    /// The range `start..end`.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// What a value is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueKind {
    /// A bare word, exactly as written: `project`, `1`, `0.1.0`, `@pier`.
    Word(String),
    /// A quoted string, escapes resolved: `"harbor-at-dusk"`.
    Str(String),
    /// A list: `[0m 0m 1m]`.
    List(Vec<Value>),
}

/// A value with where it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value {
    /// The value.
    pub kind: ValueKind,
    /// Where it is.
    pub span: Span,
}

impl Value {
    /// A bare word.
    pub fn word(text: &str) -> Self {
        Self {
            kind: ValueKind::Word(text.to_owned()),
            span: Span::default(),
        }
    }

    /// A quoted string.
    pub fn string(text: &str) -> Self {
        Self {
            kind: ValueKind::Str(text.to_owned()),
            span: Span::default(),
        }
    }

    /// A list.
    pub fn list(items: Vec<Value>) -> Self {
        Self {
            kind: ValueKind::List(items),
            span: Span::default(),
        }
    }

    /// The same value at `span`.
    pub fn at(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    /// The value as people write it, for messages: `"text"`, `word`, `[a b]`.
    pub fn describe(&self) -> String {
        match &self.kind {
            ValueKind::Word(w) => w.clone(),
            ValueKind::Str(s) => format!("{s:?}"),
            ValueKind::List(items) => {
                let inner: Vec<String> = items.iter().map(Value::describe).collect();
                format!("[{}]", inner.join(" "))
            }
        }
    }
}

/// One `key=value` property on a node's line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    /// The key.
    pub key: String,
    /// Where the key is.
    pub key_span: Span,
    /// The value.
    pub value: Value,
}

/// One entry on a node's line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A value: `package "harbor"` has the argument `"harbor"`.
    Argument(Value),
    /// A property: `my-trees path="../Trees.sau"`.
    Property(Property),
}

/// A node: a name, its entries and its child nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// The node's name, such as `package` or `null-format`.
    pub name: String,
    /// Where the name is.
    pub name_span: Span,
    /// Its values and properties, in the order written.
    pub entries: Vec<Entry>,
    /// Its `{ }` block, if it has one (an empty block is `Some` of nothing).
    pub children: Option<Vec<Node>>,
    /// The whole node.
    pub span: Span,
}

impl Node {
    /// A node with no entries and no block.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            name_span: Span::default(),
            entries: Vec::new(),
            children: None,
            span: Span::default(),
        }
    }

    /// Adds a value.
    pub fn arg(mut self, value: Value) -> Self {
        self.entries.push(Entry::Argument(value));
        self
    }

    /// Adds a `key=value` property.
    pub fn prop(mut self, key: &str, value: Value) -> Self {
        self.entries.push(Entry::Property(Property {
            key: key.to_owned(),
            key_span: Span::default(),
            value,
        }));
        self
    }

    /// Adds a child node, making the block if there is none.
    pub fn child(mut self, node: Node) -> Self {
        self.children.get_or_insert_with(Vec::new).push(node);
        self
    }

    /// Gives the node an empty block.
    pub fn block(mut self) -> Self {
        self.children.get_or_insert_with(Vec::new);
        self
    }

    /// The same node, its name at `span`.
    pub fn at(mut self, span: Span) -> Self {
        self.name_span = span;
        self.span = span;
        self
    }

    /// The node's values, in order.
    pub fn arguments(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().filter_map(|e| match e {
            Entry::Argument(v) => Some(v),
            Entry::Property(_) => None,
        })
    }

    /// The node's properties, in order.
    pub fn properties(&self) -> impl Iterator<Item = &Property> {
        self.entries.iter().filter_map(|e| match e {
            Entry::Property(p) => Some(p),
            Entry::Argument(_) => None,
        })
    }
}
