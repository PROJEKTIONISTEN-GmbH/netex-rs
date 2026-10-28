#!/usr/bin/env python3
"""Reads the NeTEx schema sets of both releases and derives what the crate needs
from them.

The schemas are the authority. Starting from each release's root,
`NeTEx_publication.xsd`, every included and imported schema is read, so that what
is walked is what a validator reads. For every element the walker flattens the
content model of its type — extension chains, group references, nested sequences
and choices, substitution groups — into the ordered list of child elements it may
hold, the attributes it may carry, and, where its type is an enumeration, the tokens
it may take.

From the two walks the script writes:

* `src/version/table.rs` — the differences between the releases: child elements,
  attributes and enumeration tokens one release knows and the other does not. The
  crate reads it to tell the releases apart and to report what a document loses
  when it is written for the other one.
* `docs/VERSIONS.md` — the same differences as a document.

With `--declare NAME` it prints the ordered child list of the element `NAME` in
both releases instead, which is how the typed declarations in `src/model/` were
drafted; `--enum NAME` prints the tokens of the enumerated type `NAME`.

Only the standard library is used. Usage, from the repository root:

    python3 tools/schema_table.py            # regenerate the table and the document
    python3 tools/schema_table.py --declare StopPlace
    python3 tools/schema_table.py --enum AllVehicleModesOfTransportEnumeration
"""

from __future__ import annotations

import os
import sys
import xml.etree.ElementTree as ET
from collections import OrderedDict, defaultdict
from dataclasses import dataclass, field

XS = "{http://www.w3.org/2001/XMLSchema}"
NETEX = "http://www.netex.org.uk/netex"
# The namespaces the NeTEx schemas import, under the prefixes the schemas and
# the documents use for them. Their elements are kept apart from NeTEx elements
# of the same local name (GML has a `Point`, and so has NeTEx).
FOREIGN_PREFIXES = {
    "http://www.siri.org.uk/siri": "siri",
    "http://www.opengis.net/gml/3.2": "gml",
}
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

RELEASES = OrderedDict(
    [
        ("1.x", os.path.join(ROOT, "tests/fixtures/netex-1.3/xsd")),
        ("2.0", os.path.join(ROOT, "tests/fixtures/netex-2.0/xsd")),
    ]
)


@dataclass
class Particle:
    """One child element an element may hold, with its cardinality."""

    name: str
    min_occurs: int
    max_occurs: int | None  # None is unbounded
    choice: int  # 0 outside a choice, else the id of the innermost choice


@dataclass
class Schema:
    """One release's schema set, by name."""

    elements: dict[str, ET.Element] = field(default_factory=dict)
    complex_types: dict[str, ET.Element] = field(default_factory=dict)
    simple_types: dict[str, ET.Element] = field(default_factory=dict)
    groups: dict[str, ET.Element] = field(default_factory=dict)
    attribute_groups: dict[str, ET.Element] = field(default_factory=dict)
    substitutes: dict[str, list[str]] = field(default_factory=lambda: defaultdict(list))
    # Local element declarations by name, for elements that are not global.
    local_elements: dict[str, list[ET.Element]] = field(default_factory=lambda: defaultdict(list))


def local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def strip_prefix(name: str) -> str:
    """The name without its prefix, unless the prefix names a foreign namespace."""
    if ":" not in name:
        return name
    prefix, local = name.split(":", 1)
    if prefix in FOREIGN_PREFIXES.values():
        return name
    return local


