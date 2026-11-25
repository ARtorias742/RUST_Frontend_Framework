//! Proc macros for the Ferrum frontend framework.
//!
//! - [`component`] — attribute macro turning `fn Name(args) -> VNode` into a
//!   props struct + `ComponentProps` impl.
//! - [`rsx`] — macro producing [`ferrum_core::VNode`] trees from a JSX-like
//!   syntax (elements, components, text interpolation, `if`/`else`, `for`,
//!   fragments, keys, event listeners).

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenStream as TS2, TokenTree};
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, Pat};

// ---------------------------------------------------------------------------
// #[component]
// ---------------------------------------------------------------------------

/// Turn a function into a Ferrum component.
///
/// ```ignore
/// #[component]
/// fn Card(title: String, children: Vec<VNode>) -> VNode { ... }
/// ```
///
/// generates a `Card` props struct (all fn args + a `children` field) and an
/// `impl ComponentProps for Card`. Inside `rsx!`, use `Card { title: ... } { .. }`.
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    expand_component(item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_component(item: ItemFn) -> syn::Result<TS2> {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = &item;

    if sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            sig.fn_token,
            "async components are not supported yet",
        ));
    }
    if !sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &sig.generics,
            "generic components are not supported yet",
        ));
    }
    if let Some(FnArg::Receiver(_)) = sig.inputs.first() {
        return Err(syn::Error::new_spanned(
            &sig.inputs,
            "method components are not supported; use a free function",
        ));
    }

    let name = &sig.ident;
    let mut fields: Vec<TS2> = Vec::new();
    let mut idents: Vec<proc_macro2::Ident> = Vec::new();
    let mut has_children = false;

    for arg in &sig.inputs {
        let FnArg::Typed(pt) = arg else {
            unreachable!("receiver checked above")
        };
        let ident = match &*pt.pat {
            Pat::Ident(pi) => pi.ident.clone(),
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "component args must be simple identifiers (`name: Type`)",
                ))
            }
        };
        let ty = &pt.ty;
        if ident == "children" {
            has_children = true;
        }
        fields.push(quote! { #vis #ident: #ty });
        idents.push(ident);
    }
    if !has_children {
        fields.push(quote! { #vis children: Vec<::ferrum_core::VNode> });
    }

    let body = quote! {
        #(#attrs)*
        #[allow(non_camel_case_types)]
        #vis struct #name {
            #(#fields,)*
        }

        impl ::ferrum_core::ComponentProps for #name {
            fn render(&self) -> ::ferrum_core::VNode {
                #[allow(unused_variables)]
                let #name { #(#idents,)* .. } = self;
                #block
            }
        }
    };
    Ok(body)
}

// ---------------------------------------------------------------------------
// rsx!
// ---------------------------------------------------------------------------

/// JSX-like macro producing a [`ferrum_core::VNode`].
///
/// ```ignore
/// rsx! {
///     div { class: "app",
///         h1 { "Hello, {name}" }
///         if show { br {} } else { span { "hidden" } }
///         for i in 0..3 { li { key: i, "{i}" } }
///         Counter { count: 5 }
///     }
/// }
/// ```
#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    let ts: TS2 = input.into();
    let mut p = P {
        t: ts.into_iter().collect(),
        i: 0,
    };
    match parse_nodes(&mut p, Stop::Eof) {
        Ok(nodes) => {
            let out = nodes_expr(&nodes);
            out.into()
        }
        Err(e) => e.to_compile_error().into(),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Stop {
    Eof,
    FragmentClose,
}

struct P {
    t: Vec<TokenTree>,
    i: usize,
}

impl P {
    fn peek(&self) -> Option<&TokenTree> {
        self.t.get(self.i)
    }
    fn peek_at(&self, n: usize) -> Option<&TokenTree> {
        self.t.get(self.i + n)
    }
    fn bump(&mut self) -> Option<TokenTree> {
        let r = self.t.get(self.i).cloned();
        if r.is_some() {
            self.i += 1;
        }
        r
    }
    fn eat_punct(&mut self, c: char) -> bool {
        if let Some(TokenTree::Punct(p)) = self.peek() {
            if p.as_char() == c {
                self.i += 1;
                return true;
            }
        }
        false
    }
    fn peek_punct(&self, c: char) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == c)
    }
    fn peek_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(id)) if id == s)
    }
    fn peek_brace_group(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace)
    }
}

