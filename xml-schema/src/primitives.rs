use std::cmp::max;
use std::str::FromStr;
use std::marker::PhantomData;
use std::fmt;

use bigdecimal::BigDecimal;
use num_traits::{Zero, One};

use xmlparser::{Token as XmlToken, ElementEnd, StrSpan};

use support::{ParseXml, ParseXmlStr, Stream, ParseContext, ParentContext, Facets, BigFloatNotNaN};
use xml_utils::*;

macro_rules! return_split {
    ( $input:expr, $position:expr, $pred:expr, $validator:ident !, $facets:expr) => {{
        let input = $input;
        let pos = $position;
        let parsed = &input[0..pos];
        $validator!(parsed, $facets);
        return Some((&input[pos..], $pred(parsed)))
    }}
}

macro_rules! validate_str {
    ( $s:expr, $facets:expr) => {{
        let facets = $facets;
        let s: &&str = &$s;
        if let Some(ref enumeration) = facets.enumeration {
            if !enumeration.contains(s) {
                panic!("Expected one of {:?}, got {:?}", enumeration, s);
            }
        }
        if let Some(ref length) = facets.length {
            if s.len() != *length {
                panic!("{:?} has length != {}", s, length);
            }
        }
        if let Some(ref min_length) = facets.min_length {
            if s.len() < *min_length {
                panic!("{:?} has length < {}", s, min_length);
            }
        }
        if let Some(ref max_length) = facets.max_length {
            if s.len() > *max_length {
                panic!("{:?} has length > {}", s, max_length);
            }
        }
    }}
}

macro_rules! validate_int {
    ( $n:expr, $facets:expr) => {{
        let n: BigDecimal = $n.into();
        validate_decimal!(n, $facets);
    }}
}
macro_rules! validate_decimal {
    ( $n:expr, $facets:expr) => {{
        let facets = $facets;
        let n: BigFloatNotNaN = $n.into();
        if let Some(ref min_exclusive) = facets.min_exclusive {
            if n <= *min_exclusive {
                panic!("{} is <= {}", n, min_exclusive);
            }
        }
        if let Some(ref min_inclusive) = facets.min_inclusive {
            if n < *min_inclusive {
                panic!("{} is < {}", n, min_inclusive);
            }
        }
        if let Some(ref max_exclusive) = facets.max_exclusive {
            if n >= *max_exclusive {
                panic!("{} is >= {}", n, max_exclusive);
            }
        }
        if let Some(ref max_inclusive) = facets.max_inclusive {
            if n > *max_inclusive {
                panic!("{} is > {}", n, max_inclusive);
            }
        }
    }}
}

pub const PRIMITIVE_TYPES: &[(&'static str, &'static str)] = &[
    ("anySimpleType", "AnySimpleType"),
    ("token", "Token"),
    ("QName", "QName"),
    ("string", "XmlString"),
    ("positiveInteger", "PositiveInteger"),
    ("anyURI", "AnyUri"),
    ("boolean", "Boolean"),
    ("NCName", "NcName"),
    ("nonNegativeInteger", "NonNegativeInteger"),
    ("dateTime", "DateTime"),
    ("date", "Date"),
    ("duration", "Duration"),
    ("decimal", "Decimal"),
    ];

pub type DateTime<'input> = Token<'input>; // TODO
pub type Date<'input> = Token<'input>; // TODO
pub type Duration<'input> = Token<'input>; // TODO

/// https://www.w3.org/TR/xmlschema11-2/#token
#[derive(Debug, PartialEq)]
pub struct Token<'input>(pub &'input str);