def load(root_xsd: str) -> Schema:
    """Reads a schema set from its root, following includes and imports."""
    schema = Schema()
    seen: set[str] = set()
    queue = [os.path.abspath(root_xsd)]
    while queue:
        path = queue.pop()
        if path in seen or not os.path.exists(path):
            continue
        seen.add(path)
        tree = ET.parse(path).getroot()
        base = os.path.dirname(path)
        namespace = tree.get("targetNamespace", NETEX)
        # A foreign schema's names are keyed with their prefix, as the NeTEx
        # schemas refer to them; its local elements are no NeTEx elements.
        prefix = FOREIGN_PREFIXES.get(namespace)
        keyed = (lambda name: f"{prefix}:{name}") if prefix else (lambda name: name)
        for node in tree:
            tag = local(node.tag)
            if tag in ("include", "import"):
                location = node.get("schemaLocation")
                if location:
                    queue.append(os.path.normpath(os.path.join(base, location)))
            elif tag == "element":
                schema.elements[keyed(node.get("name"))] = node
                head = node.get("substitutionGroup")
                if head:
                    schema.substitutes[strip_prefix(head)].append(keyed(node.get("name")))
            elif tag == "complexType":
                schema.complex_types[keyed(node.get("name"))] = node
            elif tag == "simpleType":
                schema.simple_types[keyed(node.get("name"))] = node
            elif tag == "group":
                schema.groups[keyed(node.get("name"))] = node
            elif tag == "attributeGroup":
                schema.attribute_groups[keyed(node.get("name"))] = node
        if prefix is None:
            for node in tree.iter(XS + "element"):
                if node.get("name") and node not in schema.elements.values():
                    schema.local_elements[node.get("name")].append(node)
    return schema