fn nodes_expr(nodes: &[TS2]) -> TS2 {
    match nodes.len() {
        0 => quote! { ::ferrum_core::VNode::Empty },
        1 => nodes[0].clone(),
        _ => quote! { ::ferrum_core::VNode::fragment(vec![#(#nodes),*]) },
    }
}

fn parse_nodes(p: &mut P, stop: Stop) -> syn::Result<Vec<TS2>> {
    let mut nodes = Vec::new();
    loop {
        // Fragment close: `</>`
        if stop == Stop::FragmentClose
            && p.peek_punct('<')
            && matches!(p.peek_at(1), Some(TokenTree::Punct(q)) if q.as_char() == '/')
            && matches!(p.peek_at(2), Some(TokenTree::Punct(q)) if q.as_char() == '>')
        {
            p.i += 3;
            return Ok(nodes);
        }
        match p.peek() {
            None => {
                if stop == Stop::FragmentClose {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "unclosed fragment `<>`; expected `</>`",
                    ));
                }
                return Ok(nodes);
            }
            Some(TokenTree::Punct(q)) if q.as_char() == ',' => {
                // Optional separator between children.
                p.i += 1;
            }
            Some(TokenTree::Punct(q)) if q.as_char() == '}' => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "unexpected `}`",
                ));
            }
            Some(TokenTree::Ident(id)) => {
                let s = id.to_string();
                let is_if = s == "if";
                let is_for = s == "for";
                let first_lower = s.chars().next().map(|c| c.is_lowercase()).unwrap_or(false);
                p.i += 1;
                if is_if {
                    nodes.push(parse_if(p)?);
                } else if is_for {
                    nodes.push(parse_for(p)?);
                } else if first_lower {
                    nodes.push(parse_element(p, s)?);
                } else {
                    nodes.push(parse_component(p, s)?);
                }
            }
            Some(TokenTree::Literal(lit)) => {
                let lit = lit.clone();
                p.i += 1;
                nodes.push(text_node(&lit)?);
            }
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                // Embedded expression: `{ expr }`
                let inner = g.stream();
                p.i += 1;
                nodes.push(quote! { ::ferrum_core::VNode::from(#inner) });
            }
            Some(TokenTree::Punct(q)) if q.as_char() == '<' => {
                // Fragment open: `<>`
                if matches!(p.peek_at(1), Some(TokenTree::Punct(q2)) if q2.as_char() == '>') {
                    p.i += 2;
                    let inner = parse_nodes(p, Stop::FragmentClose)?;
                    nodes.push(quote! { ::ferrum_core::VNode::fragment(vec![#(#inner),*]) });
                } else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "expected `<>` to open a fragment",
                    ));
                }
            }
            Some(other) => {
                return Err(syn::Error::new_spanned(
                    other,
                    "expected an element, component, text, `if`, `for`, `{expr}` or `<>`",
                ));
            }
        }
    }
}

// --- elements ---------------------------------------------------------------

struct Body {
    attrs: Vec<(String, TS2)>,
    listeners: Vec<(String, TS2)>,
    children: Vec<TS2>,
    key: Option<TS2>,
}

fn parse_element(p: &mut P, tag: String) -> syn::Result<TS2> {
    let group = expect_brace(p, "element body `{ ... }`")?;
    let body = parse_body(group.stream(), false)?;
    Ok(emit_element(&tag, body))
}

fn expect_brace(p: &mut P, what: &str) -> syn::Result<proc_macro2::Group> {
    match p.bump() {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => Ok(g),
        other => Err(syn::Error::new_spanned(
            other.unwrap_or_else(|| {
                proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(
                    '!',
                    proc_macro2::Spacing::Alone,
                ))
            }),
            format!("expected {}", what),
        )),
    }
}