impl<'input> ParseXmlStr<'input> for Token<'input> {
    const NODE_NAME: &'static str = "token";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, Token<'input>)> {
        if input.len() == 0 {
            return None;
        }
        let mut iter = input.char_indices().peekable();
        while let Some((i, c)) = iter.next() {
            match (i, c) {
                (0, ' ') => return None,
                (_, ' ') => {
                    // If this space is followed by a whitespace, split before both
                    match iter.peek() {
                        Some((_, ' ')) | Some((_, '\r')) | Some((_, '\n')) |
                        Some((_, '\t')) => return_split!(input, i, Token, validate_str!, facets),
                        Some((_, _)) => (),
                        None => return_split!(input, i, Token, validate_str!, facets),
                    }
                }
                (_, '\r') | (_, '\n') | (_, '\t') => return_split!(input, i, Token, validate_str!, facets),
                _ => (),
            }
        }
        validate_str!(input, facets);
        Some(("", Token(input)))
    }
}
impl<'input> Default for Token<'input> {
    fn default() -> Self {
        Token("")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct QName<'input> {
    pub namespace: Option<&'input str>,
    pub local_name: &'input str,
}
impl<'input> ParseXmlStr<'input> for QName<'input> {
    const NODE_NAME: &'static str = "QName";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, QName<'input>)> {
        if input.len() == 0 {
            return None;
        }
        let f = &mut |prefix, local| QName {
            namespace: parent_context.namespaces.get(prefix).cloned(),
            local_name: local
        };
        let mut i1 = 0;
        for (i, c) in input.char_indices() {
            if c == ':' {
                i1 = i;
            }
            else if c == ' ' { // TODO
                if i == 0 || i <= i1+1 {
                    return None;
                }
                if i1 > 0 {
                    return Some((&input[i..], f(&input[0..i1+1], &input[i1+1..i+1])))
                }
                else {
                    return Some((&input[i..], f("", &input[0..i+1])))
                }
            }
        }
        if i1 > 0 {
            return Some(("", f(&input[0..i1], &input[i1+1..])))
        }
        else {
            return Some(("", f("", input)))
        }
    }
}

impl<'input> From<&'input str> for QName<'input> {
    fn from(s: &'input str) -> QName<'input> {
        let mut splitted = s.split(":");
        let v1 = splitted.next().expect(&format!("Empty qname"));
        let v2 = splitted.next();
        assert_eq!(splitted.next(), None);
        match v2 {
            None => QName { namespace: None, local_name: v1 },
            Some(v2) => QName { namespace: Some(v1), local_name: v2 },
        }
    }
}

impl <'input> QName<'input> {
    pub fn from_strspans(prefix: StrSpan<'input>, local: StrSpan<'input>) -> QName<'input> {
        match prefix.to_str() {
            "" => QName { namespace: None, local_name: local.to_str() },
            p => QName { namespace: Some(p), local_name: local.to_str() },
        }
    }
}

impl<'input> fmt::Display for QName<'input> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.namespace {
            Some(ns) => write!(f, "{}:{}", ns, self.local_name),
            None => write!(f, "{}", self.local_name),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct AnyUri<'input>(pub &'input str);
impl<'input> ParseXmlStr<'input> for AnyUri<'input> {
    const NODE_NAME: &'static str = "AnyUri";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, AnyUri<'input>)> {
        if input.len() == 0 {
            return None;
        }
        for (i, c) in input.char_indices() {
            if c == ' ' { // TODO
                if i == 0 {
                    return None;
                }
                return Some((&input[i..], AnyUri(&input[0..i])))
            }
        }
        Some(("", AnyUri(input)))
    }
}

#[derive(Debug, PartialEq)]
pub struct AnyURIElement<'input>(StrSpan<'input>);
impl<'input> ParseXml<'input> for AnyURIElement<'input> {
    const NODE_NAME: &'static str = "AnyURIElement";
    fn parse_self_xml<TParseContext: ParseContext<'input>>(stream: &mut Stream<'input>, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>) -> Option<AnyURIElement<'input>> {
        match stream.next() {
            Some(XmlToken::Text(strspan)) => Some(AnyURIElement(strspan)),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Integer<'input>(pub i64, PhantomData<&'input ()>);