class Walker:
    """Flattens content models of one release."""

    def __init__(self, schema: Schema):
        self.schema = schema
        self.choice_counter = 0
        self._children_cache: dict[str, list[Particle]] = {}
        self._attributes_cache: dict[str, list[str]] = {}

    # -- content models ------------------------------------------------------------

    def particles_of_type(self, type_name: str, stack: tuple[str, ...] = ()) -> list[Particle]:
        if type_name in self._children_cache:
            return self._children_cache[type_name]
        node = self.schema.complex_types.get(type_name)
        if node is None or type_name in stack:
            return []
        out = self._particles_of_complex(node, stack + (type_name,))
        self._children_cache[type_name] = out
        return out

    def _particles_of_complex(self, node: ET.Element, stack: tuple[str, ...]) -> list[Particle]:
        out: list[Particle] = []
        for child in node:
            tag = local(child.tag)
            if tag in ("complexContent", "simpleContent"):
                for derivation in child:
                    base = strip_prefix(derivation.get("base", ""))
                    if local(derivation.tag) == "extension":
                        out.extend(self.particles_of_type(base, stack))
                    self._collect(derivation, out, 0, stack)
            elif tag in ("sequence", "choice", "all", "group"):
                self._collect_particle(child, out, 0, stack)
        return out

    def _collect(self, node: ET.Element, out: list[Particle], choice: int, stack) -> None:
        for child in node:
            self._collect_particle(child, out, choice, stack)

    def _collect_particle(self, node: ET.Element, out: list[Particle], choice: int, stack) -> None:
        tag = local(node.tag)
        if tag == "element":
            self._element(node, out, choice)
        elif tag == "sequence" or tag == "all":
            self._collect(node, out, choice, stack)
        elif tag == "choice":
            self.choice_counter += 1
            self._collect(node, out, self.choice_counter, stack)
        elif tag == "group":
            ref = node.get("ref")
            if ref:
                group = self.schema.groups.get(strip_prefix(ref))
                if group is not None:
                    self._collect(group, out, choice, stack)
            else:
                self._collect(node, out, choice, stack)
        elif tag == "any":
            out.append(Particle("*", 0, None, choice))

    def _element(self, node: ET.Element, out: list[Particle], choice: int) -> None:
        min_occurs = int(node.get("minOccurs", "1"))
        max_raw = node.get("maxOccurs", "1")
        max_occurs = None if max_raw == "unbounded" else int(max_raw)
        ref = node.get("ref")
        if ref:
            name = strip_prefix(ref)
            names = [name] + self._substitutes(name)
            group = choice
            if len(names) > 1 and choice == 0:
                self.choice_counter += 1
                group = self.choice_counter
            for candidate in names:
                head = self.schema.elements.get(candidate)
                if head is not None and head.get("abstract") == "true":
                    continue
                out.append(Particle(candidate, min_occurs, max_occurs, group))
        else:
            out.append(Particle(node.get("name"), min_occurs, max_occurs, choice))

    def _substitutes(self, head: str) -> list[str]:
        out: list[str] = []
        queue = list(self.schema.substitutes.get(head, []))
        while queue:
            name = queue.pop()
            if name in out:
                continue
            out.append(name)
            queue.extend(self.schema.substitutes.get(name, []))
        return out

    # -- per element -----------------------------------------------------------------

    def type_of(self, element: ET.Element) -> str | None:
        declared = element.get("type")
        if declared:
            return strip_prefix(declared)
        return None

    def children_of_element(self, element: ET.Element) -> list[Particle]:
        type_name = self.type_of(element)
        if type_name:
            return self.particles_of_type(type_name)
        inline = element.find(XS + "complexType")
        if inline is not None:
            return self._particles_of_complex(inline, ())
        return []

    def attributes_of_type(self, type_name: str, stack: tuple[str, ...] = ()) -> list[str]:
        if type_name in self._attributes_cache:
            return self._attributes_cache[type_name]
        node = self.schema.complex_types.get(type_name)
        if node is None or type_name in stack:
            return []
        out = self._attributes_of_complex(node, stack + (type_name,))
        self._attributes_cache[type_name] = out
        return out

    def _attributes_of_complex(self, node: ET.Element, stack) -> list[str]:
        out: list[str] = []
        for child in node:
            tag = local(child.tag)
            if tag in ("complexContent", "simpleContent"):
                for derivation in child:
                    # An extension adds to the base type; a restriction restates the
                    # content model but keeps the base type's attributes.
                    base = strip_prefix(derivation.get("base", ""))
                    out.extend(self.attributes_of_type(base, stack))
                    self._collect_attributes(derivation, out)
            else:
                self._collect_attributes_node(child, out)
        seen: list[str] = []
        for name in out:
            if name not in seen:
                seen.append(name)
        return seen

    def _collect_attributes(self, node: ET.Element, out: list[str]) -> None:
        for child in node:
            self._collect_attributes_node(child, out)

    def _collect_attributes_node(self, node: ET.Element, out: list[str]) -> None:
        tag = local(node.tag)
        if tag == "attribute":
            name = node.get("name") or strip_prefix(node.get("ref", ""))
            if name:
                out.append(name)
        elif tag == "attributeGroup":
            group = self.schema.attribute_groups.get(strip_prefix(node.get("ref", "")))
            if group is not None:
                self._collect_attributes(group, out)

    def attributes_of_element(self, element: ET.Element) -> list[str]:
        type_name = self.type_of(element)
        if type_name:
            return self.attributes_of_type(type_name)
        inline = element.find(XS + "complexType")
        if inline is not None:
            return self._attributes_of_complex(inline, ())
        return []

    # -- enumerations ----------------------------------------------------------------

    def tokens_of_simple(self, type_name: str, stack: tuple[str, ...] = ()) -> list[str]:
        node = self.schema.simple_types.get(type_name)
        if node is None or type_name in stack:
            return []
        out: list[str] = []
        for child in node.iter():
            tag = local(child.tag)
            if tag == "enumeration":
                out.append(child.get("value"))
            elif tag == "restriction" and child.get("base"):
                out.extend(self.tokens_of_simple(strip_prefix(child.get("base")), stack + (type_name,)))
            elif tag == "union":
                for member in (child.get("memberTypes") or "").split():
                    out.extend(self.tokens_of_simple(strip_prefix(member), stack + (type_name,)))
        return out

    def is_closed(self, type_name: str, stack: tuple[str, ...] = ()) -> bool:
        """Whether every value of a simple type is one of its enumeration tokens.

        A restriction with tokens is closed; a union is closed when every
        member is; a built-in type, a pattern or a list is not.
        """
        node = self.schema.simple_types.get(type_name)
        if node is None or type_name in stack:
            return False
        for child in node:
            tag = local(child.tag)
            if tag == "restriction":
                if any(local(c.tag) == "enumeration" for c in child):
                    return True
                base = strip_prefix(child.get("base", ""))
                return self.is_closed(base, stack + (type_name,))
            if tag == "union":
                members = (child.get("memberTypes") or "").split()
                inline = [c for c in child if local(c.tag) == "simpleType"]
                return all(self.is_closed(strip_prefix(m), stack + (type_name,)) for m in members) and all(
                    any(local(g.tag) == "enumeration" for g in c.iter()) for c in inline
                )
            if tag == "list":
                return False
        return False

    def tokens_of_element(self, element: ET.Element) -> list[str] | None:
        """Tokens of an element whose type is a closed enumeration, else `None`."""
        type_name = self.type_of(element)
        if type_name and type_name in self.schema.simple_types:
            return self.tokens_of_simple(type_name) if self.is_closed(type_name) else None
        inline = element.find(XS + "simpleType")
        if inline is not None:
            out = [child.get("value") for child in inline.iter(XS + "enumeration")]
            return out if out else None
        if type_name and type_name in self.schema.complex_types:
            # simpleContent extension of an enumeration
            node = self.schema.complex_types[type_name]
            content = node.find(XS + "simpleContent")
            if content is not None:
                for derivation in content:
                    base = strip_prefix(derivation.get("base", ""))
                    if base in self.schema.simple_types:
                        return self.tokens_of_simple(base) if self.is_closed(base) else None
        return None