fn parse_body(stream: TS2, component: bool) -> syn::Result<Body> {
    let mut p = P {
        t: stream.into_iter().collect(),
        i: 0,
    };
    let mut body = Body {
        attrs: Vec::new(),
        listeners: Vec::new(),
        children: Vec::new(),
        key: None,
    };
    while p.i < p.t.len() {
        // Keyword children: `if cond { .. }` / `for x in y { .. }`
        // (an attr named `for`/`if` is still possible via `for: value`)
        if (p.peek_ident("if") || p.peek_ident("for"))
            && !matches!(p.peek_at(1), Some(TokenTree::Punct(q)) if q.as_char() == ':')
        {
            let is_if = p.peek_ident("if");
            p.i += 1;
            let child = if is_if {
                parse_if(&mut p)?
            } else {
                parse_for(&mut p)?
            };
            body.children.push(child);
            continue;
        }
        match p.peek().cloned() {
            Some(TokenTree::Ident(id)) => {
                // Read a (possibly dashed) name: `data-id`, `onclick`, ...
                let mut name = id.to_string();
                p.i += 1;
                while p.peek_punct('-') {
                    p.i += 1;
                    match p.bump() {
                        Some(TokenTree::Ident(id2)) => {
                            name.push('-');
                            name.push_str(&id2.to_string());
                        }
                        other => {
                            return Err(syn::Error::new_spanned(
                                other.unwrap(),
                                "expected identifier after `-`",
                            ))
                        }
                    }
                }
                if p.peek_punct(':') {
                    // Attribute / listener / key / component prop
                    p.i += 1;
                    let value = maybe_interpolate(&collect_value(&mut p));
                    if component {
                        if name == "key" {
                            body.key = Some(value);
                        } else if name == "children" {
                            return Err(syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "pass children with the block syntax `Comp { .. } { .. }`, not `children:`",
                            ));
                        } else {
                            body.attrs.push((name, value));
                        }
                    } else if name == "key" {
                        body.key = Some(value);
                    } else if name.starts_with("on") {
                        body.listeners.push((name, value));
                    } else {
                        body.attrs.push((name, value));
                    }
                } else if p.peek_brace_group() {
                    // Child element or component
                    let group = expect_brace(&mut p, "child body")?;
                    let child = if name
                        .chars()
                        .next()
                        .map(|c| c.is_lowercase())
                        .unwrap_or(false)
                    {
                        let b = parse_body(group.stream(), false)?;
                        emit_element(&name, b)
                    } else {
                        parse_component_with_group(&mut p, name, group)?
                    };
                    body.children.push(child);
                } else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("expected `:` or `{{` after `{}`", name),
                    ));
                }
            }
            Some(TokenTree::Literal(lit)) => {
                p.i += 1;
                body.children.push(text_node(&lit)?);
            }
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                p.i += 1;
                let inner = g.stream();
                body.children
                    .push(quote! { ::ferrum_core::VNode::from(#inner) });
            }
            Some(TokenTree::Punct(q)) if q.as_char() == '<' => {
                p.i += 1;
                if !p.eat_punct('>') {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "expected `<>` to open a fragment",
                    ));
                }
                let inner = parse_nodes(&mut p, Stop::FragmentClose)?;
                body.children
                    .push(quote! { ::ferrum_core::VNode::fragment(vec![#(#inner),*]) });
            }
            Some(TokenTree::Punct(q)) if q.as_char() == ',' => {
                // Optional separator between children.
                p.i += 1;
            }
            Some(other) => {
                return Err(syn::Error::new_spanned(
                    other,
                    if component {
                        "component bodies only accept `name: value` props; use `Comp { .. } { children }`"
                    } else {
                        "expected `name: value`, a child node, or text"
                    },
                ));
            }
            None => break, // loop condition guarantees this is unreachable
        }
    }
    Ok(body)
}

/// Collect tokens for an attribute value up to the next top-level `,`.
fn collect_value(p: &mut P) -> TS2 {
    let mut out = Vec::new();
    while let Some(tt) = p.peek() {
        if matches!(tt, TokenTree::Punct(q) if q.as_char() == ',') {
            p.i += 1;
            break;
        }
        out.push(tt.clone());
        p.i += 1;
    }
    out.into_iter().collect()
}

/// If the value is a single string literal, return its interpolated form
/// (`"a {x} b"` -> `format!("a {} b", x)`); otherwise return the tokens
/// unchanged.
fn maybe_interpolate(value: &TS2) -> TS2 {
    let mut iter = value.clone().into_iter();
    if let (Some(TokenTree::Literal(lit)), None) = (iter.next(), iter.next()) {
        if let Ok(litstr) = syn::parse_str::<syn::LitStr>(&lit.to_string()) {
            if let Ok(ts) = interpolate_fmt(&litstr) {
                return ts;
            }
        }
    }
    value.clone()
}

/// True when the value is a closure / async block (skip `.into()` wrapping).
fn is_closure_like(ts: &TS2) -> bool {
    let mut it = ts.clone().into_iter();
    match it.next() {
        Some(TokenTree::Ident(id)) => {
            let s = id.to_string();
            s == "move" || s == "async" || s == "box"
        }
        Some(TokenTree::Punct(q)) => q.as_char() == '|' || q.as_char() == '\'',
        _ => false,
    }
}

fn emit_element(tag: &str, body: Body) -> TS2 {
    let Body {
        attrs,
        listeners,
        children,
        key,
    } = body;
    let key = match key {
        Some(v) => quote! { Some((#v).into()) },
        None => quote! { None },
    };
    let attr_pushes = attrs.iter().map(|(name, value)| {
        quote! { __attrs.push((#name.to_string(), (#value).into())); }
    });
    let listener_pushes = listeners.iter().map(|(name, value)| {
        quote! { __listeners.push((#name.to_string(), ::std::rc::Rc::new(#value))); }
    });
    quote! {{
        let mut __attrs: Vec<(String, ::ferrum_core::AttrValue)> = vec![];
        let mut __listeners: Vec<(String, ::ferrum_core::EventHandler)> = vec![];
        #(#attr_pushes)*
        #(#listener_pushes)*
        let __children: Vec<::ferrum_core::VNode> = vec![#(#children),*];
        ::ferrum_core::VNode::Element(::ferrum_core::VElement {
            tag: #tag.to_string(),
            attrs: __attrs,
            listeners: __listeners,
            children: __children,
            key: #key,
        })
    }}
}

// --- components -------------------------------------------------------------

fn parse_component(p: &mut P, name: String) -> syn::Result<TS2> {
    let group = expect_brace(p, "component props `{ ... }`")?;
    parse_component_with_group(p, name, group)
}

fn parse_component_with_group(
    p: &mut P,
    name: String,
    group: proc_macro2::Group,
) -> syn::Result<TS2> {
    let body = parse_body(group.stream(), true)?;
    // Children: text/elements parsed inside the braces + an optional
    // trailing children block (`Comp { .. } { children }`).
    let mut children = body.children;
    if p.peek_brace_group() {
        let g = expect_brace(p, "component children block")?;
        children.extend(parse_nodes(
            &mut P {
                t: g.stream().into_iter().collect(),
                i: 0,
            },
            Stop::Eof,
        )?);
    }
    let key = match body.key {
        Some(v) => quote! { Some((#v).into()) },
        None => quote! { None },
    };
    let name_lit = name.clone();
    let name_ident = format_ident!("{}", name);
    let prop_inits = body.attrs.iter().map(|(pname, value)| {
        let pname = format_ident!("{}", pname);
        if is_closure_like(value) {
            quote! { #pname: (#value), }
        } else {
            quote! { #pname: (#value).into(), }
        }
    });
    Ok(quote! {{
        ::ferrum_core::VNode::component(#name_lit, #key, #name_ident {
            #(#prop_inits)*
            children: vec![#(#children),*],
        })
    }})
}

// --- if / for ---------------------------------------------------------------

fn parse_if(p: &mut P) -> syn::Result<TS2> {
    // Condition: tokens up to the first brace group.
    let mut cond = Vec::new();
    let then_group = loop {
        match p.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                break g.clone();
            }
            Some(tt) => {
                cond.push(tt.clone());
                p.i += 1;
            }
            None => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "expected `{ ... }` after `if` condition",
                ))
            }
        }
    };
    p.i += 1; // consume the then-group
    let cond: TS2 = cond.into_iter().collect();
    let then_nodes = parse_nodes(
        &mut P {
            t: then_group.stream().into_iter().collect(),
            i: 0,
        },
        Stop::Eof,
    )?;
    let then_expr = nodes_expr(&then_nodes);

    // Optional `else if` / `else`
    let else_expr = if p.peek_ident("else") {
        p.i += 1;
        if p.peek_ident("if") {
            p.i += 1;
            parse_if(p)?
        } else {
            let g = expect_brace(p, "`{ ... }` after `else`")?;
            let nodes = parse_nodes(
                &mut P {
                    t: g.stream().into_iter().collect(),
                    i: 0,
                },
                Stop::Eof,
            )?;
            nodes_expr(&nodes)
        }
    } else {
        quote! { ::ferrum_core::VNode::Empty }
    };

    Ok(quote! {
        if #cond { #then_expr } else { #else_expr }
    })
}

fn parse_for(p: &mut P) -> syn::Result<TS2> {
    // Pattern: tokens up to `in`
    let mut pat = Vec::new();
    loop {
        match p.peek() {
            Some(TokenTree::Ident(id)) if id == "in" => {
                p.i += 1;
                break;
            }
            Some(tt) => {
                pat.push(tt.clone());
                p.i += 1;
            }
            None => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "expected `in` after `for` pattern",
                ))
            }
        }
    }
    // Iterand: tokens up to the body brace group
    let mut expr = Vec::new();
    let body_group = loop {
        match p.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => break g.clone(),
            Some(tt) => {
                expr.push(tt.clone());
                p.i += 1;
            }
            None => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "expected `{ ... }` body after `for` iterand",
                ))
            }
        }
    };
    p.i += 1;
    let pat: TS2 = pat.into_iter().collect();
    let expr: TS2 = expr.into_iter().collect();
    let body_nodes = parse_nodes(
        &mut P {
            t: body_group.stream().into_iter().collect(),
            i: 0,
        },
        Stop::Eof,
    )?;
    let body_expr = nodes_expr(&body_nodes);
    Ok(quote! {
        ::ferrum_core::VNode::fragment(
            (#expr).into_iter().map(|#pat| #body_expr).collect::<Vec<::ferrum_core::VNode>>()
        )
    })
}

// --- text -------------------------------------------------------------------

fn text_node(lit: &proc_macro2::Literal) -> syn::Result<TS2> {
    let s = lit.to_string();
    if let Ok(litstr) = syn::parse_str::<syn::LitStr>(&s) {
        let ts = interpolate_fmt(&litstr)?;
        Ok(quote! { ::ferrum_core::VNode::text(#ts) })
    } else {
        // Non-string literal (number, bool, ...): stringify it.
        Ok(quote! { ::ferrum_core::VNode::text(#lit.to_string()) })
    }
}

/// `"Hello, {name} {{ escaped }}"` -> `format!("Hello, {} {{ escaped }}", name)`
/// (or the plain literal when there is nothing to interpolate).
fn interpolate_fmt(lit: &syn::LitStr) -> syn::Result<TS2> {
    let s = lit.value();
    let mut fmt = String::new();
    let mut args: Vec<TS2> = Vec::new();
    let mut had_brace = false;
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' => {
                if it.peek() == Some(&'{') {
                    it.next();
                    fmt.push_str("{{");
                    had_brace = true;
                } else {
                    let mut inner = String::new();
                    let mut closed = false;
                    for c2 in it.by_ref() {
                        if c2 == '}' {
                            closed = true;
                            break;
                        }
                        inner.push(c2);
                    }
                    if !closed {
                        return Err(syn::Error::new(
                            lit.span(),
                            "unclosed `{` in text interpolation",
                        ));
                    }
                    if inner.trim().is_empty() {
                        return Err(syn::Error::new(lit.span(), "empty `{}` interpolation"));
                    }
                    let ts: TS2 = inner.trim().parse().map_err(|_| {
                        syn::Error::new(lit.span(), format!("invalid expression `{}`", inner))
                    })?;
                    args.push(ts);
                    fmt.push_str("{}");
                    had_brace = true;
                }
            }
            '}' => {
                if it.peek() == Some(&'}') {
                    it.next();
                    fmt.push_str("}}");
                } else {
                    fmt.push_str("}}");
                }
                had_brace = true;
            }
            _ => fmt.push(c),
        }
    }
    if !had_brace {
        return Ok(quote! { #lit });
    }
    Ok(quote! { format!(#fmt, #(#args),*) })
}