impl<'input> ParseXmlStr<'input> for Integer<'input> {
    const NODE_NAME: &'static str = "Integer";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, Integer<'input>)> {
        let mut iter = input.char_indices();
        let mut n: i64 = 0;
        let mut multiplier = 1;
        let c = iter.next()?.1;
        match c {
            '+' => multiplier = 1,
            '-' => multiplier = -1,
            '0'..='9' => n = (c as i64) - ('0' as i64),
            _ => return None,
        }

        if c == '+' || c == '-' {
            let c = iter.next()?.1;
            match c {
                '0'..='9' => n = (c as i64) - ('0' as i64),
                _ => return None,
            }
        }

        for (i,c) in iter {
            match c {
                '0'..='9' => n = n * 10 + ((c as i64) - ('0' as i64)),
                _ => {
                    let res = multiplier * n;
                    validate_int!(res, facets);
                    return Some((&input[i..], Integer(res, PhantomData::default())));
                }
            }
        }
        
        let res = multiplier * n;
        validate_int!(res, facets);
        Some(("", Integer(res, PhantomData::default())))
    }
}

#[derive(Debug, PartialEq)]
pub struct NonNegativeInteger<'input>(pub u64, PhantomData<&'input ()>);
impl<'input> ParseXmlStr<'input> for NonNegativeInteger<'input> {
    const NODE_NAME: &'static str = "NonNegativeInteger";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, parse_context: &mut TParseContext, parent_context: &ParentContext<'input>, facets: &Facets) -> Option<(&'input str, NonNegativeInteger<'input>)> {
        let min = max(BigFloatNotNaN::zero(), facets.min_inclusive.clone().unwrap_or(BigFloatNotNaN::zero()));
        let mut facets = facets.clone();
        facets.min_inclusive = Some(min);
        let (output, n) = Integer::parse_self_xml_str(input, parse_context, parent_context, &facets)?;
        Some((output, NonNegativeInteger(n.0 as u64, PhantomData::default())))
    }
}

#[derive(Debug, PartialEq)]
pub struct PositiveInteger<'input>(pub u64, PhantomData<&'input ()>);
impl<'input> ParseXmlStr<'input> for PositiveInteger<'input> {
    const NODE_NAME: &'static str = "PositiveInteger";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, parse_context: &mut TParseContext, parent_context: &ParentContext<'input>, facets: &Facets) -> Option<(&'input str, PositiveInteger<'input>)> {
        let min = max(BigFloatNotNaN::one(), facets.min_inclusive.clone().unwrap_or(BigFloatNotNaN::zero()));
        let mut facets = facets.clone();
        facets.min_inclusive = Some(min);
        let (output, n) = NonNegativeInteger::parse_self_xml_str(input, parse_context, parent_context, &facets)?;
        Some((output, PositiveInteger(n.0, PhantomData::default())))
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Decimal<'input>(pub BigDecimal, PhantomData<&'input ()>);
impl<'input> ParseXmlStr<'input> for Decimal<'input> {
    const NODE_NAME: &'static str = "Decimal";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, Decimal<'input>)> {
        for (i, c) in input.char_indices() {
            if c == ' ' { // TODO
                let res = match BigDecimal::from_str(&input[0..i]) {
                    Ok(res) => res,
                    Err(_) => return None,
                };
                validate_decimal!(res.clone(), facets);
                return Some((&input[i..], Decimal(res, PhantomData::default())))
            }
        }
        let res = match BigDecimal::from_str(input) {
            Ok(res) => res,
            Err(_) => return None,
        };
        validate_decimal!(res.clone(), facets);
        Some(("", Decimal(res, PhantomData::default())))
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Any<'input>(pub Vec<XmlToken<'input>>);
impl<'input> ParseXml<'input> for Any<'input> {
    const NODE_NAME: &'static str = "Any";
    fn parse_self_xml<TParseContext: ParseContext<'input>>(stream: &mut Stream<'input>, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>) -> Option<Any<'input>> {
        let mut tag_stack = Vec::new();
        let mut tokens = Vec::new();
        loop {
            let tx = stream.transaction();
            let tok = stream.next()?;
            match tok {
                XmlToken::Whitespaces(_) => (),
                XmlToken::Comment(_) => (),
                XmlToken::Text(_) => (),
                XmlToken::ElementStart(prefix, name) => {
                    tag_stack.push(QName::from_strspans(prefix, name));
                    tokens.push(tok);
                    break
                },
                _ => {
                    tx.rollback(stream);
                    if tokens.len() > 0 {
                        return Some(Any(tokens));
                    }
                    else {
                        return None;
                    }
                }
            }
            tokens.push(tok);
        }
        while tag_stack.len() > 0 {
            let tok = stream.next().unwrap();
            tokens.push(tok);
            match tok {
                XmlToken::ElementStart(prefix, name) => tag_stack.push(QName::from_strspans(prefix, name)),
                XmlToken::ElementEnd(end) => {
                    match end {
                        ElementEnd::Open => (),
                        ElementEnd::Close(prefix, name) => assert_eq!(QName::from_strspans(prefix, name), tag_stack.pop().unwrap()),
                        ElementEnd::Empty => { tag_stack.pop(); () },
                    }
                }
                _ => (),
            }
        }
        Some(Any(tokens))
    }
}