@dataclass
class Facts:
    """What one release allows, keyed by names."""

    children: set[tuple[str, str]]
    attributes: set[tuple[str, str]]
    tokens: set[tuple[str, str]]
    elements: set[str]


def facts(schema: Schema) -> Facts:
    walker = Walker(schema)
    children: set[tuple[str, str]] = set()
    attributes: set[tuple[str, str]] = set()
    tokens: set[tuple[str, str]] = set()
    elements: set[str] = set()
    # An element name is token-checked only if every declaration of it is a
    # closed enumeration: `Covered` is an enumeration on a site and a boolean
    # on a parking, so any token must pass.
    open_names: set[str] = set()
    declarations: list[tuple[str, ET.Element]] = [
        (name, node) for name, node in schema.elements.items() if ":" not in name
    ]
    for name, nodes in schema.local_elements.items():
        declarations.extend((name, node) for node in nodes)
    for name, node in declarations:
        if node.get("abstract") == "true":
            continue
        elements.add(name)
        for particle in walker.children_of_element(node):
            if particle.name != "*":
                children.add((name, particle.name))
        for attribute in walker.attributes_of_element(node):
            attributes.add((name, attribute))
        declared_tokens = walker.tokens_of_element(node)
        if declared_tokens is None:
            open_names.add(name)
        else:
            for token in declared_tokens:
                tokens.add((name, token))
    tokens = {(name, token) for name, token in tokens if name not in open_names}
    return Facts(children, attributes, tokens, elements)


