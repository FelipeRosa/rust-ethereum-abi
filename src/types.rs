use serde::{de::Visitor, Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Uint(usize),
    Int(usize),
    Address,
    Bool,
    String,
    FixedBytes(usize),
    Bytes,
    FixedArray(Box<Type>, usize),
    Array(Box<Type>),
    Tuple(Vec<Type>),
}

impl<'de> Deserialize<'de> for Type {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TypeVisitor)
    }
}

struct TypeVisitor;

impl<'de> Visitor<'de> for TypeVisitor {
    type Value = Type;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an ABI type")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // The parser does not ignore whitespaces yet so we remove them before
        // parsing the given string.
        let (_, ty) = parsers::parse_exact_type(&v.to_string().replace(" ", ""))
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;

        Ok(ty)
    }
}

mod parsers {
    use nom::{
        branch::alt,
        bytes::complete::tag,
        character::complete::{char, digit1},
        combinator::{map_res, opt, recognize, verify},
        exact,
        multi::{many1, separated_list1},
        sequence::delimited,
        IResult,
    };

    use super::Type;

    pub fn parse_exact_type(input: &str) -> IResult<&str, Type> {
        exact!(input, parse_type)
    }

    fn parse_type(input: &str) -> IResult<&str, Type> {
        alt((parse_tuple, parse_array, parse_simple_type))(input)
    }

    fn parse_simple_type(input: &str) -> IResult<&str, Type> {
        alt((
            parse_uint,
            parse_int,
            parse_bytes,
            parse_string,
            parse_address,
            parse_bool,
        ))(input)
    }

    fn parse_uint(input: &str) -> IResult<&str, Type> {
        verify(parse_sized("uint"), check_int_size)(input).map(|(i, size)| (i, Type::Uint(size)))
    }

    fn parse_int(input: &str) -> IResult<&str, Type> {
        verify(parse_sized("int"), check_int_size)(input).map(|(i, size)| (i, Type::Int(size)))
    }

    fn parse_address(input: &str) -> IResult<&str, Type> {
        tag("address")(input).map(|(i, _)| (i, Type::Address))
    }

    fn parse_bool(input: &str) -> IResult<&str, Type> {
        tag("bool")(input).map(|(i, _)| (i, Type::Bool))
    }

    fn parse_string(input: &str) -> IResult<&str, Type> {
        tag("string")(input).map(|(i, _)| (i, Type::String))
    }

    fn parse_bytes(input: &str) -> IResult<&str, Type> {
        let (i, _) = tag("bytes")(input)?;
        let (i, size) = opt(verify(parse_integer, check_fixed_bytes_size))(i)?;

        let ty = size.map_or(Type::Bytes, Type::FixedBytes);

        Ok((i, ty))
    }

    fn parse_array(input: &str) -> IResult<&str, Type> {
        let (i, ty) = parse_simple_type(input)?;

        let (i, sizes) = many1(delimited(char('['), opt(parse_integer), char(']')))(i)?;

        let array_from_size = |ty: Type, size: Option<usize>| match size {
            None => Type::Array(Box::new(ty)),
            Some(size) => Type::FixedArray(Box::new(ty), size),
        };

        let init_arr_ty = array_from_size(ty, sizes[0]);
        let arr_ty = sizes.into_iter().skip(1).fold(init_arr_ty, array_from_size);

        Ok((i, arr_ty))
    }

    fn parse_tuple(input: &str) -> IResult<&str, Type> {
        delimited(char('('), separated_list1(char(','), parse_type), char(')'))(input)
            .map(|(i, tys)| (i, Type::Tuple(tys)))
    }

    fn parse_sized<'a>(t: &'a str) -> impl Fn(&'a str) -> IResult<&'a str, usize> {
        move |input: &str| {
            let (i, _) = tag(t)(input)?;

            parse_integer(i)
        }
    }

    fn parse_integer(input: &str) -> IResult<&str, usize> {
        map_res(recognize(many1(digit1)), str::parse)(input)
    }

    fn check_int_size(i: &usize) -> bool {
        let i = *i;

        i > 0 && i <= 256 && i % 8 == 0
    }

    fn check_fixed_bytes_size(i: &usize) -> bool {
        let i = *i;

        i > 0 && i <= 32
    }

    #[cfg(test)]
    mod test {
        use super::super::*;
        use super::*;

        #[test]
        fn parse_uint() {
            for i in (8..=256).step_by(8) {
                let s = format!("uint{}", i);

                assert_eq!(parse_exact_type(&s), Ok(("", Type::Uint(i))));
            }
        }

        #[test]
        fn parse_int() {
            for i in (8..=256).step_by(8) {
                let s = format!("int{}", i);

                assert_eq!(parse_exact_type(&s), Ok(("", Type::Int(i))));
            }
        }

        #[test]
        fn parse_address() {
            assert_eq!(parse_exact_type("address"), Ok(("", Type::Address)));
        }

        #[test]
        fn parse_bool() {
            assert_eq!(parse_exact_type("bool"), Ok(("", Type::Bool)));
        }

        #[test]
        fn parse_string() {
            assert_eq!(parse_exact_type("string"), Ok(("", Type::String)));
        }

        #[test]
        fn parse_bytes() {
            assert_eq!(parse_exact_type("bytes"), Ok(("", Type::Bytes)));

            for i in 1..=32 {
                let s = format!("bytes{}", i);

                assert_eq!(parse_exact_type(&s), Ok(("", Type::FixedBytes(i))));
            }
        }

        #[test]
        fn parse_array() {
            assert_eq!(
                parse_exact_type("uint256[]"),
                Ok(("", Type::Array(Box::new(Type::Uint(256)))))
            );

            // Nested arrays
            assert_eq!(
                parse_exact_type("address[][]"),
                Ok((
                    "",
                    Type::Array(Box::new(Type::Array(Box::new(Type::Address))))
                ))
            );

            // Mixed arrays
            assert_eq!(
                parse_exact_type("string[2][]"),
                Ok((
                    "",
                    Type::Array(Box::new(Type::FixedArray(Box::new(Type::String), 2)))
                ))
            );
            assert_eq!(
                parse_exact_type("string[][3]"),
                Ok((
                    "",
                    Type::FixedArray(Box::new(Type::Array(Box::new(Type::String))), 3)
                ))
            );
        }

        #[test]
        fn parse_tuple() {
            assert_eq!(
                parse_exact_type("(uint256,string,address[])"),
                Ok((
                    "",
                    Type::Tuple(vec![
                        Type::Uint(256),
                        Type::String,
                        Type::Array(Box::new(Type::Address))
                    ])
                ))
            );
        }
    }
}