/// https://www.w3.org/TR/xmlschema11-2/#string
#[derive(Debug, PartialEq)]
pub struct XmlString<'input>(pub &'input str);

impl<'input> ParseXmlStr<'input> for XmlString<'input> {
    const NODE_NAME: &'static str = "XmlString";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, XmlString<'input>)> {
        for (i, c) in input.char_indices() {
            if !is_xml_char(c) {
                return_split!(input, i, XmlString, validate_str!, facets);
            }
        }
        Some(("", XmlString(input)))
    }
}

impl<'input> Default for XmlString<'input> {
    fn default() -> Self {
        XmlString("")
    }
}

/// https://www.w3.org/TR/xmlschema11-2/#anySimpleType
#[derive(Debug, PartialEq)]
pub struct AnySimpleType<'input>(pub &'input str);

impl<'input> ParseXmlStr<'input> for AnySimpleType<'input> {
    const NODE_NAME: &'static str = "AnySimpleType";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, AnySimpleType<'input>)> {
        Some(("", AnySimpleType(input)))
    }
}

impl<'input> Default for AnySimpleType<'input> {
    fn default() -> Self {
        AnySimpleType("")
    }
}


/// https://www.w3.org/TR/xmlschema11-2/#NCName
#[derive(Debug, PartialEq)]
pub struct NcName<'input>(pub &'input str);

impl<'input> ParseXmlStr<'input> for NcName<'input> {
    const NODE_NAME: &'static str = "NcName";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, NcName<'input>)> {
        let mut iter = input.char_indices();
        let c = iter.next()?.1;
        if c == ':' || !is_name_start_char(c) { return None };

        for (i, c) in iter {
            if c == ':' || !is_name_char(c) {
                return_split!(input, i, NcName, validate_str!, facets);
            }
        }

        Some(("", NcName(input)))
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Boolean<'input>(bool, PhantomData<&'input ()>);
impl<'input> ParseXmlStr<'input> for Boolean<'input> {
    const NODE_NAME: &'static str = "Boolean";
    fn parse_self_xml_str<'a, TParseContext: ParseContext<'input>>(input: &'input str, _parse_context: &mut TParseContext, _parent_context: &ParentContext<'input>, facets: &Facets<'a>) -> Option<(&'input str, Boolean<'input>)> {
        if input.len() >= 1 {
            match &input[0..1] {
                "0" => return Some((&input[1..], Boolean(false, PhantomData::default()))),
                "1" => return Some((&input[1..], Boolean(true, PhantomData::default()))),
                _ => (),
            }
        }
        if input.len() >= 4 && &input[0..4] == "true" {
            return Some((&input[4..], Boolean(true, PhantomData::default())))
        }
        if input.len() >= 5 && &input[0..4] == "false" {
            return Some((&input[5..], Boolean(false, PhantomData::default())))
        }
        None
    }
}