def rust_literal(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def rust_array(name: str, rows: list) -> list[str]:
    """A `pub static NAME: &[…] = &[…];` declaration, laid out as rustfmt lays it
    out: an empty array inline, short elements on one line when they fit, and
    everything else one element per line. A layout rustfmt would pack
    differently — many short elements — is refused rather than guessed."""
    pairs = bool(rows) and isinstance(rows[0], tuple)
    element_type = "(&str, &str)" if pairs else "&str"
    head = f"pub static {name}: &[{element_type}] = &["
    if not rows:
        return [head + "];"]
    if not pairs:
        literals = [rust_literal(a) for a in rows]
        if all(len(literal) <= 12 for literal in literals):
            line = head + ", ".join(literals) + "];"
            if len(line) > 100:
                raise SystemExit(f"{name}: rustfmt would pack these short elements; extend rust_array")
            return [line]
    body = [rust_pair(a, b) for a, b in rows] if pairs else [f"    {rust_literal(a)}," for a in rows]
    return [head, *body, "];"]


def rust_pair(a: str, b: str) -> str:
    """A `(a, b),` array entry, wrapped the way rustfmt wraps a tuple whose
    contents exceed its 60-column literal width, so that regenerating the table
    leaves a formatted tree unchanged."""
    inner = f"{rust_literal(a)}, {rust_literal(b)}"
    if len(inner) <= 60:
        return f"    ({inner}),"
    return f"    (\n        {rust_literal(a)},\n        {rust_literal(b)},\n    ),"


@dataclass
class Differences:
    """What one release knows and the other does not, in the shape the crate reads.

    A child, attribute or token name the other release does not know anywhere is
    listed once by name; a name the other release knows elsewhere is listed as the
    pair it is missing from. A token is only a difference where the element is
    enumerated in both releases: an element that is free text in the one release
    and an enumeration in the other takes any token there.
    """

    element_names: list[str]
    children: list[tuple[str, str]]
    attribute_names: list[str]
    attributes: list[tuple[str, str]]
    tokens: list[tuple[str, str]]


def differences(this: Facts, other: Facts) -> Differences:
    other_children = {child for _, child in other.children}
    other_attributes = {attribute for _, attribute in other.attributes}
    other_enumerated = {element for element, _ in other.tokens}
    element_names = sorted(
        {child for _, child in this.children if child not in other_children} | (this.elements - other.elements)
    )
    children = sorted(
        (parent, child)
        for parent, child in this.children - other.children
        if child in other_children
    )
    attribute_names = sorted({a for _, a in this.attributes if a not in other_attributes})
    attributes = sorted(
        (element, attribute)
        for element, attribute in this.attributes - other.attributes
        if attribute in other_attributes
    )
    tokens = sorted(
        (element, token)
        for element, token in this.tokens - other.tokens
        if element in other_enumerated
    )
    return Differences(element_names, children, attribute_names, attributes, tokens)


def write_table(v1: Facts, v2: Facts) -> None:
    only_1x = differences(v1, v2)
    only_20 = differences(v2, v1)
    sections = [
        ("ELEMENT_NAMES_ONLY_IN_1X", only_1x.element_names,
         "Element names the 1.x schemas declare and the 2.0 schemas do not, anywhere."),
        ("ELEMENT_NAMES_ONLY_IN_20", only_20.element_names,
         "Element names the 2.0 schemas declare and the 1.x schemas do not, anywhere."),
        ("CHILDREN_ONLY_IN_1X", only_1x.children,
         "Child elements the 1.x schemas allow under a parent and the 2.0 schemas do not, as `(parent, child)`, for children the 2.0 schemas know elsewhere."),
        ("CHILDREN_ONLY_IN_20", only_20.children,
         "Child elements the 2.0 schemas allow under a parent and the 1.x schemas do not, as `(parent, child)`, for children the 1.x schemas know elsewhere."),
        ("ATTRIBUTE_NAMES_ONLY_IN_1X", only_1x.attribute_names,
         "Attribute names the 1.x schemas declare and the 2.0 schemas do not, anywhere."),
        ("ATTRIBUTE_NAMES_ONLY_IN_20", only_20.attribute_names,
         "Attribute names the 2.0 schemas declare and the 1.x schemas do not, anywhere."),
        ("ATTRIBUTES_ONLY_IN_1X", only_1x.attributes,
         "Attributes the 1.x schemas allow on an element and the 2.0 schemas do not, as `(element, attribute)`, for attributes the 2.0 schemas know elsewhere."),
        ("ATTRIBUTES_ONLY_IN_20", only_20.attributes,
         "Attributes the 2.0 schemas allow on an element and the 1.x schemas do not, as `(element, attribute)`, for attributes the 1.x schemas know elsewhere."),
        ("TOKENS_ONLY_IN_1X", only_1x.tokens,
         "Enumeration tokens the 1.x schemas allow as the content of an element the 2.0 schemas also enumerate, and the 2.0 schemas do not, as `(element, token)`."),
        ("TOKENS_ONLY_IN_20", only_20.tokens,
         "Enumeration tokens the 2.0 schemas allow as the content of an element the 1.x schemas also enumerate, and the 1.x schemas do not, as `(element, token)`."),
    ]
    lines = [
        "//! What one release of the schemas knows and the other does not.",
        "//!",
        "//! Generated by `tools/schema_table.py` from the two schema sets under",
        "//! `tests/fixtures/`; do not edit by hand. Each table is sorted, so it can be",
        "//! searched by bisection. The names are local element names: a pair names a",
        "//! child under a parent wherever that parent occurs. A name the other release",
        "//! does not know anywhere is listed once, by name, rather than once per parent.",
        "",
    ]
    for name, rows, doc in sections:
        lines.append(f"/// {doc}")
        lines.extend(rust_array(name, rows))
        lines.append("")
    path = os.path.join(ROOT, "src/version/table.rs")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines).rstrip() + "\n")

    doc_lines = [
        "# NeTEx 1.x and 2.0: what one release knows and the other does not",
        "",
        "Derived from the two schema sets under `tests/fixtures/` by `tools/schema_table.py`;",
        "the same facts drive `src/version/table.rs`. A row names a local element: a child",
        "listed under a parent is one that parent may hold in the one release and not in",
        "the other, wherever the parent occurs. A name the other release does not know",
        "anywhere is listed once, by name. Reading a document, the crate uses the rows to",
        "tell the releases apart; writing one for the other release, it drops what that",
        "release does not know and reports each drop as a loss.",
        "",
        "| Table | 1.x only | 2.0 only |",
        "|---|---|---|",
        f"| element names | {len(only_1x.element_names)} | {len(only_20.element_names)} |",
        f"| child elements (parent, child) | {len(only_1x.children)} | {len(only_20.children)} |",
        f"| attribute names | {len(only_1x.attribute_names)} | {len(only_20.attribute_names)} |",
        f"| attributes (element, attribute) | {len(only_1x.attributes)} | {len(only_20.attributes)} |",
        f"| enumeration tokens (element, token) | {len(only_1x.tokens)} | {len(only_20.tokens)} |",
        "",
    ]
    for title, rows in [
        ("Element names only in 1.x", only_1x.element_names),
        ("Element names only in 2.0", only_20.element_names),
        ("Attribute names only in 1.x", only_1x.attribute_names),
        ("Attribute names only in 2.0", only_20.attribute_names),
    ]:
        doc_lines.append(f"## {title}")
        doc_lines.append("")
        doc_lines.append(", ".join(f"`{name}`" for name in rows) if rows else "none")
        doc_lines.append("")
    for title, rows, header in [
        ("Child elements only in 1.x", only_1x.children, "| Parent | Child |"),
        ("Child elements only in 2.0", only_20.children, "| Parent | Child |"),
        ("Attributes only in 1.x", only_1x.attributes, "| Element | Attribute |"),
        ("Attributes only in 2.0", only_20.attributes, "| Element | Attribute |"),
        ("Enumeration tokens only in 1.x", only_1x.tokens, "| Element | Token |"),
        ("Enumeration tokens only in 2.0", only_20.tokens, "| Element | Token |"),
    ]:
        doc_lines.append(f"## {title}")
        doc_lines.append("")
        doc_lines.append(header)
        doc_lines.append("|---|---|")
        for a, b in rows:
            doc_lines.append(f"| `{a}` | `{b}` |")
        doc_lines.append("")
    path = os.path.join(ROOT, "docs/VERSIONS.md")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(doc_lines).rstrip() + "\n")


def declare(name: str, schemas: dict[str, Schema]) -> None:
    for release, schema in schemas.items():
        walker = Walker(schema)
        nodes = []
        if name in schema.elements:
            nodes.append(schema.elements[name])
        nodes.extend(schema.local_elements.get(name, []))
        if not nodes:
            print(f"{release}: no element {name}")
            continue
        node = nodes[0]
        print(f"{release}: {name} (type {walker.type_of(node) or 'inline'}, {len(nodes)} declaration(s))")
        print("  attributes:", ", ".join(walker.attributes_of_element(node)))
        for particle in walker.children_of_element(node):
            card = f"{particle.min_occurs}..{'n' if particle.max_occurs is None else particle.max_occurs}"
            choice = f" choice#{particle.choice}" if particle.choice else ""
            print(f"  {particle.name:40} {card}{choice}")


def main(argv: list[str]) -> int:
    schemas = {
        release: load(os.path.join(path, "NeTEx_publication.xsd")) for release, path in RELEASES.items()
    }
    if len(argv) >= 2 and argv[0] == "--declare":
        for name in argv[1:]:
            declare(name, schemas)
        return 0
    if len(argv) >= 2 and argv[0] == "--enum":
        for name in argv[1:]:
            for release, schema in schemas.items():
                walker = Walker(schema)
                print(release, name, walker.tokens_of_simple(name), "closed" if walker.is_closed(name) else "open")
        return 0
    v1, v2 = (facts(schema) for schema in schemas.values())
    write_table(v1, v2)
    for label, this, other in (("1.x only", v1, v2), ("2.0 only", v2, v1)):
        d = differences(this, other)
        print(
            f"{label}: {len(d.element_names)} element names, {len(d.children)} (parent, child) pairs, "
            f"{len(d.attribute_names)} attribute names, {len(d.attributes)} (element, attribute) pairs, "
            f"{len(d.tokens)} (element, token) pairs"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
